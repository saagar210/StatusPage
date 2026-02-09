use shared::enums::ServiceStatus;
use shared::error::AppError;
use shared::models::service::{CreateServiceRequest, Service, UpdateServiceRequest};
use sqlx::PgPool;
use uuid::Uuid;

pub async fn create(
    pool: &PgPool,
    org_id: Uuid,
    req: &CreateServiceRequest,
) -> Result<Service, AppError> {
    // Auto-set display_order to max + 1
    let max_order: Option<i32> =
        sqlx::query_scalar("SELECT MAX(display_order) FROM services WHERE org_id = $1")
            .bind(org_id)
            .fetch_one(pool)
            .await?;

    let display_order = max_order.unwrap_or(-1) + 1;

    let service = sqlx::query_as::<_, Service>(
        r#"
        INSERT INTO services (org_id, name, description, group_name, is_visible, display_order)
        VALUES ($1, $2, $3, $4, $5, $6)
        RETURNING *
        "#,
    )
    .bind(org_id)
    .bind(&req.name)
    .bind(&req.description)
    .bind(&req.group_name)
    .bind(req.is_visible.unwrap_or(true))
    .bind(display_order)
    .fetch_one(pool)
    .await?;

    Ok(service)
}

pub async fn find_by_org(pool: &PgPool, org_id: Uuid) -> Result<Vec<Service>, AppError> {
    let services = sqlx::query_as::<_, Service>(
        "SELECT * FROM services WHERE org_id = $1 ORDER BY display_order",
    )
    .bind(org_id)
    .fetch_all(pool)
    .await?;

    Ok(services)
}

pub async fn find_by_id(
    pool: &PgPool,
    service_id: Uuid,
    org_id: Uuid,
) -> Result<Option<Service>, AppError> {
    let service =
        sqlx::query_as::<_, Service>("SELECT * FROM services WHERE id = $1 AND org_id = $2")
            .bind(service_id)
            .bind(org_id)
            .fetch_optional(pool)
            .await?;

    Ok(service)
}

pub async fn update(
    pool: &PgPool,
    service_id: Uuid,
    org_id: Uuid,
    req: &UpdateServiceRequest,
) -> Result<Service, AppError> {
    let service = sqlx::query_as::<_, Service>(
        r#"
        UPDATE services SET
            name = COALESCE($3, name),
            description = COALESCE($4, description),
            current_status = COALESCE($5, current_status),
            group_name = COALESCE($6, group_name),
            is_visible = COALESCE($7, is_visible),
            updated_at = NOW()
        WHERE id = $1 AND org_id = $2
        RETURNING *
        "#,
    )
    .bind(service_id)
    .bind(org_id)
    .bind(&req.name)
    .bind(&req.description)
    .bind(req.current_status)
    .bind(&req.group_name)
    .bind(req.is_visible)
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| AppError::NotFound("Service not found".to_string()))?;

    Ok(service)
}

pub async fn delete(pool: &PgPool, service_id: Uuid, org_id: Uuid) -> Result<(), AppError> {
    let result = sqlx::query("DELETE FROM services WHERE id = $1 AND org_id = $2")
        .bind(service_id)
        .bind(org_id)
        .execute(pool)
        .await?;

    if result.rows_affected() == 0 {
        return Err(AppError::NotFound("Service not found".to_string()));
    }

    Ok(())
}

pub async fn reorder(pool: &PgPool, org_id: Uuid, service_ids: &[Uuid]) -> Result<(), AppError> {
    let mut tx = pool.begin().await?;

    for (i, service_id) in service_ids.iter().enumerate() {
        sqlx::query(
            "UPDATE services SET display_order = $1, updated_at = NOW() WHERE id = $2 AND org_id = $3",
        )
        .bind(i as i32)
        .bind(service_id)
        .bind(org_id)
        .execute(&mut *tx)
        .await?;
    }

    tx.commit().await?;
    Ok(())
}

#[allow(dead_code)]
pub async fn update_status(
    pool: &PgPool,
    service_id: Uuid,
    status: ServiceStatus,
) -> Result<(), AppError> {
    sqlx::query("UPDATE services SET current_status = $1, updated_at = NOW() WHERE id = $2")
        .bind(status)
        .bind(service_id)
        .execute(pool)
        .await?;

    Ok(())
}
