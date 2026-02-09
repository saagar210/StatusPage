use shared::enums::{CheckStatus, ServiceStatus};
use shared::models::monitor::Monitor;
use sqlx::PgPool;

use crate::checker::CheckResult;
use crate::db;

pub async fn evaluate(pool: &PgPool, monitor: &Monitor, result: &CheckResult) -> anyhow::Result<()> {
    // 1. Insert check result
    db::insert_check(pool, monitor.id, result).await?;

    match result.status {
        CheckStatus::Success => handle_success(pool, monitor).await?,
        CheckStatus::Failure | CheckStatus::Timeout => handle_failure(pool, monitor, result).await?,
    }

    Ok(())
}

async fn handle_success(pool: &PgPool, monitor: &Monitor) -> anyhow::Result<()> {
    // Reset consecutive failures
    if monitor.consecutive_failures > 0 {
        db::reset_failures(pool, monitor.id).await?;
    }

    // Check if service is in outage and should recover
    let current_status = db::get_service_current_status(pool, monitor.service_id).await?;
    if current_status != ServiceStatus::Operational && current_status != ServiceStatus::UnderMaintenance {
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
            db::resolve_auto_incident(pool, monitor.service_id).await?;
        }
    }

    Ok(())
}

async fn handle_failure(
    pool: &PgPool,
    monitor: &Monitor,
    result: &CheckResult,
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
        let current_status = db::get_service_current_status(pool, monitor.service_id).await?;

        if current_status == ServiceStatus::Operational
            || current_status == ServiceStatus::DegradedPerformance
        {
            tracing::warn!(
                monitor_id = %monitor.id,
                service_id = %monitor.service_id,
                consecutive_failures = new_consecutive,
                "Failure threshold reached, setting service to major outage"
            );

            db::update_service_status(pool, monitor.service_id, ServiceStatus::MajorOutage)
                .await?;

            // Create auto-incident if one doesn't already exist
            let has_incident = db::has_active_auto_incident(pool, monitor.service_id).await?;
            if !has_incident {
                let error_msg = result
                    .error_message
                    .as_deref()
                    .unwrap_or("Monitor check failed");
                db::create_auto_incident(pool, monitor.org_id, monitor.service_id, error_msg)
                    .await?;
            }
        }
    }

    Ok(())
}
