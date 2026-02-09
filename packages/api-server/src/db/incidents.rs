use shared::enums::{IncidentImpact, IncidentStatus, ServiceStatus};
use shared::error::AppError;
use shared::models::incident::{
    AffectedService, CreateIncidentRequest, Incident, IncidentWithDetails,
};
use shared::models::incident_update::IncidentUpdate;
use sqlx::PgPool;
use uuid::Uuid;

pub async fn create(
    pool: &PgPool,
    org_id: Uuid,
    req: &CreateIncidentRequest,
    user_id: Uuid,
) -> Result<Incident, AppError> {
    if req.affected_service_ids.is_empty() {
        return Err(AppError::Validation(
            "At least one affected service is required".to_string(),
        ));
    }

    let mut tx = pool.begin().await?;

    let status = req.status.unwrap_or(IncidentStatus::Investigating);

    // 1. Insert incident
    let incident = sqlx::query_as::<_, Incident>(
        r#"
        INSERT INTO incidents (org_id, title, status, impact, created_by)
        VALUES ($1, $2, $3, $4, $5)
        RETURNING *
        "#,
    )
    .bind(org_id)
    .bind(&req.title)
    .bind(status)
    .bind(req.impact)
    .bind(user_id)
    .fetch_one(&mut *tx)
    .await?;

    // 2. Insert initial update
    sqlx::query(
        r#"
        INSERT INTO incident_updates (incident_id, status, message, created_by)
        VALUES ($1, $2, $3, $4)
        "#,
    )
    .bind(incident.id)
    .bind(status)
    .bind(&req.message)
    .bind(user_id)
    .execute(&mut *tx)
    .await?;

    // 3. Link affected services
    for service_id in &req.affected_service_ids {
        sqlx::query("INSERT INTO incident_services (incident_id, service_id) VALUES ($1, $2)")
            .bind(incident.id)
            .bind(service_id)
            .execute(&mut *tx)
            .await?;
    }

    // 4. Update affected service statuses based on impact
    let new_status = req.impact.to_service_status();
    if new_status != ServiceStatus::Operational {
        for service_id in &req.affected_service_ids {
            sqlx::query(
                "UPDATE services SET current_status = $1, updated_at = NOW() WHERE id = $2 AND org_id = $3",
            )
            .bind(new_status)
            .bind(service_id)
            .bind(org_id)
            .execute(&mut *tx)
            .await?;
        }
    }

    tx.commit().await?;
    Ok(incident)
}

pub async fn find_by_org(
    pool: &PgPool,
    org_id: Uuid,
    status_filter: Option<IncidentStatus>,
    page: i64,
    per_page: i64,
) -> Result<(Vec<Incident>, i64), AppError> {
    let offset = (page - 1) * per_page;

    let total: i64 = if let Some(status) = status_filter {
        sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM incidents WHERE org_id = $1 AND status = $2",
        )
        .bind(org_id)
        .bind(status)
        .fetch_one(pool)
        .await
        .unwrap_or(0)
    } else {
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM incidents WHERE org_id = $1")
            .bind(org_id)
            .fetch_one(pool)
            .await
            .unwrap_or(0)
    };

    let incidents = if let Some(status) = status_filter {
        sqlx::query_as::<_, Incident>(
            r#"
            SELECT * FROM incidents
            WHERE org_id = $1 AND status = $2
            ORDER BY
                CASE WHEN status != 'resolved' THEN 0 ELSE 1 END,
                created_at DESC
            LIMIT $3 OFFSET $4
            "#,
        )
        .bind(org_id)
        .bind(status)
        .bind(per_page)
        .bind(offset)
        .fetch_all(pool)
        .await?
    } else {
        sqlx::query_as::<_, Incident>(
            r#"
            SELECT * FROM incidents
            WHERE org_id = $1
            ORDER BY
                CASE WHEN status != 'resolved' THEN 0 ELSE 1 END,
                created_at DESC
            LIMIT $2 OFFSET $3
            "#,
        )
        .bind(org_id)
        .bind(per_page)
        .bind(offset)
        .fetch_all(pool)
        .await?
    };

    Ok((incidents, total))
}

pub async fn find_by_id_with_details(
    pool: &PgPool,
    incident_id: Uuid,
    org_id: Uuid,
) -> Result<Option<IncidentWithDetails>, AppError> {
    let incident =
        sqlx::query_as::<_, Incident>("SELECT * FROM incidents WHERE id = $1 AND org_id = $2")
            .bind(incident_id)
            .bind(org_id)
            .fetch_optional(pool)
            .await?;

    let incident = match incident {
        Some(i) => i,
        None => return Ok(None),
    };

    let updates = sqlx::query_as::<_, IncidentUpdate>(
        "SELECT * FROM incident_updates WHERE incident_id = $1 ORDER BY created_at DESC",
    )
    .bind(incident_id)
    .fetch_all(pool)
    .await?;

    let affected_services = sqlx::query_as::<_, AffectedService>(
        r#"
        SELECT s.id as service_id, s.name as service_name
        FROM services s
        JOIN incident_services isvc ON isvc.service_id = s.id
        WHERE isvc.incident_id = $1
        "#,
    )
    .bind(incident_id)
    .fetch_all(pool)
    .await?;

    Ok(Some(IncidentWithDetails {
        incident,
        updates,
        affected_services,
    }))
}

