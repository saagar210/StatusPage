use shared::error::AppError;
use shared::models::notification_preference::{
    NotificationPreferences, UpdateNotificationPreferencesRequest,
};
use sqlx::PgPool;
use uuid::Uuid;

pub async fn get_or_create(
    pool: &PgPool,
    org_id: Uuid,
) -> Result<NotificationPreferences, AppError> {
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

    let preferences = sqlx::query_as::<_, NotificationPreferences>(
        r#"
        SELECT
            id,
            org_id,
            email_on_incident_created,
            email_on_incident_updated,
            email_on_incident_resolved,
            email_on_service_status_changed,
            webhook_on_incident_created,
            webhook_on_incident_updated,
            webhook_on_incident_resolved,
            webhook_on_service_status_changed,
            uptime_alert_threshold::float8 as uptime_alert_threshold,
            uptime_alert_enabled,
            created_at,
            updated_at
        FROM notification_preferences
        WHERE org_id = $1
        "#,
    )
    .bind(org_id)
    .fetch_one(pool)
    .await?;

    Ok(preferences)
}

pub async fn update(
    pool: &PgPool,
    org_id: Uuid,
    req: &UpdateNotificationPreferencesRequest,
) -> Result<NotificationPreferences, AppError> {
    let _ = get_or_create(pool, org_id).await?;

    let preferences = sqlx::query_as::<_, NotificationPreferences>(
        r#"
        UPDATE notification_preferences SET
            email_on_incident_created = COALESCE($2, email_on_incident_created),
            email_on_incident_updated = COALESCE($3, email_on_incident_updated),
            email_on_incident_resolved = COALESCE($4, email_on_incident_resolved),
            email_on_service_status_changed = COALESCE($5, email_on_service_status_changed),
            webhook_on_incident_created = COALESCE($6, webhook_on_incident_created),
            webhook_on_incident_updated = COALESCE($7, webhook_on_incident_updated),
            webhook_on_incident_resolved = COALESCE($8, webhook_on_incident_resolved),
            webhook_on_service_status_changed = COALESCE($9, webhook_on_service_status_changed),
            uptime_alert_threshold = COALESCE($10, uptime_alert_threshold),
            uptime_alert_enabled = COALESCE($11, uptime_alert_enabled),
            updated_at = NOW()
        WHERE org_id = $1
        RETURNING
            id,
            org_id,
            email_on_incident_created,
            email_on_incident_updated,
            email_on_incident_resolved,
            email_on_service_status_changed,
            webhook_on_incident_created,
            webhook_on_incident_updated,
            webhook_on_incident_resolved,
            webhook_on_service_status_changed,
            uptime_alert_threshold::float8 as uptime_alert_threshold,
            uptime_alert_enabled,
            created_at,
            updated_at
        "#,
    )
    .bind(org_id)
    .bind(req.email_on_incident_created)
    .bind(req.email_on_incident_updated)
    .bind(req.email_on_incident_resolved)
    .bind(req.email_on_service_status_changed)
    .bind(req.webhook_on_incident_created)
    .bind(req.webhook_on_incident_updated)
    .bind(req.webhook_on_incident_resolved)
    .bind(req.webhook_on_service_status_changed)
    .bind(req.uptime_alert_threshold)
    .bind(req.uptime_alert_enabled)
    .fetch_one(pool)
    .await?;

    Ok(preferences)
}
