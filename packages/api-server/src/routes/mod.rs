pub mod incidents;
pub mod monitors;
pub mod notifications;
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
        .route("/ready", get(ready))
        .route("/ops/summary", get(ops_summary))
        .nest("/api/organizations", organizations::router())
        .nest("/api/organizations/{slug}/services", services::router())
        .nest("/api/organizations/{slug}/incidents", incidents::router())
        .nest("/api/organizations/{slug}/monitors", monitors::router())
        .nest(
            "/api/organizations/{slug}/notifications",
            notifications::router(),
        )
        .nest("/api/public", public::router())
        .with_state(state)
}

#[derive(Serialize)]
struct HealthResponse {
    status: &'static str,
    database: &'static str,
    redis: &'static str,
}

#[derive(Serialize)]
struct OpsSummaryResponse {
    status: &'static str,
    database: &'static str,
    redis: &'static str,
    organizations: i64,
    services: i64,
    monitors: i64,
    active_incidents: i64,
    subscribers: i64,
    pending_email_deliveries: i64,
    failed_email_deliveries: i64,
    pending_webhook_deliveries: i64,
    failed_webhook_deliveries: i64,
}

async fn health(State(state): State<AppState>) -> Result<axum::Json<HealthResponse>, StatusCode> {
    // Check database connection
    let db_status = match sqlx::query("SELECT 1").fetch_one(&state.pool).await {
        Ok(_) => "ok",
        Err(_) => "error",
    };

    // Check Redis connection - try to get a connection
    let redis_status = match state
        .redis
        .clone()
        .get::<_, Option<String>>("health_check")
        .await
    {
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

async fn ready(State(state): State<AppState>) -> Result<StatusCode, StatusCode> {
    let db_ok = sqlx::query("SELECT 1").fetch_one(&state.pool).await.is_ok();
    let redis_ok = state
        .redis
        .clone()
        .get::<_, Option<String>>("health_check")
        .await
        .is_ok();

    if db_ok && redis_ok {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(StatusCode::SERVICE_UNAVAILABLE)
    }
}

async fn ops_summary(
    State(state): State<AppState>,
) -> Result<axum::Json<OpsSummaryResponse>, StatusCode> {
    let db_status = match sqlx::query("SELECT 1").fetch_one(&state.pool).await {
        Ok(_) => "ok",
        Err(_) => "error",
    };
    let redis_status = match state
        .redis
        .clone()
        .get::<_, Option<String>>("health_check")
        .await
    {
        Ok(_) => "ok",
        Err(_) => "error",
    };

    let counts = async {
        let organizations = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM organizations")
            .fetch_one(&state.pool)
            .await?;
        let services = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM services")
            .fetch_one(&state.pool)
            .await?;
        let monitors = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM monitors")
            .fetch_one(&state.pool)
            .await?;
        let active_incidents = sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM incidents WHERE status != 'resolved'",
        )
        .fetch_one(&state.pool)
        .await?;
        let subscribers = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM subscribers")
            .fetch_one(&state.pool)
            .await?;
        let pending_email_deliveries = sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM notification_logs WHERE status = 'pending'",
        )
        .fetch_one(&state.pool)
        .await?;
        let failed_email_deliveries = sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM notification_logs WHERE status = 'failed'",
        )
        .fetch_one(&state.pool)
        .await?;
        let pending_webhook_deliveries = sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM webhook_deliveries WHERE status = 'pending'",
        )
        .fetch_one(&state.pool)
        .await?;
        let failed_webhook_deliveries = sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM webhook_deliveries WHERE status = 'failed'",
        )
        .fetch_one(&state.pool)
        .await?;

        Ok::<_, sqlx::Error>((
            organizations,
            services,
            monitors,
            active_incidents,
            subscribers,
            pending_email_deliveries,
            failed_email_deliveries,
            pending_webhook_deliveries,
            failed_webhook_deliveries,
        ))
    }
    .await
    .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;

    let status = if db_status == "ok" && redis_status == "ok" {
        "ok"
    } else {
        "degraded"
    };

    Ok(axum::Json(OpsSummaryResponse {
        status,
        database: db_status,
        redis: redis_status,
        organizations: counts.0,
        services: counts.1,
        monitors: counts.2,
        active_incidents: counts.3,
        subscribers: counts.4,
        pending_email_deliveries: counts.5,
        failed_email_deliveries: counts.6,
        pending_webhook_deliveries: counts.7,
        failed_webhook_deliveries: counts.8,
    }))
}
