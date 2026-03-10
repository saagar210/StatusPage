use chrono::{Datelike, NaiveDate, Utc};
use shared::enums::{IncidentImpact, IncidentStatus, ServiceStatus};
use shared::models::incident_update::IncidentUpdate;
use shared::models::monitor::Monitor;
use sqlx::types::Json;
use sqlx::{PgPool, Row};
use uuid::Uuid;

use crate::checker::CheckResult;

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct ServiceSnapshot {
    pub service_id: Uuid,
    pub service_name: String,
    pub current_status: ServiceStatus,
}

#[derive(Debug, Clone)]
pub struct AutoIncidentCreated {
    pub incident_id: Uuid,
    pub title: String,
    pub status: IncidentStatus,
    pub impact: IncidentImpact,
    pub affected_services: Vec<Uuid>,
}

#[derive(Debug, Clone)]
pub struct AutoIncidentResolved {
    pub incident_id: Uuid,
    pub update: IncidentUpdate,
}

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

pub async fn get_service_snapshot(
    pool: &PgPool,
    service_id: Uuid,
) -> anyhow::Result<ServiceSnapshot> {
    let snapshot = sqlx::query_as::<_, ServiceSnapshot>(
        r#"
        SELECT
            id as service_id,
            name as service_name,
            current_status
        FROM services
        WHERE id = $1
        "#,
    )
    .bind(service_id)
    .fetch_one(pool)
    .await?;

    Ok(snapshot)
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
) -> anyhow::Result<AutoIncidentCreated> {
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

    Ok(AutoIncidentCreated {
        incident_id,
        title: format!("{} is experiencing issues", service_name),
        status: IncidentStatus::Investigating,
        impact: IncidentImpact::Major,
        affected_services: vec![service_id],
    })
}

pub async fn resolve_auto_incident(
    pool: &PgPool,
    service_id: Uuid,
) -> anyhow::Result<Option<AutoIncidentResolved>> {
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

        let update = sqlx::query_as::<_, IncidentUpdate>(
            r#"
            INSERT INTO incident_updates (incident_id, status, message)
            VALUES ($1, 'resolved', 'Service has recovered. Automated monitoring confirmed recovery.')
            RETURNING *
            "#,
        )
        .bind(id)
        .fetch_one(&mut *tx)
        .await?;

        tx.commit().await?;

        return Ok(Some(AutoIncidentResolved {
            incident_id: id,
            update,
        }));
    }

    Ok(None)
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

pub async fn enqueue_webhook_deliveries<T: serde::Serialize>(
    pool: &PgPool,
    org_id: Uuid,
    event_type: &str,
    payload: &T,
) -> anyhow::Result<u64> {
    ensure_notification_preferences(pool, org_id).await?;

    let payload = Json(serde_json::to_value(payload)?);
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

pub async fn enqueue_incident_notification_emails(
    pool: &PgPool,
    org_id: Uuid,
    event_type: &str,
    title: &str,
    status: IncidentStatus,
    message: &str,
) -> anyhow::Result<u64> {
    if !email_event_enabled(pool, org_id, event_type).await? {
        return Ok(0);
    }

    let org_slug: String = sqlx::query_scalar("SELECT slug FROM organizations WHERE id = $1")
        .bind(org_id)
        .fetch_one(pool)
        .await?;
    let app_base_url =
        std::env::var("APP_BASE_URL").unwrap_or_else(|_| "http://localhost:3000".to_string());

    let subscribers = sqlx::query(
        r#"
        SELECT email, unsubscribe_token
        FROM subscribers
        WHERE org_id = $1 AND is_verified = TRUE
        "#,
    )
    .bind(org_id)
    .fetch_all(pool)
    .await?;

    let notification_type = if status == IncidentStatus::Resolved {
        "incident_resolved"
    } else if event_type == "incident.created" {
        "incident_created"
    } else {
        "incident_updated"
    };

    let mut inserted = 0;
    for subscriber in subscribers {
        let email: String = subscriber.get("email");
        let unsubscribe_token: String = subscriber.get("unsubscribe_token");
        let subject = if status == IncidentStatus::Resolved {
            format!("Incident resolved: {title}")
        } else if event_type == "incident.created" {
            format!("New incident: {title}")
        } else {
            format!("Incident update: {title}")
        };
        let unsubscribe_link = format!(
            "{}/s/{}/unsubscribe?token={}",
            app_base_url, org_slug, unsubscribe_token
        );
        let body = format!(
            "Status update for {title}\n\nStatus: {}\nMessage: {}\n\nFollow updates at:\n{}/s/{}/history\n\nUnsubscribe:\n{unsubscribe_link}",
            status.as_str(),
            message,
            app_base_url,
            org_slug,
        );

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
            VALUES ($1, $2, 'subscriber', $3, $4, $5, 'pending', NOW())
            "#,
        )
        .bind(org_id)
        .bind(notification_type)
        .bind(email)
        .bind(subject)
        .bind(body)
        .execute(pool)
        .await?;
        inserted += 1;
    }

    Ok(inserted)
}

