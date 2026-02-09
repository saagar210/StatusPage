use axum::{
    extract::{Path, State},
    routing::{get, patch, post},
    Json, Router,
};
use serde::Serialize;
use uuid::Uuid;

use shared::error::AppError;
use shared::models::service::{
    CreateServiceRequest, ReorderServicesRequest, Service, UpdateServiceRequest,
};

use crate::db;
use crate::middleware::org_access::OrgAccess;
use crate::state::AppState;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", post(create_service).get(list_services))
        .route(
            "/:id",
            get(get_service)
                .patch(update_service)
                .delete(delete_service),
        )
        .route("/reorder", patch(reorder_services))
}

#[derive(Serialize)]
struct DataResponse<T: Serialize> {
    data: T,
}

async fn create_service(
    State(state): State<AppState>,
    org_access: OrgAccess,
    Json(req): Json<CreateServiceRequest>,
) -> Result<(axum::http::StatusCode, Json<DataResponse<Service>>), AppError> {
    org_access.require_admin()?;

    if req.name.trim().is_empty() {
        return Err(AppError::Validation("Service name is required".to_string()));
    }

    let service = db::services::create(&state.pool, org_access.org.id, &req).await?;
    Ok((
        axum::http::StatusCode::CREATED,
        Json(DataResponse { data: service }),
    ))
}

async fn list_services(
    State(state): State<AppState>,
    org_access: OrgAccess,
) -> Result<Json<DataResponse<Vec<Service>>>, AppError> {
    let services = db::services::find_by_org(&state.pool, org_access.org.id).await?;
    Ok(Json(DataResponse { data: services }))
}

async fn get_service(
    State(state): State<AppState>,
    org_access: OrgAccess,
    Path((_slug, id)): Path<(String, Uuid)>,
) -> Result<Json<DataResponse<Service>>, AppError> {
    let service = db::services::find_by_id(&state.pool, id, org_access.org.id)
        .await?
        .ok_or_else(|| AppError::NotFound("Service not found".to_string()))?;
    Ok(Json(DataResponse { data: service }))
}

async fn update_service(
    State(state): State<AppState>,
    org_access: OrgAccess,
    Path((_slug, id)): Path<(String, Uuid)>,
    Json(req): Json<UpdateServiceRequest>,
) -> Result<Json<DataResponse<Service>>, AppError> {
    org_access.require_admin()?;

    let service = db::services::update(&state.pool, id, org_access.org.id, &req).await?;
    Ok(Json(DataResponse { data: service }))
}

async fn delete_service(
    State(state): State<AppState>,
    org_access: OrgAccess,
    Path((_slug, id)): Path<(String, Uuid)>,
) -> Result<axum::http::StatusCode, AppError> {
    org_access.require_admin()?;

    db::services::delete(&state.pool, id, org_access.org.id).await?;
    Ok(axum::http::StatusCode::NO_CONTENT)
}

async fn reorder_services(
    State(state): State<AppState>,
    org_access: OrgAccess,
    Json(req): Json<ReorderServicesRequest>,
) -> Result<axum::http::StatusCode, AppError> {
    org_access.require_admin()?;

    db::services::reorder(&state.pool, org_access.org.id, &req.service_ids).await?;
    Ok(axum::http::StatusCode::NO_CONTENT)
}
