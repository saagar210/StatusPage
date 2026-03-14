use anyhow::{Context, Result};

#[derive(Debug, Clone)]
pub struct Config {
    pub database_url: String,
    pub redis_url: String,
    pub webhook_dispatch_interval_secs: u64,
    pub webhook_dispatch_batch_size: i64,
    pub webhook_timeout_secs: u64,
    pub smtp_host: Option<String>,
    pub smtp_port: u16,
    pub smtp_username: Option<String>,
    pub smtp_password: Option<String>,
    pub smtp_secure: bool,
    pub email_from: String,
    pub app_base_url: String,
    pub email_dispatch_interval_secs: u64,
    pub email_dispatch_batch_size: i64,
    pub stripe_secret_key: Option<String>,
    pub stripe_webhook_secret: Option<String>,
    pub stripe_price_pro: Option<String>,
    pub stripe_price_team: Option<String>,
    pub internal_admin_token: Option<String>,
    pub downgrade_enforcement_interval_secs: u64,
    pub api_port: u16,
    pub api_host: String,
    pub cors_origin: String,
    pub statuspage_host: Option<String>,
    pub run_migrations_on_start: bool,
    pub run_migrations_only: bool,
    pub log_level: String,
}

impl Config {
    pub fn from_env() -> Result<Self> {
        Ok(Self {
            database_url: std::env::var("DATABASE_URL").context("DATABASE_URL must be set")?,
            redis_url: std::env::var("REDIS_URL")
                .unwrap_or_else(|_| "redis://localhost:6379".to_string()),
            webhook_dispatch_interval_secs: std::env::var("WEBHOOK_DISPATCH_INTERVAL_SECS")
                .unwrap_or_else(|_| "3".to_string())
                .parse()
                .context("WEBHOOK_DISPATCH_INTERVAL_SECS must be a number")?,
            webhook_dispatch_batch_size: std::env::var("WEBHOOK_DISPATCH_BATCH_SIZE")
                .unwrap_or_else(|_| "10".to_string())
                .parse()
                .context("WEBHOOK_DISPATCH_BATCH_SIZE must be a number")?,
            webhook_timeout_secs: std::env::var("WEBHOOK_TIMEOUT_SECS")
                .unwrap_or_else(|_| "10".to_string())
                .parse()
                .context("WEBHOOK_TIMEOUT_SECS must be a number")?,
            smtp_host: std::env::var("SMTP_HOST")
                .ok()
                .filter(|value| !value.trim().is_empty()),
            smtp_port: std::env::var("SMTP_PORT")
                .unwrap_or_else(|_| "1025".to_string())
                .parse()
                .context("SMTP_PORT must be a valid port number")?,
            smtp_username: std::env::var("SMTP_USERNAME")
                .ok()
                .filter(|value| !value.trim().is_empty()),
            smtp_password: std::env::var("SMTP_PASSWORD")
                .ok()
                .filter(|value| !value.trim().is_empty()),
            smtp_secure: std::env::var("SMTP_SECURE")
                .unwrap_or_else(|_| "false".to_string())
                .parse()
                .context("SMTP_SECURE must be true or false")?,
            email_from: std::env::var("EMAIL_FROM")
                .unwrap_or_else(|_| "alerts@statuspage.local".to_string()),
            app_base_url: std::env::var("APP_BASE_URL")
                .unwrap_or_else(|_| "http://localhost:3000".to_string()),
            email_dispatch_interval_secs: std::env::var("EMAIL_DISPATCH_INTERVAL_SECS")
                .unwrap_or_else(|_| "3".to_string())
                .parse()
                .context("EMAIL_DISPATCH_INTERVAL_SECS must be a number")?,
            email_dispatch_batch_size: std::env::var("EMAIL_DISPATCH_BATCH_SIZE")
                .unwrap_or_else(|_| "20".to_string())
                .parse()
                .context("EMAIL_DISPATCH_BATCH_SIZE must be a number")?,
            stripe_secret_key: std::env::var("STRIPE_SECRET_KEY")
                .ok()
                .filter(|value| !value.trim().is_empty()),
            stripe_webhook_secret: std::env::var("STRIPE_WEBHOOK_SECRET")
                .ok()
                .filter(|value| !value.trim().is_empty()),
            stripe_price_pro: std::env::var("STRIPE_PRICE_PRO")
                .ok()
                .filter(|value| !value.trim().is_empty()),
            stripe_price_team: std::env::var("STRIPE_PRICE_TEAM")
                .ok()
                .filter(|value| !value.trim().is_empty()),
            internal_admin_token: std::env::var("INTERNAL_ADMIN_TOKEN")
                .ok()
                .filter(|value| !value.trim().is_empty()),
            downgrade_enforcement_interval_secs: std::env::var(
                "DOWNGRADE_ENFORCEMENT_INTERVAL_SECS",
            )
            .unwrap_or_else(|_| "60".to_string())
            .parse()
            .context("DOWNGRADE_ENFORCEMENT_INTERVAL_SECS must be a number")?,
            api_port: std::env::var("API_PORT")
                .unwrap_or_else(|_| "4000".to_string())
                .parse()
                .context("API_PORT must be a valid port number")?,
            api_host: std::env::var("API_HOST").unwrap_or_else(|_| "0.0.0.0".to_string()),
            cors_origin: std::env::var("CORS_ORIGIN")
                .unwrap_or_else(|_| "http://localhost:3000".to_string()),
            statuspage_host: std::env::var("STATUSPAGE_HOST")
                .ok()
                .filter(|value| !value.trim().is_empty()),
            run_migrations_on_start: std::env::var("RUN_MIGRATIONS_ON_START")
                .unwrap_or_else(|_| "true".to_string())
                .parse()
                .context("RUN_MIGRATIONS_ON_START must be true or false")?,
            run_migrations_only: std::env::var("RUN_MIGRATIONS_ONLY")
                .unwrap_or_else(|_| "false".to_string())
                .parse()
                .context("RUN_MIGRATIONS_ONLY must be true or false")?,
            log_level: std::env::var("LOG_LEVEL").unwrap_or_else(|_| "info".to_string()),
        })
    }
}
