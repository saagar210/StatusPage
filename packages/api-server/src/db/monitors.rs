use shared::error::AppError;
use shared::models::monitor::{CreateMonitorRequest, Monitor, UpdateMonitorRequest};
use sqlx::PgPool;
use uuid::Uuid;

pub async fn create(
    pool: &PgPool,
    org_id: Uuid,
    req: &CreateMonitorRequest,
) -> Result<Monitor, AppError> {
    // Verify service belongs to org
    let service_exists: bool =
        sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM services WHERE id = $1 AND org_id = $2)")
            .bind(req.service_id)
            .bind(org_id)
            .fetch_one(pool)
            .await?;

    if !service_exists {
        return Err(AppError::NotFound(
            "Service not found in this organization".to_string(),
        ));
    }

    let monitor = sqlx::query_as::<_, Monitor>(
        r#"
        INSERT INTO monitors (service_id, org_id, monitor_type, config, interval_seconds, timeout_ms, failure_threshold)
        VALUES ($1, $2, $3, $4, $5, $6, $7)
        RETURNING *
        "#,
    )
    .bind(req.service_id)
    .bind(org_id)
    .bind(req.monitor_type)
    .bind(&req.config)
    .bind(req.interval_seconds.unwrap_or(60))
    .bind(req.timeout_ms.unwrap_or(10000))
    .bind(req.failure_threshold.unwrap_or(3))
    .fetch_one(pool)
    .await?;

    Ok(monitor)
}

pub async fn find_by_org(pool: &PgPool, org_id: Uuid) -> Result<Vec<Monitor>, AppError> {
    let monitors = sqlx::query_as::<_, Monitor>(
        "SELECT * FROM monitors WHERE org_id = $1 ORDER BY created_at",
    )
    .bind(org_id)
    .fetch_all(pool)
    .await?;

    Ok(monitors)
}

pub async fn count_by_org(pool: &PgPool, org_id: Uuid) -> Result<i64, AppError> {
    let total: i64 =
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM monitors WHERE org_id = $1")
            .bind(org_id)
            .fetch_one(pool)
            .await?;

    Ok(total)
}

pub async fn find_by_id(
    pool: &PgPool,
    monitor_id: Uuid,
    org_id: Uuid,
) -> Result<Option<Monitor>, AppError> {
    let monitor =
        sqlx::query_as::<_, Monitor>("SELECT * FROM monitors WHERE id = $1 AND org_id = $2")
            .bind(monitor_id)
            .bind(org_id)
            .fetch_optional(pool)
            .await?;

    Ok(monitor)
}

pub async fn update(
    pool: &PgPool,
    monitor_id: Uuid,
    org_id: Uuid,
    req: &UpdateMonitorRequest,
) -> Result<Monitor, AppError> {
    let monitor = sqlx::query_as::<_, Monitor>(
        r#"
        UPDATE monitors SET
            config = COALESCE($3, config),
            interval_seconds = COALESCE($4, interval_seconds),
            timeout_ms = COALESCE($5, timeout_ms),
            failure_threshold = COALESCE($6, failure_threshold),
            is_active = COALESCE($7, is_active),
            updated_at = NOW()
        WHERE id = $1 AND org_id = $2
        RETURNING *
        "#,
    )
    .bind(monitor_id)
    .bind(org_id)
    .bind(&req.config)
    .bind(req.interval_seconds)
    .bind(req.timeout_ms)
    .bind(req.failure_threshold)
    .bind(req.is_active)
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| AppError::NotFound("Monitor not found".to_string()))?;

    Ok(monitor)
}

pub async fn delete(pool: &PgPool, monitor_id: Uuid, org_id: Uuid) -> Result<(), AppError> {
    let result = sqlx::query("DELETE FROM monitors WHERE id = $1 AND org_id = $2")
        .bind(monitor_id)
        .bind(org_id)
        .execute(pool)
        .await?;

    if result.rows_affected() == 0 {
        return Err(AppError::NotFound("Monitor not found".to_string()));
    }

    Ok(())
}

pub async fn get_check_history(
    pool: &PgPool,
    monitor_id: Uuid,
    page: i64,
    per_page: i64,
) -> Result<(Vec<shared::models::monitor::MonitorCheck>, i64), AppError> {
    let offset = (page - 1) * per_page;

    let total: i64 =
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM monitor_checks WHERE monitor_id = $1")
            .bind(monitor_id)
            .fetch_one(pool)
            .await
            .unwrap_or(0);

    let checks: Vec<shared::models::monitor::MonitorCheck> = sqlx::query_as(
        r#"
        SELECT id, monitor_id, status, response_time_ms, status_code, error_message, checked_at
        FROM monitor_checks
        WHERE monitor_id = $1
        ORDER BY checked_at DESC
        LIMIT $2 OFFSET $3
        "#,
    )
    .bind(monitor_id)
    .bind(per_page)
    .bind(offset)
    .fetch_all(pool)
    .await?;

    Ok((checks, total))
}
