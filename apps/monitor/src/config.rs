use anyhow::{Context, Result};

#[derive(Debug, Clone)]
pub struct Config {
    pub database_url: String,
    pub max_concurrent_checks: usize,
    pub config_reload_interval_secs: u64,
    pub log_level: String,
}

impl Config {
    pub fn from_env() -> Result<Self> {
        Ok(Self {
            database_url: std::env::var("DATABASE_URL")
                .context("DATABASE_URL must be set")?,
            max_concurrent_checks: std::env::var("MAX_CONCURRENT_CHECKS")
                .unwrap_or_else(|_| "50".to_string())
                .parse()
                .context("MAX_CONCURRENT_CHECKS must be a number")?,
            config_reload_interval_secs: std::env::var("CONFIG_RELOAD_INTERVAL_SECS")
                .unwrap_or_else(|_| "60".to_string())
                .parse()
                .context("CONFIG_RELOAD_INTERVAL_SECS must be a number")?,
            log_level: std::env::var("LOG_LEVEL")
                .unwrap_or_else(|_| "info".to_string()),
        })
    }
}
