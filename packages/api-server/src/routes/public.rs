use axum::{
    extract::{Path, Query, State},
    routing::get,
    Json, Router,
};
use chrono::{Duration, Utc};
use serde::{Deserialize, Serialize};

use shared::enums::{IncidentStatus, ServiceStatus};
use shared::error::AppError;
use shared::models::incident::Incident;
use shared::models::incident_update::IncidentUpdate;

use crate::state::AppState;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/:slug/status", get(get_status))
        .route("/:slug/incidents", get(get_incident_history))
        .route("/:slug/uptime", get(get_uptime))
}

#[derive(Serialize)]
struct DataResponse<T: Serialize> {
    data: T,
}

// --- Status endpoint ---

#[derive(Serialize)]
struct StatusResponse {
    organization: PublicOrg,
    overall_status: ServiceStatus,
    services: Vec<PublicService>,
    active_incidents: Vec<PublicIncident>,
}

#[derive(Serialize)]
struct PublicOrg {
    name: String,
    logo_url: Option<String>,
    brand_color: String,
}

#[derive(Serialize, sqlx::FromRow)]
struct PublicService {
    id: uuid::Uuid,
    name: String,
    current_status: ServiceStatus,
    group_name: Option<String>,
}

#[derive(Serialize)]
struct PublicIncident {
    id: uuid::Uuid,
    title: String,
    status: IncidentStatus,
    impact: shared::enums::IncidentImpact,
    started_at: chrono::DateTime<Utc>,
    updates: Vec<IncidentUpdate>,
    affected_services: Vec<String>,
}

async fn get_status(
    State(state): State<AppState>,
    Path(slug): Path<String>,
) -> Result<Json<DataResponse<StatusResponse>>, AppError> {
    // Get org
    let org = sqlx::query_as::<_, OrgRow>(
        "SELECT id, name, logo_url, brand_color FROM organizations WHERE slug = $1",
    )
    .bind(&slug)
    .fetch_optional(&state.pool)
    .await?
    .ok_or_else(|| AppError::NotFound("Status page not found".to_string()))?;

    // Get visible services
    let services = sqlx::query_as::<_, PublicService>(
        "SELECT id, name, current_status, group_name FROM services WHERE org_id = $1 AND is_visible = true ORDER BY display_order",
    )
    .bind(org.id)
    .fetch_all(&state.pool)
    .await?;

    // Calculate overall status (worst case)
    let overall_status = services
        .iter()
        .map(|s| &s.current_status)
        .fold(ServiceStatus::Operational, |worst, status| {
            worst_status(&worst, status)
        });

    // Get active incidents
    let active_incidents_raw = sqlx::query_as::<_, Incident>(
        "SELECT * FROM incidents WHERE org_id = $1 AND status != 'resolved' ORDER BY created_at DESC",
    )
    .bind(org.id)
    .fetch_all(&state.pool)
    .await?;

    let mut active_incidents = Vec::new();
    for incident in active_incidents_raw {
        let updates = sqlx::query_as::<_, IncidentUpdate>(
            "SELECT * FROM incident_updates WHERE incident_id = $1 ORDER BY created_at DESC",
        )
        .bind(incident.id)
        .fetch_all(&state.pool)
        .await?;

        let affected: Vec<String> = sqlx::query_scalar(
            r#"
            SELECT s.name FROM services s
            JOIN incident_services isvc ON isvc.service_id = s.id
            WHERE isvc.incident_id = $1
            "#,
        )
        .bind(incident.id)
        .fetch_all(&state.pool)
        .await?;

        active_incidents.push(PublicIncident {
            id: incident.id,
            title: incident.title,
            status: incident.status,
            impact: incident.impact,
            started_at: incident.started_at,
            updates,
            affected_services: affected,
        });
    }

    Ok(Json(DataResponse {
        data: StatusResponse {
            organization: PublicOrg {
                name: org.name,
                logo_url: org.logo_url,
                brand_color: org.brand_color,
            },
            overall_status,
            services,
            active_incidents,
        },
    }))
}

#[derive(sqlx::FromRow)]
struct OrgRow {
    id: uuid::Uuid,
    name: String,
    logo_url: Option<String>,
    brand_color: String,
}

fn worst_status(a: &ServiceStatus, b: &ServiceStatus) -> ServiceStatus {
    let severity = |s: &ServiceStatus| -> u8 {
        match s {
            ServiceStatus::MajorOutage => 4,
            ServiceStatus::PartialOutage => 3,
            ServiceStatus::DegradedPerformance => 2,
            ServiceStatus::UnderMaintenance => 1,
            ServiceStatus::Operational => 0,
        }
    };
    if severity(a) >= severity(b) {
        *a
    } else {
        *b
    }
}

// --- Incident history endpoint ---

#[derive(Deserialize)]
struct HistoryParams {
    page: Option<i64>,
    per_page: Option<i64>,
}

#[derive(Serialize)]
struct HistoryResponse {
    incidents: Vec<PublicIncident>,
    pagination: PaginationInfo,
}

#[derive(Serialize)]
struct PaginationInfo {
    page: i64,
    per_page: i64,
    total: i64,
}

