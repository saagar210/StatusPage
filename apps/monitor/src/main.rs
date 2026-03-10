mod checker;
mod config;
mod db;
mod evaluator;
mod redis_publisher;
mod rollup;
mod scheduler;

use std::path::PathBuf;
use std::time::Duration;

use sqlx::postgres::PgPoolOptions;
use tracing_subscriber::EnvFilter;

use crate::config::Config;
use crate::redis_publisher::RedisPublisher;
use crate::scheduler::Scheduler;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();

    let config = Config::from_env()?;

    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(&config.log_level)),
        )
        .json()
        .init();

    tracing::info!("Monitor engine starting...");

    let pool = PgPoolOptions::new()
        .max_connections(10)
        .min_connections(2)
        .connect(&config.database_url)
        .await?;

    if config.run_migrations_on_start {
        tracing::info!("Running migrations...");
        sqlx::migrate!("../../migrations").run(&pool).await?;
    } else {
        tracing::info!("Skipping automatic migrations on startup");
    }

    if let Some(path) = config.healthcheck_file.clone() {
        tokio::spawn(async move {
            write_healthcheck_heartbeat(path).await;
        });
    }

    let publisher = match redis::Client::open(config.redis_url.clone()) {
        Ok(redis_client) => match redis::aio::ConnectionManager::new(redis_client).await {
            Ok(redis) => {
                tracing::info!("Monitor Redis connection established");
                Some(RedisPublisher::new(redis))
            }
            Err(error) => {
                tracing::warn!(
                    error = %error,
                    "Monitor failed to connect to Redis; realtime publishing disabled"
                );
                None
            }
        },
        Err(error) => {
            tracing::warn!(
                error = %error,
                "Monitor received invalid Redis URL; realtime publishing disabled"
            );
            None
        }
    };

    let scheduler = Scheduler::new(pool, config, publisher);
    scheduler.run().await?;

    Ok(())
}

async fn write_healthcheck_heartbeat(path: String) {
    let path = PathBuf::from(path);

    if let Some(parent) = path.parent() {
        if let Err(error) = tokio::fs::create_dir_all(parent).await {
            tracing::warn!(error = %error, "Failed to create monitor healthcheck directory");
            return;
        }
    }

    loop {
        if let Err(error) = tokio::fs::write(&path, chrono::Utc::now().to_rfc3339()).await {
            tracing::warn!(error = %error, "Failed to update monitor healthcheck heartbeat");
        }

        tokio::time::sleep(Duration::from_secs(15)).await;
    }
}
