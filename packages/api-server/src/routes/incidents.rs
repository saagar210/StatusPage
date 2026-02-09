use axum::{
    extract::{Path, Query, State},
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use shared::enums::IncidentStatus;
use shared::error::AppError;
use shared::models::incident::{
    CreateIncidentRequest, Incident, IncidentWithDetails, UpdateIncidentRequest,
};
use shared::models::incident_update::{CreateIncidentUpdateRequest, IncidentUpdate};

use crate::db;
use crate::middleware::org_access::OrgAccess;
use crate::state::AppState;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", post(create_incident).get(list_incidents))
        .route(
            "/:id",
            get(get_incident)
                .patch(update_incident)
                .delete(delete_incident),
        )
        .route("/:id/updates", post(create_update))
}

#[derive(Serialize)]
struct DataResponse<T: Serialize> {
    data: T,
}

#[derive(Serialize)]
struct ListResponse<T: Serialize> {
    data: T,
    pagination: Pagination,
}

#[derive(Serialize)]
struct Pagination {
    page: i64,
    per_page: i64,
    total: i64,
}

#[derive(Deserialize)]
struct ListParams {
    status: Option<IncidentStatus>,
    page: Option<i64>,
    per_page: Option<i64>,
}

async fn create_incident(
    State(state): State<AppState>,
    org_access: OrgAccess,
    Json(req): Json<CreateIncidentRequest>,
) -> Result<(axum::http::StatusCode, Json<DataResponse<Incident>>), AppError> {
    org_access.require_admin()?;

    if req.title.trim().is_empty() {
        return Err(AppError::Validation("Title is required".to_string()));
    }
    if req.message.trim().is_empty() {
        return Err(AppError::Validation("Message is required".to_string()));
    }

    let incident =
        db::incidents::create(&state.pool, org_access.org.id, &req, org_access.user.id).await?;

    Ok((
        axum::http::StatusCode::CREATED,
        Json(DataResponse { data: incident }),
    ))
}

async fn list_incidents(
    State(state): State<AppState>,
    org_access: OrgAccess,
    Query(params): Query<ListParams>,
) -> Result<Json<ListResponse<Vec<Incident>>>, AppError> {
    let page = params.page.unwrap_or(1).max(1);
    let per_page = params.per_page.unwrap_or(20).clamp(1, 100);

    let (incidents, total) = db::incidents::find_by_org(
        &state.pool,
        org_access.org.id,
        params.status,
        page,
        per_page,
    )
    .await?;

    Ok(Json(ListResponse {
        data: incidents,
        pagination: Pagination {
            page,
            per_page,
            total,
        },
    }))
}

async fn get_incident(
    State(state): State<AppState>,
    org_access: OrgAccess,
    Path((_slug, id)): Path<(String, Uuid)>,
) -> Result<Json<DataResponse<IncidentWithDetails>>, AppError> {
    let incident = db::incidents::find_by_id_with_details(&state.pool, id, org_access.org.id)
        .await?
        .ok_or_else(|| AppError::NotFound("Incident not found".to_string()))?;

    Ok(Json(DataResponse { data: incident }))
}

async fn update_incident(
    State(state): State<AppState>,
    org_access: OrgAccess,
    Path((_slug, id)): Path<(String, Uuid)>,
    Json(req): Json<UpdateIncidentRequest>,
) -> Result<Json<DataResponse<Incident>>, AppError> {
    org_access.require_admin()?;

    // If status change to resolved, use the resolve-and-recalculate logic
    if req.status == Some(IncidentStatus::Resolved) {
        let incident = db::incidents::resolve_and_recalculate(
            &state.pool,
            id,
            org_access.org.id,
            org_access.user.id,
        )
        .await?;
        return Ok(Json(DataResponse { data: incident }));
    }

    // Validate status transition if status is being changed
    if let Some(new_status) = req.status {
        let current = db::incidents::find_by_id_with_details(&state.pool, id, org_access.org.id)
            .await?
            .ok_or_else(|| AppError::NotFound("Incident not found".to_string()))?;

        if !current.incident.status.can_transition_to(&new_status) {
            return Err(AppError::Validation(format!(
                "Cannot transition from {} to {}",
                current.incident.status, new_status
            )));
        }
    }

    let incident = db::incidents::update_status(
        &state.pool,
        id,
        org_access.org.id,
        req.status.unwrap_or(IncidentStatus::Investigating),
        req.impact,
        req.title,
    )
    .await?;

    Ok(Json(DataResponse { data: incident }))
}

async fn delete_incident(
    State(state): State<AppState>,
    org_access: OrgAccess,
    Path((_slug, id)): Path<(String, Uuid)>,
) -> Result<axum::http::StatusCode, AppError> {
    org_access.require_owner()?;

    db::incidents::delete(&state.pool, id, org_access.org.id).await?;
    Ok(axum::http::StatusCode::NO_CONTENT)
}

async fn create_update(
    State(state): State<AppState>,
    org_access: OrgAccess,
    Path((_slug, id)): Path<(String, Uuid)>,
    Json(req): Json<CreateIncidentUpdateRequest>,
) -> Result<(axum::http::StatusCode, Json<DataResponse<IncidentUpdate>>), AppError> {
    org_access.require_admin()?;

    if req.message.trim().is_empty() {
        return Err(AppError::Validation("Message is required".to_string()));
    }

    // Validate status transition
    let incident = db::incidents::find_by_id_with_details(&state.pool, id, org_access.org.id)
        .await?
        .ok_or_else(|| AppError::NotFound("Incident not found".to_string()))?;

    if !incident.incident.status.can_transition_to(&req.status) {
        return Err(AppError::Validation(format!(
            "Cannot transition from {} to {}",
            incident.incident.status, req.status
        )));
    }

    // Create the update
    let update = db::incident_updates::create(&state.pool, id, &req, org_access.user.id).await?;

    // Also update incident status if changed
    if req.status != incident.incident.status {
        if req.status == IncidentStatus::Resolved {
            db::incidents::resolve_and_recalculate(
                &state.pool,
                id,
                org_access.org.id,
                org_access.user.id,
            )
            .await?;
        } else {
            db::incidents::update_status(
                &state.pool,
                id,
                org_access.org.id,
                req.status,
                None,
                None,
            )
            .await?;
        }
    }

    Ok((
        axum::http::StatusCode::CREATED,
        Json(DataResponse { data: update }),
    ))
}
