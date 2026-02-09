use chrono::{Duration, NaiveDate, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use crate::db;

pub async fn run_rollup(pool: &PgPool) -> anyhow::Result<()> {
    tracing::info!("Running daily rollup...");

    // Ensure partitions exist
    db::ensure_upcoming_partitions(pool).await?;

    // Get all active monitors
    let monitors = db::get_active_monitors(pool).await?;
    let today = Utc::now().date_naive();

    for monitor in &monitors {
        // Rollup today (partial data)
        if let Err(e) = db::rollup_daily(pool, monitor.id, today).await {
            tracing::error!(
                monitor_id = %monitor.id,
                error = %e,
                "Failed to rollup today's data"
            );
        }

        // Check for missing days (up to 7 days back to catch up)
        for days_back in 1..=7 {
            let date = today - Duration::days(days_back);
            let needs_rollup = check_needs_rollup(pool, monitor.id, date).await;
            if needs_rollup {
                if let Err(e) = db::rollup_daily(pool, monitor.id, date).await {
                    tracing::error!(
                        monitor_id = %monitor.id,
                        date = %date,
                        error = %e,
                        "Failed to rollup missing day"
                    );
                }
            }
        }
    }

    tracing::info!("Daily rollup complete for {} monitors", monitors.len());
    Ok(())
}

async fn check_needs_rollup(pool: &PgPool, monitor_id: Uuid, date: NaiveDate) -> bool {
    let exists: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM uptime_daily WHERE monitor_id = $1 AND date = $2)",
    )
    .bind(monitor_id)
    .bind(date)
    .fetch_one(pool)
    .await
    .unwrap_or(false);

    !exists
}
