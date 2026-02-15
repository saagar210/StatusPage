pub mod incidents;
pub mod monitors;
pub mod organizations;
pub mod public;
pub mod services;

use axum::{extract::State, http::StatusCode, routing::get, Router};
use redis::AsyncCommands;
use serde::Serialize;

use crate::state::AppState;

pub fn api_router(state: AppState) -> Router {
    Router::new()
        .route("/health", get(health))
        .nest("/api/organizations", organizations::router())
        .nest("/api/organizations/:slug/services", services::router())
        .nest("/api/organizations/:slug/incidents", incidents::router())
        .nest("/api/organizations/:slug/monitors", monitors::router())
        .nest("/api/public", public::router())
        .with_state(state)
}

#[derive(Serialize)]
struct HealthResponse {
    status: &'static str,
    database: &'static str,
    redis: &'static str,
}

async fn health(State(state): State<AppState>) -> Result<axum::Json<HealthResponse>, StatusCode> {
    // Check database connection
    let db_status = match sqlx::query("SELECT 1").fetch_one(&state.pool).await {
        Ok(_) => "ok",
        Err(_) => "error",
    };

    // Check Redis connection - try to get a connection
    let redis_status = match state.redis.clone().get::<_, Option<String>>("health_check").await {
        Ok(_) => "ok",
        Err(_) => "error",
    };

    let overall_status = if db_status == "ok" && redis_status == "ok" {
        "ok"
    } else {
        "degraded"
    };

    let response = HealthResponse {
        status: overall_status,
        database: db_status,
        redis: redis_status,
    };

    if overall_status == "ok" {
        Ok(axum::Json(response))
    } else {
        Err(StatusCode::SERVICE_UNAVAILABLE)
    }
}
