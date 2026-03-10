use shared::enums::{CheckStatus, ServiceStatus};
use shared::models::monitor::Monitor;
use sqlx::PgPool;

use crate::checker::CheckResult;
use crate::db;
use crate::redis_publisher::RedisPublisher;

pub async fn evaluate(
    pool: &PgPool,
    monitor: &Monitor,
    result: &CheckResult,
    publisher: Option<&RedisPublisher>,
) -> anyhow::Result<()> {
    // 1. Insert check result
    db::insert_check(pool, monitor.id, result).await?;

    match result.status {
        CheckStatus::Success => handle_success(pool, monitor, publisher).await?,
        CheckStatus::Failure | CheckStatus::Timeout => {
            handle_failure(pool, monitor, result, publisher).await?
        }
    }

    Ok(())
}

async fn handle_success(
    pool: &PgPool,
    monitor: &Monitor,
    publisher: Option<&RedisPublisher>,
) -> anyhow::Result<()> {
    // Reset consecutive failures
    if monitor.consecutive_failures > 0 {
        db::reset_failures(pool, monitor.id).await?;
    }

    // Check if service is in outage and should recover
    let service = db::get_service_snapshot(pool, monitor.service_id).await?;
    if service.current_status != ServiceStatus::Operational
        && service.current_status != ServiceStatus::UnderMaintenance
    {
        // Check if any OTHER monitors for this service are still failing
        let others_failing =
            db::get_other_failing_monitors_for_service(pool, monitor.service_id, monitor.id)
                .await?;

        if !others_failing {
            tracing::info!(
                monitor_id = %monitor.id,
                service_id = %monitor.service_id,
                "Service recovered, setting to operational"
            );
            db::update_service_status(pool, monitor.service_id, ServiceStatus::Operational).await?;

            // Auto-resolve any auto-incidents for this service
            let resolution = db::resolve_auto_incident(pool, monitor.service_id).await?;

            publish_service_status(
                pool,
                publisher,
                monitor.org_id,
                service.service_id,
                service.service_name.clone(),
                service.current_status,
                ServiceStatus::Operational,
            )
            .await;

            if let Some(resolution) = resolution {
                publish_incident_updated(
                    pool,
                    publisher,
                    monitor.org_id,
                    resolution.incident_id,
                    resolution.update.id,
                    resolution.update.status,
                    resolution.update.message,
                )
                .await;
            }
        }
    }

    Ok(())
}

async fn handle_failure(
    pool: &PgPool,
    monitor: &Monitor,
    result: &CheckResult,
    publisher: Option<&RedisPublisher>,
) -> anyhow::Result<()> {
    // Optimistic increment of consecutive failures
    let incremented =
        db::increment_failures(pool, monitor.id, monitor.consecutive_failures).await?;

    if !incremented {
        // Another concurrent check already incremented — skip evaluation
        tracing::debug!(
            monitor_id = %monitor.id,
            "Concurrent failure increment detected, skipping evaluation"
        );
        return Ok(());
    }

    let new_consecutive = monitor.consecutive_failures + 1;

    // Check if we've hit the threshold
    if new_consecutive >= monitor.failure_threshold {
        let service = db::get_service_snapshot(pool, monitor.service_id).await?;

        if service.current_status == ServiceStatus::Operational
            || service.current_status == ServiceStatus::DegradedPerformance
        {
            tracing::warn!(
                monitor_id = %monitor.id,
                service_id = %monitor.service_id,
                consecutive_failures = new_consecutive,
                "Failure threshold reached, setting service to major outage"
            );

            db::update_service_status(pool, monitor.service_id, ServiceStatus::MajorOutage).await?;

            publish_service_status(
                pool,
                publisher,
                monitor.org_id,
                service.service_id,
                service.service_name.clone(),
                service.current_status,
                ServiceStatus::MajorOutage,
            )
            .await;

            // Create auto-incident if one doesn't already exist
            let has_incident = db::has_active_auto_incident(pool, monitor.service_id).await?;
            if !has_incident {
                let error_msg = result
                    .error_message
                    .as_deref()
                    .unwrap_or("Monitor check failed");
                let incident =
                    db::create_auto_incident(pool, monitor.org_id, monitor.service_id, error_msg)
                        .await?;

                publish_incident_created(pool, publisher, monitor.org_id, incident).await;
            }
        }
    }

    Ok(())
}