pub async fn update_status(
    pool: &PgPool,
    incident_id: Uuid,
    org_id: Uuid,
    new_status: IncidentStatus,
    new_impact: Option<IncidentImpact>,
    title: Option<String>,
) -> Result<Incident, AppError> {
    let incident = sqlx::query_as::<_, Incident>(
        r#"
        UPDATE incidents SET
            status = $3,
            impact = COALESCE($4, impact),
            title = COALESCE($5, title),
            resolved_at = CASE WHEN $3 = 'resolved' THEN NOW() ELSE resolved_at END,
            updated_at = NOW()
        WHERE id = $1 AND org_id = $2
        RETURNING *
        "#,
    )
    .bind(incident_id)
    .bind(org_id)
    .bind(new_status)
    .bind(new_impact)
    .bind(title)
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| AppError::NotFound("Incident not found".to_string()))?;

    Ok(incident)
}

/// When resolving an incident, recalculate affected service statuses.
pub async fn resolve_and_recalculate(
    pool: &PgPool,
    incident_id: Uuid,
    org_id: Uuid,
    user_id: Uuid,
) -> Result<Incident, AppError> {
    let mut tx = pool.begin().await?;

    // Update incident to resolved
    let incident = sqlx::query_as::<_, Incident>(
        r#"
        UPDATE incidents SET
            status = 'resolved',
            resolved_at = NOW(),
            updated_at = NOW()
        WHERE id = $1 AND org_id = $2
        RETURNING *
        "#,
    )
    .bind(incident_id)
    .bind(org_id)
    .fetch_optional(&mut *tx)
    .await?
    .ok_or_else(|| AppError::NotFound("Incident not found".to_string()))?;

    // Add resolution update
    sqlx::query(
        r#"
        INSERT INTO incident_updates (incident_id, status, message, created_by)
        VALUES ($1, 'resolved', 'This incident has been resolved.', $2)
        "#,
    )
    .bind(incident_id)
    .bind(user_id)
    .execute(&mut *tx)
    .await?;

    // Get affected services
    let service_ids: Vec<Uuid> =
        sqlx::query_scalar("SELECT service_id FROM incident_services WHERE incident_id = $1")
            .bind(incident_id)
            .fetch_all(&mut *tx)
            .await?;

    // For each affected service, recalculate status
    for service_id in service_ids {
        let worst_impact: Option<IncidentImpact> = sqlx::query_scalar(
            r#"
            SELECT i.impact FROM incidents i
            JOIN incident_services isvc ON isvc.incident_id = i.id
            WHERE isvc.service_id = $1
              AND i.id != $2
              AND i.status != 'resolved'
            ORDER BY
                CASE i.impact
                    WHEN 'critical' THEN 0
                    WHEN 'major' THEN 1
                    WHEN 'minor' THEN 2
                    WHEN 'none' THEN 3
                END
            LIMIT 1
            "#,
        )
        .bind(service_id)
        .bind(incident_id)
        .fetch_optional(&mut *tx)
        .await?;

        let new_status = match worst_impact {
            Some(impact) => impact.to_service_status(),
            None => ServiceStatus::Operational,
        };

        sqlx::query("UPDATE services SET current_status = $1, updated_at = NOW() WHERE id = $2")
            .bind(new_status)
            .bind(service_id)
            .execute(&mut *tx)
            .await?;
    }

    tx.commit().await?;
    Ok(incident)
}

pub async fn delete(pool: &PgPool, incident_id: Uuid, org_id: Uuid) -> Result<(), AppError> {
    // Get affected services before deletion for status recalculation
    let service_ids: Vec<Uuid> =
        sqlx::query_scalar("SELECT service_id FROM incident_services WHERE incident_id = $1")
            .bind(incident_id)
            .fetch_all(pool)
            .await?;

    let result = sqlx::query("DELETE FROM incidents WHERE id = $1 AND org_id = $2")
        .bind(incident_id)
        .bind(org_id)
        .execute(pool)
        .await?;

    if result.rows_affected() == 0 {
        return Err(AppError::NotFound("Incident not found".to_string()));
    }

    // Recalculate service statuses after deletion
    for service_id in service_ids {
        let worst_impact: Option<IncidentImpact> = sqlx::query_scalar(
            r#"
            SELECT i.impact FROM incidents i
            JOIN incident_services isvc ON isvc.incident_id = i.id
            WHERE isvc.service_id = $1 AND i.status != 'resolved'
            ORDER BY
                CASE i.impact
                    WHEN 'critical' THEN 0
                    WHEN 'major' THEN 1
                    WHEN 'minor' THEN 2
                    WHEN 'none' THEN 3
                END
            LIMIT 1
            "#,
        )
        .bind(service_id)
        .fetch_optional(pool)
        .await?;

        let new_status = match worst_impact {
            Some(impact) => impact.to_service_status(),
            None => ServiceStatus::Operational,
        };

        sqlx::query("UPDATE services SET current_status = $1, updated_at = NOW() WHERE id = $2")
            .bind(new_status)
            .bind(service_id)
            .execute(pool)
            .await?;
    }

    Ok(())
}
