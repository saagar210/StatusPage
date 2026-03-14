use serde_json::Value;
use shared::error::AppError;
use sqlx::PgPool;
use uuid::Uuid;

#[derive(Debug, Clone, serde::Serialize, sqlx::FromRow)]
pub struct BillingEventEntry {
    pub stripe_event_id: String,
    pub event_type: String,
    pub processed_at: chrono::DateTime<chrono::Utc>,
}

pub async fn exists(pool: &PgPool, stripe_event_id: &str) -> Result<bool, AppError> {
    let exists = sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS(SELECT 1 FROM billing_events WHERE stripe_event_id = $1)",
    )
    .bind(stripe_event_id)
    .fetch_one(pool)
    .await?;

    Ok(exists)
}

pub async fn record(
    pool: &PgPool,
    stripe_event_id: &str,
    event_type: &str,
    org_id: Option<Uuid>,
    payload: &Value,
) -> Result<(), AppError> {
    sqlx::query(
        r#"
        INSERT INTO billing_events (stripe_event_id, event_type, org_id, payload)
        VALUES ($1, $2, $3, $4)
        ON CONFLICT (stripe_event_id) DO NOTHING
        "#,
    )
    .bind(stripe_event_id)
    .bind(event_type)
    .bind(org_id)
    .bind(payload)
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn list_recent_by_org(
    pool: &PgPool,
    org_id: Uuid,
    limit: i64,
) -> Result<Vec<BillingEventEntry>, AppError> {
    let events = sqlx::query_as::<_, BillingEventEntry>(
        r#"
        SELECT stripe_event_id, event_type, processed_at
        FROM billing_events
        WHERE org_id = $1
        ORDER BY processed_at DESC
        LIMIT $2
        "#,
    )
    .bind(org_id)
    .bind(limit)
    .fetch_all(pool)
    .await?;

    Ok(events)
}
