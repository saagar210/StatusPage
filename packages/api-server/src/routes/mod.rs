pub mod incidents;
pub mod monitors;
pub mod organizations;
pub mod public;
pub mod services;

use axum::{routing::get, Router};
use serde::Serialize;

use crate::state::AppState;

pub fn api_router(state: AppState) -> Router {
    Router::new()
        .route("/health", get(health))
        .nest("/api/organizations", organizations::router())
        .nest(
            "/api/organizations/:slug/services",
            services::router(),
        )
        .nest(
            "/api/organizations/:slug/incidents",
            incidents::router(),
        )
        .nest(
            "/api/organizations/:slug/monitors",
            monitors::router(),
        )
        .nest("/api/public", public::router())
        .with_state(state)
}

#[derive(Serialize)]
struct HealthResponse {
    status: &'static str,
}

async fn health() -> axum::Json<HealthResponse> {
    axum::Json(HealthResponse { status: "ok" })
}
