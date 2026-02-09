use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use shared::models::monitor::{Monitor, MonitorConfig};
use sqlx::PgPool;
use tokio::sync::Semaphore;
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

use crate::checker;
use crate::config::Config;
use crate::db;
use crate::evaluator;
use crate::rollup;

struct MonitorTask {
    handle: JoinHandle<()>,
    cancel: CancellationToken,
    config_hash: u64,
}

pub struct Scheduler {
    pool: PgPool,
    config: Config,
}

impl Scheduler {
    pub fn new(pool: PgPool, config: Config) -> Self {
        Self { pool, config }
    }

    pub async fn run(&self) -> anyhow::Result<()> {
        let semaphore = Arc::new(Semaphore::new(self.config.max_concurrent_checks));
        let shutdown = CancellationToken::new();
        let mut tasks: HashMap<Uuid, MonitorTask> = HashMap::new();

        // Initial partition setup
        if let Err(e) = db::ensure_upcoming_partitions(&self.pool).await {
            tracing::warn!(error = %e, "Failed to create initial partitions");
        }

        // Initial load
        self.reload_monitors(&mut tasks, &semaphore, &shutdown).await;
        tracing::info!("Scheduler started with {} monitors", tasks.len());

        let reload_interval = Duration::from_secs(self.config.config_reload_interval_secs);
        let mut reload_timer = tokio::time::interval(reload_interval);
        reload_timer.tick().await; // Skip the first immediate tick

        let mut rollup_timer = tokio::time::interval(Duration::from_secs(3600));
        rollup_timer.tick().await;

        loop {
            tokio::select! {
                _ = reload_timer.tick() => {
                    self.reload_monitors(&mut tasks, &semaphore, &shutdown).await;
                }
                _ = rollup_timer.tick() => {
                    let pool = self.pool.clone();
                    tokio::spawn(async move {
                        if let Err(e) = rollup::run_rollup(&pool).await {
                            tracing::error!(error = %e, "Rollup failed");
                        }
                    });
                }
                _ = shutdown_signal() => {
                    tracing::info!("Shutdown signal received, stopping all monitors...");
                    shutdown.cancel();

                    // Wait for all tasks with a timeout
                    let mut handles: Vec<JoinHandle<()>> = Vec::new();
                    for (_, task) in tasks.drain() {
                        task.cancel.cancel();
                        handles.push(task.handle);
                    }

                    let _ = tokio::time::timeout(
                        Duration::from_secs(10),
                        futures_join_all(handles),
                    ).await;

                    tracing::info!("All monitors stopped");
                    break;
                }
            }
        }

        Ok(())
    }

    async fn reload_monitors(
        &self,
        tasks: &mut HashMap<Uuid, MonitorTask>,
        semaphore: &Arc<Semaphore>,
        _shutdown: &CancellationToken,
    ) {
        let monitors = match db::get_active_monitors(&self.pool).await {
            Ok(m) => m,
            Err(e) => {
                tracing::error!(error = %e, "Failed to load monitors");
                return;
            }
        };

        let active_ids: std::collections::HashSet<Uuid> =
            monitors.iter().map(|m| m.id).collect();

        // Remove tasks for monitors that no longer exist or are disabled
        let to_remove: Vec<Uuid> = tasks
            .keys()
            .filter(|id| !active_ids.contains(id))
            .cloned()
            .collect();

        for id in to_remove {
            if let Some(task) = tasks.remove(&id) {
                tracing::info!(monitor_id = %id, "Stopping removed/disabled monitor");
                task.cancel.cancel();
            }
        }

        // Add or update monitors
        for monitor in monitors {
            let hash = config_hash(&monitor);

            if let Some(existing) = tasks.get(&monitor.id) {
                if existing.config_hash == hash {
                    continue; // No change
                }
                // Config changed — cancel and respawn
                tracing::info!(monitor_id = %monitor.id, "Monitor config changed, respawning");
                existing.cancel.cancel();
                tasks.remove(&monitor.id);
            }

            // Spawn new task
            let cancel = CancellationToken::new();
            let pool = self.pool.clone();
            let sem = semaphore.clone();
            let cancel_clone = cancel.clone();

            let monitor_id = monitor.id;
            let handle = tokio::spawn(async move {
                run_monitor_loop(pool, monitor, sem, cancel_clone).await;
            });

            tasks.insert(
                monitor_id,
                MonitorTask {
                    handle,
                    cancel,
                    config_hash: hash,
                },
            );
        }
    }
}

async fn run_monitor_loop(
    pool: PgPool,
    monitor: Monitor,
    semaphore: Arc<Semaphore>,
    cancel: CancellationToken,
) {
    let config: MonitorConfig = match serde_json::from_value(monitor.config.clone()) {
        Ok(c) => c,
        Err(e) => {
            tracing::error!(
                monitor_id = %monitor.id,
                error = %e,
                "Invalid monitor config, stopping"
            );
            return;
        }
    };

    let check_impl = checker::create_checker(&config);
    let interval = Duration::from_secs(monitor.interval_seconds as u64);
    let timeout = Duration::from_millis(monitor.timeout_ms as u64);

    tracing::info!(
        monitor_id = %monitor.id,
        monitor_type = %monitor.monitor_type,
        interval_secs = monitor.interval_seconds,
        "Monitor task started"
    );

    loop {
        tokio::select! {
            _ = cancel.cancelled() => {
                tracing::info!(monitor_id = %monitor.id, "Monitor task cancelled");
                break;
            }
            _ = tokio::time::sleep(interval) => {
                // Acquire semaphore permit
                let _permit = match semaphore.acquire().await {
                    Ok(p) => p,
                    Err(_) => break, // Semaphore closed
                };

                let result = check_impl.check(timeout).await;

                tracing::debug!(
                    monitor_id = %monitor.id,
                    status = %result.status,
                    response_time_ms = result.response_time_ms,
                    "Check completed"
                );

                // Re-fetch monitor state for evaluator (may have changed)
                let current_monitor = match sqlx::query_as::<_, Monitor>(
                    "SELECT * FROM monitors WHERE id = $1",
                )
                .bind(monitor.id)
                .fetch_optional(&pool)
                .await
                {
                    Ok(Some(m)) => m,
                    Ok(None) => {
                        tracing::info!(monitor_id = %monitor.id, "Monitor deleted, stopping");
                        break;
                    }
                    Err(e) => {
                        tracing::error!(monitor_id = %monitor.id, error = %e, "Failed to fetch monitor");
                        continue;
                    }
                };

                if let Err(e) = evaluator::evaluate(&pool, &current_monitor, &result).await {
                    tracing::error!(
                        monitor_id = %monitor.id,
                        error = %e,
                        "Evaluator failed"
                    );
                }
            }
        }
    }
}

fn config_hash(monitor: &Monitor) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    monitor.config.to_string().hash(&mut hasher);
    monitor.interval_seconds.hash(&mut hasher);
    monitor.timeout_ms.hash(&mut hasher);
    monitor.failure_threshold.hash(&mut hasher);
    monitor.is_active.hash(&mut hasher);
    hasher.finish()
}

async fn futures_join_all(handles: Vec<JoinHandle<()>>) {
    for handle in handles {
        let _ = handle.await;
    }
}

async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("Failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("Failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }
}
