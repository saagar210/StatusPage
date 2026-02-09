use shared::error::AppError;
use shared::models::incident_update::{CreateIncidentUpdateRequest, IncidentUpdate};
use sqlx::PgPool;
use uuid::Uuid;

pub async fn create(
    pool: &PgPool,
    incident_id: Uuid,
    req: &CreateIncidentUpdateRequest,
    user_id: Uuid,
) -> Result<IncidentUpdate, AppError> {
    let update = sqlx::query_as::<_, IncidentUpdate>(
        r#"
        INSERT INTO incident_updates (incident_id, status, message, created_by)
        VALUES ($1, $2, $3, $4)
        RETURNING *
        "#,
    )
    .bind(incident_id)
    .bind(req.status)
    .bind(&req.message)
    .bind(user_id)
    .fetch_one(pool)
    .await?;

    Ok(update)
}

#[allow(dead_code)]
pub async fn find_by_incident(
    pool: &PgPool,
    incident_id: Uuid,
) -> Result<Vec<IncidentUpdate>, AppError> {
    let updates = sqlx::query_as::<_, IncidentUpdate>(
        "SELECT * FROM incident_updates WHERE incident_id = $1 ORDER BY created_at DESC",
    )
    .bind(incident_id)
    .fetch_all(pool)
    .await?;

    Ok(updates)
}