async fn publish_service_status(
    pool: &PgPool,
    publisher: Option<&RedisPublisher>,
    org_id: uuid::Uuid,
    service_id: uuid::Uuid,
    service_name: String,
    old_status: ServiceStatus,
    new_status: ServiceStatus,
) {
    let payload = serde_json::json!({
        "event_type": "service.status_changed",
        "org_id": org_id,
        "occurred_at": chrono::Utc::now(),
        "data": {
            "service_id": service_id,
            "service_name": service_name.clone(),
            "old_status": old_status,
            "new_status": new_status,
        }
    });
    if let Err(error) =
        db::enqueue_webhook_deliveries(pool, org_id, "service.status_changed", &payload).await
    {
        tracing::warn!(
            error = %error,
            org_id = %org_id,
            service_id = %service_id,
            "Failed to queue service status webhook deliveries from monitor worker"
        );
    }

    if let Err(error) = db::enqueue_service_status_notification_emails(
        pool,
        org_id,
        &service_name,
        old_status,
        new_status,
    )
    .await
    {
        tracing::warn!(
            error = %error,
            org_id = %org_id,
            service_id = %service_id,
            "Failed to queue service status subscriber emails from monitor worker"
        );
    }

    if let Some(publisher) = publisher {
        let event =
            RedisPublisher::service_status_event(service_id, service_name, old_status, new_status);
        if let Err(error) = publisher.publish_service_status_change(org_id, event).await {
            tracing::warn!(
                error = %error,
                org_id = %org_id,
                service_id = %service_id,
                "Failed to publish service status event from monitor worker"
            );
        }
    }
}

async fn publish_incident_created(
    pool: &PgPool,
    publisher: Option<&RedisPublisher>,
    org_id: uuid::Uuid,
    incident: db::AutoIncidentCreated,
) {
    let payload = serde_json::json!({
        "event_type": "incident.created",
        "org_id": org_id,
        "occurred_at": chrono::Utc::now(),
        "data": {
            "incident_id": incident.incident_id,
            "title": incident.title.clone(),
            "status": incident.status,
            "impact": incident.impact.as_str(),
            "affected_services": incident.affected_services.clone(),
        }
    });
    if let Err(error) =
        db::enqueue_webhook_deliveries(pool, org_id, "incident.created", &payload).await
    {
        tracing::warn!(
            error = %error,
            org_id = %org_id,
            incident_id = %incident.incident_id,
            "Failed to queue incident created webhook deliveries from monitor worker"
        );
    }

    if let Err(error) = db::enqueue_incident_notification_emails(
        pool,
        org_id,
        "incident.created",
        &incident.title,
        incident.status,
        "Automated monitoring detected failures.",
    )
    .await
    {
        tracing::warn!(
            error = %error,
            org_id = %org_id,
            incident_id = %incident.incident_id,
            "Failed to queue incident created subscriber emails from monitor worker"
        );
    }

    if let Some(publisher) = publisher {
        let event = RedisPublisher::incident_created_event(
            incident.incident_id,
            incident.title,
            incident.status,
            incident.impact.as_str().to_string(),
            incident.affected_services,
        );
        if let Err(error) = publisher.publish_incident_created(org_id, event).await {
            tracing::warn!(
                error = %error,
                org_id = %org_id,
                incident_id = %incident.incident_id,
                "Failed to publish incident created event from monitor worker"
            );
        }
    }
}

async fn publish_incident_updated(
    pool: &PgPool,
    publisher: Option<&RedisPublisher>,
    org_id: uuid::Uuid,
    incident_id: uuid::Uuid,
    update_id: uuid::Uuid,
    status: shared::enums::IncidentStatus,
    message: String,
) {
    let webhook_event_type = if status == shared::enums::IncidentStatus::Resolved {
        "incident.resolved"
    } else {
        "incident.updated"
    };
    let payload = serde_json::json!({
        "event_type": webhook_event_type,
        "org_id": org_id,
        "occurred_at": chrono::Utc::now(),
        "data": {
            "incident_id": incident_id,
            "update_id": update_id,
            "status": status,
            "message": message.clone(),
        }
    });
    if let Err(error) =
        db::enqueue_webhook_deliveries(pool, org_id, webhook_event_type, &payload).await
    {
        tracing::warn!(
            error = %error,
            org_id = %org_id,
            incident_id = %incident_id,
            "Failed to queue incident webhook deliveries from monitor worker"
        );
    }

    if let Err(error) = db::enqueue_incident_notification_emails(
        pool,
        org_id,
        webhook_event_type,
        "Automated incident update",
        status,
        &message,
    )
    .await
    {
        tracing::warn!(
            error = %error,
            org_id = %org_id,
            incident_id = %incident_id,
            "Failed to queue incident update subscriber emails from monitor worker"
        );
    }

    if let Some(publisher) = publisher {
        let event = RedisPublisher::incident_updated_event(incident_id, update_id, status, message);
        if let Err(error) = publisher.publish_incident_updated(org_id, event).await {
            tracing::warn!(
                error = %error,
                org_id = %org_id,
                incident_id = %incident_id,
                "Failed to publish incident updated event from monitor worker"
            );
        }
    }
}
