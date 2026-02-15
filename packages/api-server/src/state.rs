use redis::aio::ConnectionManager;
use sqlx::PgPool;

use crate::config::Config;
use crate::services::redis_publisher::RedisPublisher;

#[derive(Clone)]
pub struct AppState {
    pub pool: PgPool,
    pub redis: ConnectionManager,
    pub publisher: RedisPublisher,
    #[allow(dead_code)]
    pub config: Config,
}
