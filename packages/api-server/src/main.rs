mod config;
mod db;
mod middleware;
mod routes;
mod services;
mod state;

use std::time::Duration;

use axum::http::{HeaderValue, Method};
use sqlx::postgres::PgPoolOptions;
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;
use tracing_subscriber::EnvFilter;

use crate::config::Config;
use crate::middleware::request_id::RequestIdLayer;
use crate::routes::api_router;
use crate::state::AppState;

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

    tracing::info!("Connecting to database...");
    let pool = PgPoolOptions::new()
        .max_connections(20)
        .min_connections(5)
        .acquire_timeout(Duration::from_secs(3))
        .connect(&config.database_url)
        .await?;

    tracing::info!("Running migrations...");
    sqlx::migrate!("../../migrations").run(&pool).await?;

    tracing::info!("Connecting to Redis...");
    let redis_client = redis::Client::open(config.redis_url.clone())?;
    let redis = redis::aio::ConnectionManager::new(redis_client).await?;
    tracing::info!("Redis connection established");

    let cors = CorsLayer::new()
        .allow_origin(
            config
                .cors_origin
                .parse::<HeaderValue>()
                .expect("Invalid CORS origin"),
        )
        .allow_methods([
            Method::GET,
            Method::POST,
            Method::PATCH,
            Method::DELETE,
            Method::OPTIONS,
        ])
        .allow_headers(
            axum::http::header::HeaderMap::new()
                .keys()
                .cloned()
                .collect::<Vec<_>>(),
        )
        .allow_credentials(true);

    let publisher = services::redis_publisher::RedisPublisher::new(redis.clone());

    let state = AppState {
        pool,
        redis,
        publisher,
        config: config.clone(),
    };

    let app = api_router(state)
        .layer(TraceLayer::new_for_http())
        .layer(cors)
        .layer(RequestIdLayer);

    let addr = format!("{}:{}", config.api_host, config.api_port);
    tracing::info!("Listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(&addr).await?;

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    tracing::info!("Server stopped");
    Ok(())
}

async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("Failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("Failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }

    tracing::info!("Shutdown signal received");
}
