use chrono::Utc;
use serde::Serialize;
use sqlx::PgPool;
use uuid::Uuid;

use shared::error::AppError;

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct PendingNotification {
    pub id: Uuid,
    pub recipient_email: String,
    pub subject: Option<String>,
    pub body_text: String,
    pub attempt_count: i32,
    pub max_attempts: i32,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct NotificationLogEntry {
    pub id: Uuid,
    pub notification_type: String,
    pub recipient_type: String,
    pub recipient_email: String,
    pub subject: Option<String>,
    pub status: String,
    pub error_message: Option<String>,
    pub attempt_count: i32,
    pub max_attempts: i32,
    pub sent_at: Option<chrono::DateTime<chrono::Utc>>,
    pub next_retry_at: Option<chrono::DateTime<chrono::Utc>>,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

pub struct NotificationFailureUpdate<'a> {
    pub attempt_count: i32,
    pub max_attempts: i32,
    pub error_message: Option<&'a str>,
    pub next_retry_at: Option<chrono::DateTime<chrono::Utc>>,
}

pub async fn enqueue(
    pool: &PgPool,
    org_id: Uuid,
    notification_type: &str,
    recipient_type: &str,
    recipient_email: &str,
    subject: &str,
    body_text: &str,
) -> Result<(), AppError> {
    sqlx::query(
        r#"
        INSERT INTO notification_logs (
            org_id,
            notification_type,
            recipient_type,
            recipient_email,
            subject,
            body_text,
            status,
            next_retry_at
        )
        VALUES ($1, $2, $3, $4, $5, $6, 'pending', NOW())
        "#,
    )
    .bind(org_id)
    .bind(notification_type)
    .bind(recipient_type)
    .bind(recipient_email)
    .bind(subject)
    .bind(body_text)
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn claim_pending(
    pool: &PgPool,
    limit: i64,
) -> Result<Vec<PendingNotification>, AppError> {
    let notifications = sqlx::query_as::<_, PendingNotification>(
        r#"
        WITH due AS (
            SELECT id
            FROM notification_logs
            WHERE status = 'pending'
              AND (next_retry_at IS NULL OR next_retry_at <= NOW())
            ORDER BY created_at ASC
            LIMIT $1
            FOR UPDATE SKIP LOCKED
        )
        UPDATE notification_logs nl
        SET
            status = 'sending',
            attempt_count = nl.attempt_count + 1
        FROM due
        WHERE nl.id = due.id
        RETURNING
            nl.id,
            nl.recipient_email,
            nl.subject,
            nl.body_text,
            nl.attempt_count,
            nl.max_attempts
        "#,
    )
    .bind(limit)
    .fetch_all(pool)
    .await?;

    Ok(notifications)
}

pub async fn mark_sent(pool: &PgPool, id: Uuid) -> Result<(), AppError> {
    sqlx::query(
        r#"
        UPDATE notification_logs
        SET
            status = 'sent',
            error_message = NULL,
            next_retry_at = NULL,
            sent_at = NOW()
        WHERE id = $1
        "#,
    )
    .bind(id)
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn mark_failed(
    pool: &PgPool,
    id: Uuid,
    update: NotificationFailureUpdate<'_>,
) -> Result<(), AppError> {
    let status = if update.attempt_count >= update.max_attempts {
        "failed"
    } else {
        "pending"
    };

    sqlx::query(
        r#"
        UPDATE notification_logs
        SET
            status = $2,
            error_message = $3,
            next_retry_at = $4,
            sent_at = NULL
        WHERE id = $1
        "#,
    )
    .bind(id)
    .bind(status)
    .bind(update.error_message)
    .bind(update.next_retry_at)
    .execute(pool)
    .await?;

    Ok(())
}

pub fn next_retry_at(
    attempt_count: i32,
    max_attempts: i32,
) -> Option<chrono::DateTime<chrono::Utc>> {
    if attempt_count >= max_attempts {
        return None;
    }

    let delay_secs = match attempt_count {
        0 | 1 => 15,
        2 => 60,
        3 => 300,
        _ => 900,
    };

    Some(Utc::now() + chrono::Duration::seconds(delay_secs))
}

pub async fn list_recent_by_org(
    pool: &PgPool,
    org_id: Uuid,
    page: i64,
    per_page: i64,
    status: Option<&str>,
) -> Result<(Vec<NotificationLogEntry>, i64), AppError> {
    let total = sqlx::query_scalar::<_, i64>(
        r#"
        SELECT COUNT(*)
        FROM notification_logs
        WHERE org_id = $1
          AND ($2::text IS NULL OR status = $2)
        "#,
    )
    .bind(org_id)
    .bind(status)
    .fetch_one(pool)
    .await?;

    let offset = (page - 1) * per_page;
    let entries = sqlx::query_as::<_, NotificationLogEntry>(
        r#"
        SELECT
            id,
            notification_type,
            recipient_type,
            recipient_email,
            subject,
            status,
            error_message,
            attempt_count,
            max_attempts,
            sent_at,
            next_retry_at,
            created_at
        FROM notification_logs
        WHERE org_id = $1
          AND ($4::text IS NULL OR status = $4)
        ORDER BY created_at DESC
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
    id: Uuid,
) -> Result<Option<NotificationLogEntry>, AppError> {
    let entry = sqlx::query_as::<_, NotificationLogEntry>(
        r#"
        UPDATE notification_logs
        SET
            status = 'pending',
            error_message = NULL,
            next_retry_at = NOW(),
            sent_at = NULL
        WHERE id = $1
          AND org_id = $2
          AND status = 'failed'
        RETURNING
            id,
            notification_type,
            recipient_type,
            recipient_email,
            subject,
            status,
            error_message,
            attempt_count,
            max_attempts,
            sent_at,
            next_retry_at,
            created_at
        "#,
    )
    .bind(id)
    .bind(org_id)
    .fetch_optional(pool)
    .await?;

    Ok(entry)
}