pub async fn enqueue_service_status_notification_emails(
    pool: &PgPool,
    org_id: Uuid,
    service_name: &str,
    old_status: ServiceStatus,
    new_status: ServiceStatus,
) -> anyhow::Result<u64> {
    if !email_event_enabled(pool, org_id, "service.status_changed").await? {
        return Ok(0);
    }

    let org_slug: String = sqlx::query_scalar("SELECT slug FROM organizations WHERE id = $1")
        .bind(org_id)
        .fetch_one(pool)
        .await?;
    let app_base_url =
        std::env::var("APP_BASE_URL").unwrap_or_else(|_| "http://localhost:3000".to_string());

    let subscribers = sqlx::query(
        r#"
        SELECT email, unsubscribe_token
        FROM subscribers
        WHERE org_id = $1 AND is_verified = TRUE
        "#,
    )
    .bind(org_id)
    .fetch_all(pool)
    .await?;

    let mut inserted = 0;
    for subscriber in subscribers {
        let email: String = subscriber.get("email");
        let unsubscribe_token: String = subscriber.get("unsubscribe_token");
        let subject = format!("Service status changed: {service_name}");
        let unsubscribe_link = format!(
            "{}/s/{}/unsubscribe?token={}",
            app_base_url, org_slug, unsubscribe_token
        );
        let body = format!(
            "{service_name} changed from {} to {}.\n\nSee current status at:\n{}/s/{}\n\nUnsubscribe:\n{unsubscribe_link}",
            old_status.as_str(),
            new_status.as_str(),
            app_base_url,
            org_slug,
        );

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
            VALUES ($1, 'service_status_changed', 'subscriber', $2, $3, $4, 'pending', NOW())
            "#,
        )
        .bind(org_id)
        .bind(email)
        .bind(subject)
        .bind(body)
        .execute(pool)
        .await?;
        inserted += 1;
    }

    Ok(inserted)
}

async fn ensure_notification_preferences(pool: &PgPool, org_id: Uuid) -> anyhow::Result<()> {
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

async fn email_event_enabled(
    pool: &PgPool,
    org_id: Uuid,
    event_type: &str,
) -> anyhow::Result<bool> {
    ensure_notification_preferences(pool, org_id).await?;

    let row = sqlx::query(
        r#"
        SELECT
            email_on_incident_created,
            email_on_incident_updated,
            email_on_incident_resolved,
            email_on_service_status_changed
        FROM notification_preferences
        WHERE org_id = $1
        "#,
    )
    .bind(org_id)
    .fetch_one(pool)
    .await?;

    Ok(match event_type {
        "incident.created" => row.get("email_on_incident_created"),
        "incident.updated" => row.get("email_on_incident_updated"),
        "incident.resolved" => row.get("email_on_incident_resolved"),
        "service.status_changed" => row.get("email_on_service_status_changed"),
        _ => false,
    })
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
