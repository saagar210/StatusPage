use chrono::{Datelike, NaiveDate, Utc};
use shared::enums::ServiceStatus;
use shared::models::monitor::Monitor;
use sqlx::PgPool;
use uuid::Uuid;

use crate::checker::CheckResult;

pub async fn get_active_monitors(pool: &PgPool) -> anyhow::Result<Vec<Monitor>> {
    let monitors = sqlx::query_as::<_, Monitor>("SELECT * FROM monitors WHERE is_active = true")
        .fetch_all(pool)
        .await?;

    Ok(monitors)
}

pub async fn insert_check(
    pool: &PgPool,
    monitor_id: Uuid,
    result: &CheckResult,
) -> anyhow::Result<()> {
    // Try to create partition for current month if it doesn't exist
    let now = Utc::now();
    let _ = ensure_partition(pool, now.date_naive()).await;

    sqlx::query(
        r#"
        INSERT INTO monitor_checks (monitor_id, status, response_time_ms, status_code, error_message, checked_at)
        VALUES ($1, $2, $3, $4, $5, NOW())
        "#,
    )
    .bind(monitor_id)
    .bind(result.status)
    .bind(result.response_time_ms as i32)
    .bind(result.status_code.map(|c| c as i32))
    .bind(&result.error_message)
    .execute(pool)
    .await?;

    // Update monitor's last check info
    sqlx::query(
        r#"
        UPDATE monitors SET
            last_checked_at = NOW(),
            last_response_time_ms = $2,
            updated_at = NOW()
        WHERE id = $1
        "#,
    )
    .bind(monitor_id)
    .bind(result.response_time_ms as i32)
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn increment_failures(
    pool: &PgPool,
    monitor_id: Uuid,
    expected_current: i32,
) -> anyhow::Result<bool> {
    let result = sqlx::query(
        r#"
        UPDATE monitors SET consecutive_failures = consecutive_failures + 1, updated_at = NOW()
        WHERE id = $1 AND consecutive_failures = $2
        "#,
    )
    .bind(monitor_id)
    .bind(expected_current)
    .execute(pool)
    .await?;

    Ok(result.rows_affected() > 0)
}

pub async fn reset_failures(pool: &PgPool, monitor_id: Uuid) -> anyhow::Result<()> {
    sqlx::query("UPDATE monitors SET consecutive_failures = 0, updated_at = NOW() WHERE id = $1")
        .bind(monitor_id)
        .execute(pool)
        .await?;

    Ok(())
}

pub async fn update_service_status(
    pool: &PgPool,
    service_id: Uuid,
    status: ServiceStatus,
) -> anyhow::Result<()> {
    sqlx::query("UPDATE services SET current_status = $1, updated_at = NOW() WHERE id = $2")
        .bind(status)
        .bind(service_id)
        .execute(pool)
        .await?;

    Ok(())
}

pub async fn get_service_current_status(
    pool: &PgPool,
    service_id: Uuid,
) -> anyhow::Result<ServiceStatus> {
    let status: ServiceStatus =
        sqlx::query_scalar("SELECT current_status FROM services WHERE id = $1")
            .bind(service_id)
            .fetch_one(pool)
            .await?;

    Ok(status)
}

pub async fn has_active_auto_incident(pool: &PgPool, service_id: Uuid) -> anyhow::Result<bool> {
    let exists: bool = sqlx::query_scalar(
        r#"
        SELECT EXISTS(
            SELECT 1 FROM incidents i
            JOIN incident_services isvc ON isvc.incident_id = i.id
            WHERE isvc.service_id = $1 AND i.is_auto = true AND i.status != 'resolved'
        )
        "#,
    )
    .bind(service_id)
    .fetch_one(pool)
    .await?;

    Ok(exists)
}

pub async fn create_auto_incident(
    pool: &PgPool,
    org_id: Uuid,
    service_id: Uuid,
    error_message: &str,
) -> anyhow::Result<()> {
    let mut tx = pool.begin().await?;

    // Get service name
    let service_name: String = sqlx::query_scalar("SELECT name FROM services WHERE id = $1")
        .bind(service_id)
        .fetch_one(&mut *tx)
        .await?;

    let incident_id = Uuid::new_v4();

    sqlx::query(
        r#"
        INSERT INTO incidents (id, org_id, title, status, impact, is_auto, started_at)
        VALUES ($1, $2, $3, 'investigating', 'major', true, NOW())
        "#,
    )
    .bind(incident_id)
    .bind(org_id)
    .bind(format!("{} is experiencing issues", service_name))
    .execute(&mut *tx)
    .await?;

    sqlx::query(
        r#"
        INSERT INTO incident_updates (incident_id, status, message)
        VALUES ($1, 'investigating', $2)
        "#,
    )
    .bind(incident_id)
    .bind(format!(
        "Automated monitoring detected failures: {}",
        error_message
    ))
    .execute(&mut *tx)
    .await?;

    sqlx::query("INSERT INTO incident_services (incident_id, service_id) VALUES ($1, $2)")
        .bind(incident_id)
        .bind(service_id)
        .execute(&mut *tx)
        .await?;

    tx.commit().await?;
    Ok(())
}

pub async fn resolve_auto_incident(pool: &PgPool, service_id: Uuid) -> anyhow::Result<()> {
    // Find active auto-incident for this service
    let incident_id: Option<Uuid> = sqlx::query_scalar(
        r#"
        SELECT i.id FROM incidents i
        JOIN incident_services isvc ON isvc.incident_id = i.id
        WHERE isvc.service_id = $1 AND i.is_auto = true AND i.status != 'resolved'
        LIMIT 1
        "#,
    )
    .bind(service_id)
    .fetch_optional(pool)
    .await?;

    if let Some(id) = incident_id {
        let mut tx = pool.begin().await?;

        sqlx::query(
            r#"
            UPDATE incidents SET status = 'resolved', resolved_at = NOW(), updated_at = NOW()
            WHERE id = $1
            "#,
        )
        .bind(id)
        .execute(&mut *tx)
        .await?;

        sqlx::query(
            r#"
            INSERT INTO incident_updates (incident_id, status, message)
            VALUES ($1, 'resolved', 'Service has recovered. Automated monitoring confirmed recovery.')
            "#,
        )
        .bind(id)
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;
    }

    Ok(())
}

pub async fn get_other_failing_monitors_for_service(
    pool: &PgPool,
    service_id: Uuid,
    exclude_monitor_id: Uuid,
) -> anyhow::Result<bool> {
    let has_failures: bool = sqlx::query_scalar(
        r#"
        SELECT EXISTS(
            SELECT 1 FROM monitors
            WHERE service_id = $1 AND id != $2 AND is_active = true
              AND consecutive_failures >= failure_threshold
        )
        "#,
    )
    .bind(service_id)
    .bind(exclude_monitor_id)
    .fetch_one(pool)
    .await?;

    Ok(has_failures)
}

pub async fn ensure_partition(pool: &PgPool, date: NaiveDate) -> anyhow::Result<()> {
    sqlx::query("SELECT create_monthly_partition('monitor_checks', $1::DATE)")
        .bind(date)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn ensure_upcoming_partitions(pool: &PgPool) -> anyhow::Result<()> {
    let now = Utc::now().date_naive();
    ensure_partition(pool, now).await?;

    // Next month
    let next_month = if now.month() == 12 {
        NaiveDate::from_ymd_opt(now.year() + 1, 1, 1).unwrap()
    } else {
        NaiveDate::from_ymd_opt(now.year(), now.month() + 1, 1).unwrap()
    };
    ensure_partition(pool, next_month).await?;

    // Month after
    let month_after = if next_month.month() == 12 {
        NaiveDate::from_ymd_opt(next_month.year() + 1, 1, 1).unwrap()
    } else {
        NaiveDate::from_ymd_opt(next_month.year(), next_month.month() + 1, 1).unwrap()
    };
    ensure_partition(pool, month_after).await?;

    Ok(())
}

pub async fn rollup_daily(pool: &PgPool, monitor_id: Uuid, date: NaiveDate) -> anyhow::Result<()> {
    sqlx::query(
        r#"
        INSERT INTO uptime_daily (monitor_id, date, total_checks, successful_checks,
                                  avg_response_time_ms, min_response_time_ms, max_response_time_ms)
        SELECT
            $1,
            $2,
            COUNT(*)::INT,
            COUNT(*) FILTER (WHERE status = 'success')::INT,
            AVG(response_time_ms)::FLOAT,
            MIN(response_time_ms),
            MAX(response_time_ms)
        FROM monitor_checks
        WHERE monitor_id = $1
          AND checked_at >= $2::DATE
          AND checked_at < ($2::DATE + INTERVAL '1 day')
        HAVING COUNT(*) > 0
        ON CONFLICT (monitor_id, date) DO UPDATE SET
            total_checks = EXCLUDED.total_checks,
            successful_checks = EXCLUDED.successful_checks,
            avg_response_time_ms = EXCLUDED.avg_response_time_ms,
            min_response_time_ms = EXCLUDED.min_response_time_ms,
            max_response_time_ms = EXCLUDED.max_response_time_ms
        "#,
    )
    .bind(monitor_id)
    .bind(date)
    .execute(pool)
    .await?;

    Ok(())
}
