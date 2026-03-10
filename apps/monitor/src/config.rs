use anyhow::{Context, Result};

#[derive(Debug, Clone)]
pub struct Config {
    pub database_url: String,
    pub redis_url: String,
    pub max_concurrent_checks: usize,
    pub config_reload_interval_secs: u64,
    pub run_migrations_on_start: bool,
    pub healthcheck_file: Option<String>,
    pub log_level: String,
}

impl Config {
    pub fn from_env() -> Result<Self> {
        Ok(Self {
            database_url: std::env::var("DATABASE_URL").context("DATABASE_URL must be set")?,
            redis_url: std::env::var("REDIS_URL")
                .unwrap_or_else(|_| "redis://localhost:6379".to_string()),
            max_concurrent_checks: std::env::var("MAX_CONCURRENT_CHECKS")
                .unwrap_or_else(|_| "50".to_string())
                .parse()
                .context("MAX_CONCURRENT_CHECKS must be a number")?,
            config_reload_interval_secs: std::env::var("CONFIG_RELOAD_INTERVAL_SECS")
                .unwrap_or_else(|_| "60".to_string())
                .parse()
                .context("CONFIG_RELOAD_INTERVAL_SECS must be a number")?,
            run_migrations_on_start: std::env::var("RUN_MIGRATIONS_ON_START")
                .unwrap_or_else(|_| "true".to_string())
                .parse()
                .context("RUN_MIGRATIONS_ON_START must be true or false")?,
            healthcheck_file: std::env::var("HEALTHCHECK_FILE")
                .ok()
                .filter(|value| !value.trim().is_empty()),
            log_level: std::env::var("LOG_LEVEL").unwrap_or_else(|_| "info".to_string()),
        })
    }
}
