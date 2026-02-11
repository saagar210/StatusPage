use axum::{
    extract::{Path, Query, State},
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use shared::enums::OrganizationPlan;
use shared::error::AppError;
use shared::models::monitor::{CreateMonitorRequest, Monitor, MonitorCheck, UpdateMonitorRequest};

use crate::db;
use crate::middleware::org_access::OrgAccess;
use crate::state::AppState;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", post(create_monitor).get(list_monitors))
        .route(
            "/:id",
            get(get_monitor)
                .patch(update_monitor)
                .delete(delete_monitor),
        )
        .route("/:id/checks", get(get_check_history))
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
    page: Option<i64>,
    per_page: Option<i64>,
}

async fn create_monitor(
    State(state): State<AppState>,
    org_access: OrgAccess,
    Json(req): Json<CreateMonitorRequest>,
) -> Result<(axum::http::StatusCode, Json<DataResponse<Monitor>>), AppError> {
    org_access.require_admin()?;

    // Validate interval
    if let Some(interval) = req.interval_seconds {
        if !(30..=300).contains(&interval) {
            return Err(AppError::Validation(
                "Interval must be between 30 and 300 seconds".to_string(),
            ));
        }
    }

    // Validate timeout
    if let Some(timeout) = req.timeout_ms {
        if !(1000..=30000).contains(&timeout) {
            return Err(AppError::Validation(
                "Timeout must be between 1000 and 30000 milliseconds".to_string(),
            ));
        }
    }

    if let Some(limit) = org_access.org.plan.max_monitors() {
        let monitor_count = db::monitors::count_by_org(&state.pool, org_access.org.id).await?;
        if monitor_count >= limit {
            return Err(AppError::Validation(format!(
                "{} plan allows up to {} monitors. Upgrade to add more.",
                plan_name(org_access.org.plan),
                limit
            )));
        }
    }

    let monitor = db::monitors::create(&state.pool, org_access.org.id, &req).await?;
    Ok((
        axum::http::StatusCode::CREATED,
        Json(DataResponse { data: monitor }),
    ))
}

fn plan_name(plan: OrganizationPlan) -> &'static str {
    match plan {
        OrganizationPlan::Free => "Free",
        OrganizationPlan::Pro => "Pro",
        OrganizationPlan::Team => "Team",
    }
}

async fn list_monitors(
    State(state): State<AppState>,
    org_access: OrgAccess,
) -> Result<Json<DataResponse<Vec<Monitor>>>, AppError> {
    let monitors = db::monitors::find_by_org(&state.pool, org_access.org.id).await?;
    Ok(Json(DataResponse { data: monitors }))
}

async fn get_monitor(
    State(state): State<AppState>,
    org_access: OrgAccess,
    Path((_slug, id)): Path<(String, Uuid)>,
) -> Result<Json<DataResponse<Monitor>>, AppError> {
    let monitor = db::monitors::find_by_id(&state.pool, id, org_access.org.id)
        .await?
        .ok_or_else(|| AppError::NotFound("Monitor not found".to_string()))?;
    Ok(Json(DataResponse { data: monitor }))
}

async fn update_monitor(
    State(state): State<AppState>,
    org_access: OrgAccess,
    Path((_slug, id)): Path<(String, Uuid)>,
    Json(req): Json<UpdateMonitorRequest>,
) -> Result<Json<DataResponse<Monitor>>, AppError> {
    org_access.require_admin()?;

    let monitor = db::monitors::update(&state.pool, id, org_access.org.id, &req).await?;
    Ok(Json(DataResponse { data: monitor }))
}

async fn delete_monitor(
    State(state): State<AppState>,
    org_access: OrgAccess,
    Path((_slug, id)): Path<(String, Uuid)>,
) -> Result<axum::http::StatusCode, AppError> {
    org_access.require_admin()?;

    db::monitors::delete(&state.pool, id, org_access.org.id).await?;
    Ok(axum::http::StatusCode::NO_CONTENT)
}

async fn get_check_history(
    State(state): State<AppState>,
    org_access: OrgAccess,
    Path((_slug, id)): Path<(String, Uuid)>,
    Query(params): Query<ListParams>,
) -> Result<Json<ListResponse<Vec<MonitorCheck>>>, AppError> {
    // Verify monitor belongs to org
    db::monitors::find_by_id(&state.pool, id, org_access.org.id)
        .await?
        .ok_or_else(|| AppError::NotFound("Monitor not found".to_string()))?;

    let page = params.page.unwrap_or(1).max(1);
    let per_page = params.per_page.unwrap_or(50).clamp(1, 100);

    let (checks, total) = db::monitors::get_check_history(&state.pool, id, page, per_page).await?;

    Ok(Json(ListResponse {
        data: checks,
        pagination: Pagination {
            page,
            per_page,
            total,
        },
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plan_name_formats_for_messages() {
        assert_eq!(plan_name(OrganizationPlan::Free), "Free");
        assert_eq!(plan_name(OrganizationPlan::Pro), "Pro");
        assert_eq!(plan_name(OrganizationPlan::Team), "Team");
    }
}
