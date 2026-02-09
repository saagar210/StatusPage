use anyhow::{Context, Result};

#[derive(Debug, Clone)]
pub struct Config {
    pub database_url: String,
    pub api_port: u16,
    pub api_host: String,
    pub cors_origin: String,
    pub log_level: String,
}

impl Config {
    pub fn from_env() -> Result<Self> {
        Ok(Self {
            database_url: std::env::var("DATABASE_URL")
                .context("DATABASE_URL must be set")?,
            api_port: std::env::var("API_PORT")
                .unwrap_or_else(|_| "4000".to_string())
                .parse()
                .context("API_PORT must be a valid port number")?,
            api_host: std::env::var("API_HOST")
                .unwrap_or_else(|_| "0.0.0.0".to_string()),
            cors_origin: std::env::var("CORS_ORIGIN")
                .unwrap_or_else(|_| "http://localhost:3000".to_string()),
            log_level: std::env::var("LOG_LEVEL")
                .unwrap_or_else(|_| "info".to_string()),
        })
    }
}
