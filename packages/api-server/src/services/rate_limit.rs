use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

use redis::{aio::ConnectionManager, Script};
use shared::error::AppError;

static RATE_LIMITS: OnceLock<Mutex<HashMap<String, Vec<Instant>>>> = OnceLock::new();

const RATE_LIMIT_SCRIPT: &str = r#"
local current = redis.call('INCR', KEYS[1])
if current == 1 then
  redis.call('EXPIRE', KEYS[1], ARGV[1])
end
return current
"#;

pub async fn enforce_rate_limit(
    redis: &ConnectionManager,
    scope: &str,
    subject: &str,
    limit: usize,
    window: Duration,
) -> Result<(), AppError> {
    let key = redis_rate_limit_key(scope, subject);

    match increment_redis_attempt(redis, &key, window).await {
        Ok(attempt_count) if should_block(attempt_count, limit) => Err(rate_limit_exceeded_error()),
        Ok(_) => Ok(()),
        Err(error) => {
            tracing::warn!(
                error = %error,
                scope,
                "Redis-backed rate limiting failed, falling back to in-memory limiter"
            );
            enforce_in_memory_rate_limit(scope, subject, limit, window)
        }
    }
}

fn enforce_in_memory_rate_limit(
    scope: &str,
    subject: &str,
    limit: usize,
    window: Duration,
) -> Result<(), AppError> {
    let now = Instant::now();
    let key = format!("{scope}:{subject}");
    let store = RATE_LIMITS.get_or_init(|| Mutex::new(HashMap::new()));
    let mut store = store
        .lock()
        .map_err(|_| AppError::Internal(anyhow::anyhow!("Rate limit store is poisoned")))?;

    let attempts = store.entry(key).or_default();
    attempts.retain(|instant| now.duration_since(*instant) <= window);

    if attempts.len() >= limit {
        return Err(rate_limit_exceeded_error());
    }

    attempts.push(now);
    Ok(())
}

async fn increment_redis_attempt(
    redis: &ConnectionManager,
    key: &str,
    window: Duration,
) -> Result<i64, redis::RedisError> {
    let mut conn = redis.clone();
    let ttl_secs = window.as_secs().max(1);

    Script::new(RATE_LIMIT_SCRIPT)
        .key(key)
        .arg(ttl_secs)
        .invoke_async(&mut conn)
        .await
}

fn redis_rate_limit_key(scope: &str, subject: &str) -> String {
    format!("rate_limit:{scope}:{}", hex::encode(subject.as_bytes()))
}

fn should_block(attempt_count: i64, limit: usize) -> bool {
    attempt_count > limit as i64
}

fn rate_limit_exceeded_error() -> AppError {
    AppError::Validation("Too many attempts. Please wait a few minutes and try again.".to_string())
}

pub fn rate_limit_subject(headers: &axum::http::HeaderMap, fallback: &str) -> String {
    headers
        .get("x-forwarded-for")
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.split(',').next())
        .or_else(|| {
            headers
                .get("x-real-ip")
                .and_then(|value| value.to_str().ok())
        })
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or(fallback)
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rate_limit_allows_first_attempt() {
        let result = enforce_in_memory_rate_limit("test", "subject-a", 1, Duration::from_secs(60));
        assert!(result.is_ok());
    }

    #[test]
    fn rate_limit_blocks_after_limit() {
        let subject = format!("subject-{}", uuid::Uuid::new_v4());
        assert!(enforce_in_memory_rate_limit("test", &subject, 1, Duration::from_secs(60)).is_ok());
        assert!(
            enforce_in_memory_rate_limit("test", &subject, 1, Duration::from_secs(60)).is_err()
        );
    }

    #[test]
    fn redis_rate_limit_key_hex_encodes_subject() {
        let key = redis_rate_limit_key("public_subscribe", "127.0.0.1:user@example.com");
        assert!(key.starts_with("rate_limit:public_subscribe:"));
        assert!(!key.contains('@'));
    }

    #[test]
    fn should_block_only_after_limit_is_exceeded() {
        assert!(!should_block(1, 1));
        assert!(!should_block(3, 3));
        assert!(should_block(4, 3));
    }
}
