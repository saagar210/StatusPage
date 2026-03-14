use shared::error::AppError;
use shared::models::webhook::{
    CreateWebhookConfigRequest, UpdateWebhookConfigRequest, WebhookConfig,
};
use sqlx::PgPool;
use uuid::Uuid;

pub async fn find_by_org(pool: &PgPool, org_id: Uuid) -> Result<Vec<WebhookConfig>, AppError> {
    let webhooks = sqlx::query_as::<_, WebhookConfig>(
        r#"
        SELECT id, org_id, name, url, event_types, is_enabled, created_at, updated_at
             , disabled_reason
        FROM webhook_configs
        WHERE org_id = $1
        ORDER BY created_at DESC
        "#,
    )
    .bind(org_id)
    .fetch_all(pool)
    .await?;

    Ok(webhooks)
}

pub async fn create(
    pool: &PgPool,
    org_id: Uuid,
    req: &CreateWebhookConfigRequest,
) -> Result<WebhookConfig, AppError> {
    let webhook = sqlx::query_as::<_, WebhookConfig>(
        r#"
        INSERT INTO webhook_configs (org_id, name, url, secret, event_types, is_enabled, disabled_reason)
        VALUES ($1, $2, $3, $4, $5, COALESCE($6, true), NULL)
        RETURNING id, org_id, name, url, event_types, is_enabled, disabled_reason, created_at, updated_at
        "#,
    )
    .bind(org_id)
    .bind(&req.name)
    .bind(&req.url)
    .bind(&req.secret)
    .bind(&req.event_types)
    .bind(req.is_enabled)
    .fetch_one(pool)
    .await?;

    Ok(webhook)
}

pub async fn update(
    pool: &PgPool,
    id: Uuid,
    org_id: Uuid,
    req: &UpdateWebhookConfigRequest,
) -> Result<WebhookConfig, AppError> {
    let webhook = sqlx::query_as::<_, WebhookConfig>(
        r#"
        UPDATE webhook_configs SET
            name = COALESCE($3, name),
            url = COALESCE($4, url),
            secret = COALESCE($5, secret),
            event_types = COALESCE($6, event_types),
            is_enabled = COALESCE($7, is_enabled),
            disabled_reason = CASE
                WHEN COALESCE($7, is_enabled) THEN NULL
                ELSE disabled_reason
            END,
            updated_at = NOW()
        WHERE id = $1 AND org_id = $2
        RETURNING id, org_id, name, url, event_types, is_enabled, disabled_reason, created_at, updated_at
        "#,
    )
    .bind(id)
    .bind(org_id)
    .bind(&req.name)
    .bind(&req.url)
    .bind(&req.secret)
    .bind(&req.event_types)
    .bind(req.is_enabled)
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| AppError::NotFound("Webhook not found".to_string()))?;

    Ok(webhook)
}

pub async fn delete(pool: &PgPool, id: Uuid, org_id: Uuid) -> Result<(), AppError> {
    let result = sqlx::query("DELETE FROM webhook_configs WHERE id = $1 AND org_id = $2")
        .bind(id)
        .bind(org_id)
        .execute(pool)
        .await?;

    if result.rows_affected() == 0 {
        return Err(AppError::NotFound("Webhook not found".to_string()));
    }

    Ok(())
}

pub async fn disable_all_for_org(pool: &PgPool, org_id: Uuid) -> Result<(), AppError> {
    sqlx::query(
        r#"
        UPDATE webhook_configs
        SET is_enabled = FALSE, disabled_reason = 'plan_limit', updated_at = NOW()
        WHERE org_id = $1 AND is_enabled = TRUE
        "#,
    )
    .bind(org_id)
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn restore_plan_limited(pool: &PgPool, org_id: Uuid) -> Result<(), AppError> {
    sqlx::query(
        r#"
        UPDATE webhook_configs
        SET is_enabled = TRUE, disabled_reason = NULL, updated_at = NOW()
        WHERE org_id = $1 AND disabled_reason = 'plan_limit'
        "#,
    )
    .bind(org_id)
    .execute(pool)
    .await?;

    Ok(())
}
