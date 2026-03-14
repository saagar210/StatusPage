use serde::Serialize;
use sqlx::PgPool;
use uuid::Uuid;

use shared::error::AppError;

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct AuditLogEntry {
    pub id: Uuid,
    pub org_id: Uuid,
    pub actor_user_id: Option<Uuid>,
    pub actor_type: String,
    pub action: String,
    pub target_type: String,
    pub target_id: Option<String>,
    pub details: serde_json::Value,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

pub struct NewAuditLog<'a> {
    pub org_id: Uuid,
    pub actor_user_id: Option<Uuid>,
    pub actor_type: &'a str,
    pub action: &'a str,
    pub target_type: &'a str,
    pub target_id: Option<&'a str>,
    pub details: serde_json::Value,
}

pub async fn record(pool: &PgPool, entry: NewAuditLog<'_>) -> Result<(), AppError> {
    sqlx::query(
        r#"
        INSERT INTO audit_logs (
            org_id,
            actor_user_id,
            actor_type,
            action,
            target_type,
            target_id,
            details
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7)
        "#,
    )
    .bind(entry.org_id)
    .bind(entry.actor_user_id)
    .bind(entry.actor_type)
    .bind(entry.action)
    .bind(entry.target_type)
    .bind(entry.target_id)
    .bind(sqlx::types::Json(entry.details))
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn list_recent_by_org(
    pool: &PgPool,
    org_id: Uuid,
    limit: i64,
) -> Result<Vec<AuditLogEntry>, AppError> {
    let entries = sqlx::query_as::<_, AuditLogEntry>(
        r#"
        SELECT
            id,
            org_id,
            actor_user_id,
            actor_type,
            action,
            target_type,
            target_id,
            details,
            created_at
        FROM audit_logs
        WHERE org_id = $1
        ORDER BY created_at DESC
        LIMIT $2
        "#,
    )
    .bind(org_id)
    .bind(limit)
    .fetch_all(pool)
    .await?;

    Ok(entries)
}
