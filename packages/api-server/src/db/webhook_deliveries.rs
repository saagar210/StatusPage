use chrono::{DateTime, Utc};
use serde::Serialize;
use sqlx::types::Json;
use sqlx::{PgPool, Postgres, Transaction};
use uuid::Uuid;

use shared::error::AppError;

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct PendingWebhookDelivery {
    pub delivery_id: Uuid,
    pub event_type: String,
    pub payload: Json<serde_json::Value>,
    pub attempt_count: i32,
    pub max_attempts: i32,
    pub url: String,
    pub secret: String,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct WebhookDeliveryEntry {
    pub id: Uuid,
    pub webhook_config_id: Uuid,
    pub webhook_name: String,
    pub webhook_url: String,
    pub event_type: String,
    pub status: String,
    pub response_status_code: Option<i32>,
    pub error_message: Option<String>,
    pub attempt_count: i32,
    pub max_attempts: i32,
    pub next_retry_at: Option<DateTime<Utc>>,
    pub delivered_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

pub struct DeliveryFailureUpdate<'a> {
    pub attempt_count: i32,
    pub max_attempts: i32,
    pub response_status_code: Option<i32>,
    pub response_body: Option<&'a str>,
    pub error_message: Option<&'a str>,
    pub next_retry_at: Option<chrono::DateTime<chrono::Utc>>,
}

pub async fn enqueue_for_event<T: Serialize>(
    pool: &PgPool,
    org_id: Uuid,
    event_type: &str,
    payload: &T,
) -> Result<u64, AppError> {
    ensure_preferences_row(pool, org_id).await?;

    let payload =
        Json(serde_json::to_value(payload).map_err(|error| AppError::Internal(error.into()))?);
    let result = sqlx::query(
        r#"
        INSERT INTO webhook_deliveries (
            webhook_config_id,
            event_type,
            payload,
            status,
            next_retry_at
        )
        SELECT
            wc.id,
            $2,
            $3,
            'pending',
            NOW()
        FROM webhook_configs wc
        JOIN notification_preferences np ON np.org_id = wc.org_id
        WHERE wc.org_id = $1
          AND wc.is_enabled = TRUE
          AND $2 = ANY(wc.event_types)
          AND CASE
                WHEN $2 = 'incident.created' THEN np.webhook_on_incident_created
                WHEN $2 = 'incident.updated' THEN np.webhook_on_incident_updated
                WHEN $2 = 'incident.resolved' THEN np.webhook_on_incident_resolved
                WHEN $2 = 'service.status_changed' THEN np.webhook_on_service_status_changed
                ELSE FALSE
              END = TRUE
        "#,
    )
    .bind(org_id)
    .bind(event_type)
    .bind(payload)
    .execute(pool)
    .await?;

    Ok(result.rows_affected())
}

pub async fn claim_pending(
    pool: &PgPool,
    limit: i64,
) -> Result<Vec<PendingWebhookDelivery>, AppError> {
    let mut tx = pool.begin().await?;
    let claimed = claim_pending_in_tx(&mut tx, limit).await?;
    tx.commit().await?;
    Ok(claimed)
}

async fn claim_pending_in_tx(
    tx: &mut Transaction<'_, Postgres>,
    limit: i64,
) -> Result<Vec<PendingWebhookDelivery>, AppError> {
    let rows = sqlx::query_as::<_, PendingWebhookDelivery>(
        r#"
        WITH due AS (
            SELECT wd.id
            FROM webhook_deliveries wd
            JOIN webhook_configs wc ON wc.id = wd.webhook_config_id
            WHERE wd.status = 'pending'
              AND wc.is_enabled = TRUE
              AND (wd.next_retry_at IS NULL OR wd.next_retry_at <= NOW())
            ORDER BY wd.created_at ASC
            LIMIT $1
            FOR UPDATE SKIP LOCKED
        )
        UPDATE webhook_deliveries wd
        SET
            status = 'sending',
            attempt_count = wd.attempt_count + 1
        FROM due, webhook_configs wc
        WHERE wd.id = due.id
          AND wc.id = wd.webhook_config_id
        RETURNING
            wd.id AS delivery_id,
            wd.event_type,
            wd.payload,
            wd.attempt_count,
            wd.max_attempts,
            wc.url,
            wc.secret
        "#,
    )
    .bind(limit)
    .fetch_all(&mut **tx)
    .await?;

    Ok(rows)
}

pub async fn mark_success(
    pool: &PgPool,
    delivery_id: Uuid,
    response_status_code: Option<i32>,
    response_body: Option<&str>,
) -> Result<(), AppError> {
    sqlx::query(
        r#"
        UPDATE webhook_deliveries
        SET
            status = 'success',
            response_status_code = $2,
            response_body = $3,
            error_message = NULL,
            next_retry_at = NULL,
            delivered_at = NOW()
        WHERE id = $1
        "#,
    )
    .bind(delivery_id)
    .bind(response_status_code)
    .bind(response_body)
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn mark_failure(
    pool: &PgPool,
    delivery_id: Uuid,
    update: DeliveryFailureUpdate<'_>,
) -> Result<(), AppError> {
    let status = if update.attempt_count >= update.max_attempts {
        "failed"
    } else {
        "pending"
    };

    sqlx::query(
        r#"
        UPDATE webhook_deliveries
        SET
            status = $2,
            response_status_code = $3,
            response_body = $4,
            error_message = $5,
            next_retry_at = $6,
            delivered_at = NULL
        WHERE id = $1
        "#,
    )
    .bind(delivery_id)
    .bind(status)
    .bind(update.response_status_code)
    .bind(update.response_body)
    .bind(update.error_message)
    .bind(update.next_retry_at)
    .execute(pool)
    .await?;

    Ok(())
}

async fn ensure_preferences_row(pool: &PgPool, org_id: Uuid) -> Result<(), AppError> {
    sqlx::query(
        r#"
        INSERT INTO notification_preferences (org_id)
        VALUES ($1)
        ON CONFLICT (org_id) DO NOTHING
        "#,
    )
    .bind(org_id)
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn list_recent_by_org(
    pool: &PgPool,
    org_id: Uuid,
    page: i64,
    per_page: i64,
    status: Option<&str>,
) -> Result<(Vec<WebhookDeliveryEntry>, i64), AppError> {
    let total = sqlx::query_scalar::<_, i64>(
        r#"
        SELECT COUNT(*)
        FROM webhook_deliveries wd
        JOIN webhook_configs wc ON wc.id = wd.webhook_config_id
        WHERE wc.org_id = $1
          AND ($2::text IS NULL OR wd.status = $2)
        "#,
    )
    .bind(org_id)
    .bind(status)
    .fetch_one(pool)
    .await?;

    let offset = (page - 1) * per_page;
    let entries = sqlx::query_as::<_, WebhookDeliveryEntry>(
        r#"
        SELECT
            wd.id,
            wd.webhook_config_id,
            wc.name AS webhook_name,
            wc.url AS webhook_url,
            wd.event_type,
            wd.status,
            wd.response_status_code,
            wd.error_message,
            wd.attempt_count,
            wd.max_attempts,
            wd.next_retry_at,
            wd.delivered_at,
            wd.created_at
        FROM webhook_deliveries wd
        JOIN webhook_configs wc ON wc.id = wd.webhook_config_id
        WHERE wc.org_id = $1
          AND ($4::text IS NULL OR wd.status = $4)
        ORDER BY wd.created_at DESC
        LIMIT $2
        OFFSET $3
        "#,
    )
    .bind(org_id)
    .bind(per_page)
    .bind(offset)
    .bind(status)
    .fetch_all(pool)
    .await?;

    Ok((entries, total))
}

pub async fn retry_failed_by_id(
    pool: &PgPool,
    org_id: Uuid,
    delivery_id: Uuid,
) -> Result<Option<WebhookDeliveryEntry>, AppError> {
    let entry = sqlx::query_as::<_, WebhookDeliveryEntry>(
        r#"
        UPDATE webhook_deliveries wd
        SET
            status = 'pending',
            response_status_code = NULL,
            response_body = NULL,
            error_message = NULL,
            next_retry_at = NOW(),
            delivered_at = NULL
        FROM webhook_configs wc
        WHERE wd.id = $1
          AND wc.id = wd.webhook_config_id
          AND wc.org_id = $2
          AND wd.status = 'failed'
        RETURNING
            wd.id,
            wd.webhook_config_id,
            wc.name AS webhook_name,
            wc.url AS webhook_url,
            wd.event_type,
            wd.status,
            wd.response_status_code,
            wd.error_message,
            wd.attempt_count,
            wd.max_attempts,
            wd.next_retry_at,
            wd.delivered_at,
            wd.created_at
        "#,
    )
    .bind(delivery_id)
    .bind(org_id)
    .fetch_optional(pool)
    .await?;

    Ok(entry)
}