async fn get_incident_history(
    State(state): State<AppState>,
    Path(slug): Path<String>,
    Query(params): Query<HistoryParams>,
) -> Result<Json<DataResponse<HistoryResponse>>, AppError> {
    let org = sqlx::query_as::<_, OrgRow>(
        "SELECT id, name, logo_url, brand_color FROM organizations WHERE slug = $1",
    )
    .bind(&slug)
    .fetch_optional(&state.pool)
    .await?
    .ok_or_else(|| AppError::NotFound("Status page not found".to_string()))?;

    let page = params.page.unwrap_or(1).max(1);
    let per_page = params.per_page.unwrap_or(20).clamp(1, 100);
    let offset = (page - 1) * per_page;
    let ninety_days_ago = Utc::now() - Duration::days(90);

    let total: i64 = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM incidents WHERE org_id = $1 AND status = 'resolved' AND created_at > $2",
    )
    .bind(org.id)
    .bind(ninety_days_ago)
    .fetch_one(&state.pool)
    .await
    .unwrap_or(0);

    let incidents_raw = sqlx::query_as::<_, Incident>(
        r#"
        SELECT * FROM incidents
        WHERE org_id = $1 AND status = 'resolved' AND created_at > $2
        ORDER BY created_at DESC
        LIMIT $3 OFFSET $4
        "#,
    )
    .bind(org.id)
    .bind(ninety_days_ago)
    .bind(per_page)
    .bind(offset)
    .fetch_all(&state.pool)
    .await?;

    let mut incidents = Vec::new();
    for incident in incidents_raw {
        let updates = sqlx::query_as::<_, IncidentUpdate>(
            "SELECT * FROM incident_updates WHERE incident_id = $1 ORDER BY created_at DESC",
        )
        .bind(incident.id)
        .fetch_all(&state.pool)
        .await?;

        let affected: Vec<String> = sqlx::query_scalar(
            "SELECT s.name FROM services s JOIN incident_services isvc ON isvc.service_id = s.id WHERE isvc.incident_id = $1",
        )
        .bind(incident.id)
        .fetch_all(&state.pool)
        .await?;

        incidents.push(PublicIncident {
            id: incident.id,
            title: incident.title,
            status: incident.status,
            impact: incident.impact,
            started_at: incident.started_at,
            updates,
            affected_services: affected,
        });
    }

    Ok(Json(DataResponse {
        data: HistoryResponse {
            incidents,
            pagination: PaginationInfo {
                page,
                per_page,
                total,
            },
        },
    }))
}

// --- Uptime endpoint ---

#[derive(Serialize)]
struct UptimeResponse {
    services: Vec<ServiceUptime>,
}

#[derive(Serialize)]
struct ServiceUptime {
    service_id: uuid::Uuid,
    service_name: String,
    days: Vec<UptimeDay>,
    overall_uptime: Option<f64>,
}

#[derive(Serialize)]
struct UptimeDay {
    date: chrono::NaiveDate,
    uptime_percentage: Option<f64>,
    avg_response_time_ms: Option<f64>,
}

async fn get_uptime(
    State(state): State<AppState>,
    Path(slug): Path<String>,
) -> Result<Json<DataResponse<UptimeResponse>>, AppError> {
    let org = sqlx::query_as::<_, OrgRow>(
        "SELECT id, name, logo_url, brand_color FROM organizations WHERE slug = $1",
    )
    .bind(&slug)
    .fetch_optional(&state.pool)
    .await?
    .ok_or_else(|| AppError::NotFound("Status page not found".to_string()))?;

    let services = sqlx::query_as::<_, PublicService>(
        "SELECT id, name, current_status, group_name FROM services WHERE org_id = $1 AND is_visible = true ORDER BY display_order",
    )
    .bind(org.id)
    .fetch_all(&state.pool)
    .await?;

    let today = Utc::now().date_naive();
    let ninety_days_ago = today - Duration::days(89);

    let mut service_uptimes = Vec::new();

    for service in &services {
        // Try to get uptime data from uptime_daily via monitor
        let daily_data: Vec<DailyRow> = sqlx::query_as(
            r#"
            SELECT ud.date, ud.uptime_percentage, ud.avg_response_time_ms
            FROM uptime_daily ud
            JOIN monitors m ON m.id = ud.monitor_id
            WHERE m.service_id = $1 AND ud.date >= $2
            ORDER BY ud.date
            "#,
        )
        .bind(service.id)
        .bind(ninety_days_ago)
        .fetch_all(&state.pool)
        .await
        .unwrap_or_default();

        let daily_map: std::collections::HashMap<chrono::NaiveDate, &DailyRow> =
            daily_data.iter().map(|d| (d.date, d)).collect();

        let mut days = Vec::with_capacity(90);
        let mut total_checks_weighted = 0.0_f64;
        let mut total_uptime_weighted = 0.0_f64;

        for i in 0..90 {
            let date = ninety_days_ago + Duration::days(i);
            let day = if let Some(row) = daily_map.get(&date) {
                if let Some(pct) = row.uptime_percentage {
                    total_checks_weighted += 1.0;
                    total_uptime_weighted += pct;
                }
                UptimeDay {
                    date,
                    uptime_percentage: row.uptime_percentage,
                    avg_response_time_ms: row.avg_response_time_ms,
                }
            } else {
                UptimeDay {
                    date,
                    uptime_percentage: None,
                    avg_response_time_ms: None,
                }
            };
            days.push(day);
        }

        let overall_uptime = if total_checks_weighted > 0.0 {
            Some(total_uptime_weighted / total_checks_weighted)
        } else {
            None
        };

        service_uptimes.push(ServiceUptime {
            service_id: service.id,
            service_name: service.name.clone(),
            days,
            overall_uptime,
        });
    }

    Ok(Json(DataResponse {
        data: UptimeResponse {
            services: service_uptimes,
        },
    }))
}

#[derive(sqlx::FromRow)]
struct DailyRow {
    date: chrono::NaiveDate,
    uptime_percentage: Option<f64>,
    avg_response_time_ms: Option<f64>,
}
