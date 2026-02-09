use sqlx::PgPool;

use crate::config::Config;

#[derive(Clone)]
pub struct AppState {
    pub pool: PgPool,
    #[allow(dead_code)]
    pub config: Config,
}
