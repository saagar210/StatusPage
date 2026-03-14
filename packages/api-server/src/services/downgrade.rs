use std::time::Duration;

use chrono::{DateTime, Utc};
use shared::enums::{CustomDomainStatus, DowngradeState, OrganizationPlan};
use shared::error::AppError;
use shared::models::organization::{EntitlementViolation, Organization};
use sqlx::PgPool;

use crate::config::Config;
use crate::db;

const DOWNGRADE_GRACE_DAYS: i64 = 14;

#[derive(Debug, Clone)]
pub struct EnforcementResult {
    pub disabled_monitor_ids: Vec<uuid::Uuid>,
    pub blocked_custom_domain: bool,
    pub disabled_webhooks: bool,
}

pub fn downgrade_lifecycle_for_plan_change(
    org: &Organization,
    incoming_plan: OrganizationPlan,
    now: DateTime<Utc>,
) -> db::organizations::DowngradeLifecycle {
    if plan_rank(incoming_plan) < plan_rank(org.plan) {
        let continuing_same_target = org.downgrade_target_plan == Some(incoming_plan)
            && matches!(
                org.downgrade_state,
                DowngradeState::PendingCustomerAction | DowngradeState::ReadyToEnforce
            );
        let started_at = if continuing_same_target {
            org.downgrade_started_at.unwrap_or(now)
        } else {
            now
        };
        let grace_ends_at = if continuing_same_target {
            org.downgrade_grace_ends_at
                .unwrap_or_else(|| started_at + chrono::Duration::days(DOWNGRADE_GRACE_DAYS))
        } else {
            started_at + chrono::Duration::days(DOWNGRADE_GRACE_DAYS)
        };

        return db::organizations::DowngradeLifecycle {
            target_plan: Some(incoming_plan),
            started_at: Some(started_at),
            grace_ends_at: Some(grace_ends_at),
            state: DowngradeState::PendingCustomerAction,
            warning_stage: if continuing_same_target {
                org.downgrade_warning_stage
            } else {
                0
            },
        };
    }

    if org.downgrade_state != DowngradeState::None {
        return db::organizations::DowngradeLifecycle {
            target_plan: None,
            started_at: None,
            grace_ends_at: None,
            state: DowngradeState::Canceled,
            warning_stage: 0,
        };
    }

    db::organizations::DowngradeLifecycle {
        target_plan: None,
        started_at: None,
        grace_ends_at: None,
        state: DowngradeState::None,
        warning_stage: 0,
    }
}

pub async fn entitlement_violations(
    pool: &PgPool,
    org: &Organization,
) -> Result<Vec<EntitlementViolation>, AppError> {
    let target_plan = enforcement_plan(org);
    let active_monitor_count = db::monitors::count_by_org(pool, org.id).await?;
    let enabled_webhook_count = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM webhook_configs WHERE org_id = $1 AND is_enabled = TRUE",
    )
    .bind(org.id)
    .fetch_one(pool)
    .await?;

    let mut violations = Vec::new();

    if let Some(max_monitors) = target_plan.max_monitors() {
        if active_monitor_count > max_monitors {
            violations.push(EntitlementViolation {
                code: "monitor_limit".to_string(),
                message: format!(
                    "This organization has {} active monitors, but the {} plan allows {}.",
                    active_monitor_count, target_plan, max_monitors
                ),
                current_count: Some(active_monitor_count),
                allowed_count: Some(max_monitors),
            });
        }
    }

    if !target_plan.allows_custom_domain()
        && org
            .custom_domain
            .as_ref()
            .is_some_and(|value| !value.trim().is_empty())
    {
        violations.push(EntitlementViolation {
            code: "custom_domain".to_string(),
            message: "Custom domains are not included on the target plan.".to_string(),
            current_count: Some(1),
            allowed_count: Some(0),
        });
    }

    if !target_plan.allows_outbound_webhooks() && enabled_webhook_count > 0 {
        violations.push(EntitlementViolation {
            code: "outbound_webhooks".to_string(),
            message: format!(
                "This organization has {} enabled outbound webhooks, but the target plan does not include them.",
                enabled_webhook_count
            ),
            current_count: Some(enabled_webhook_count),
            allowed_count: Some(0),
        });
    }

    Ok(violations)
}

pub fn required_actions(violations: &[EntitlementViolation]) -> Vec<String> {
    violations
        .iter()
        .map(|violation| match violation.code.as_str() {
            "monitor_limit" => "Reduce active monitors before the grace period ends.".to_string(),
            "custom_domain" => {
                "Remove the custom domain or upgrade back to a paid plan.".to_string()
            }
            "outbound_webhooks" => {
                "Disable outbound webhooks or upgrade back to a paid plan.".to_string()
            }
            _ => violation.message.clone(),
        })
        .collect()
}

pub async fn process_due_work(pool: &PgPool, config: &Config) -> Result<(), AppError> {
    for org in db::organizations::find_orgs_with_active_downgrades(pool).await? {
        maybe_queue_due_warning(pool, config, &org).await?;
    }

    for org in db::organizations::find_due_for_downgrade_enforcement(pool, 20).await? {
        let _ = enforce_now(pool, &org).await?;
    }

    Ok(())
}

pub async fn enforce_now(pool: &PgPool, org: &Organization) -> Result<EnforcementResult, AppError> {
    let target_plan = org
        .downgrade_target_plan
        .ok_or_else(|| AppError::Validation("No downgrade target plan is set".to_string()))?;

    let disabled_monitor_ids = match target_plan.max_monitors() {
        Some(limit) => db::monitors::disable_excess_for_plan(pool, org.id, limit).await?,
        None => {
            db::monitors::restore_plan_limited(pool, org.id).await?;
            Vec::new()
        }
    };

    let blocked_custom_domain = if target_plan.allows_custom_domain() {
        if org.custom_domain.is_some()
            && org.custom_domain_status == CustomDomainStatus::BlockedByPlan
        {
            db::organizations::set_custom_domain_status(
                pool,
                org.id,
                CustomDomainStatus::PendingVerification,
            )
            .await?;
        }
        false
    } else if org.custom_domain.is_some() {
        db::organizations::set_custom_domain_status(
            pool,
            org.id,
            CustomDomainStatus::BlockedByPlan,
        )
        .await?;
        true
    } else {
        false
    };

    let disabled_webhooks = if target_plan.allows_outbound_webhooks() {
        db::webhooks::restore_plan_limited(pool, org.id).await?;
        false
    } else {
        db::webhooks::disable_all_for_org(pool, org.id).await?;
        true
    };

    db::organizations::complete_downgrade_enforcement(pool, org.id, target_plan).await?;

    Ok(EnforcementResult {
        disabled_monitor_ids,
        blocked_custom_domain,
        disabled_webhooks,
    })
}

pub fn spawn(pool: PgPool, config: Config) {
    tokio::spawn(async move {
        let interval = Duration::from_secs(config.downgrade_enforcement_interval_secs.max(15));
        loop {
            if let Err(error) = process_due_work(&pool, &config).await {
                tracing::error!(error = %error, "downgrade processing failed");
            }
            tokio::time::sleep(interval).await;
        }
    });
}

fn plan_rank(plan: OrganizationPlan) -> i32 {
    match plan {
        OrganizationPlan::Free => 0,
        OrganizationPlan::Pro => 1,
        OrganizationPlan::Team => 2,
    }
}

fn enforcement_plan(org: &Organization) -> OrganizationPlan {
    match org.downgrade_state {
        DowngradeState::PendingCustomerAction | DowngradeState::ReadyToEnforce => {
            org.downgrade_target_plan.unwrap_or(org.plan)
        }
        _ => org.plan,
    }
}

async fn maybe_queue_due_warning(
    pool: &PgPool,
    config: &Config,
    org: &Organization,
) -> Result<(), AppError> {
    let Some(started_at) = org.downgrade_started_at else {
        return Ok(());
    };
    let Some(target_plan) = org.downgrade_target_plan else {
        return Ok(());
    };
    let Some(next_stage) = next_warning_stage(org.downgrade_warning_stage, started_at, Utc::now())
    else {
        return Ok(());
    };

    let violations = entitlement_violations(pool, org).await?;
    let actions = required_actions(&violations);
    let stage_label = match next_stage {
        1 => "started",
        2 => "day-7",
        3 => "day-13",
        4 => "enforced",
        _ => "update",
    };

    let recipients = sqlx::query_scalar::<_, String>(
        r#"
        SELECT u.email
        FROM members m
        JOIN users u ON u.id = m.user_id
        WHERE m.org_id = $1
          AND m.role IN ('owner', 'admin')
        ORDER BY u.email
        "#,
    )
    .bind(org.id)
    .fetch_all(pool)
    .await?;

    for recipient in recipients {
        let subject = match next_stage {
            1 => format!("Downgrade started for {}", org.name),
            2 => format!("7-day downgrade reminder for {}", org.name),
            3 => format!("Final downgrade reminder for {}", org.name),
            4 => format!("Downgrade enforced for {}", org.name),
            _ => format!("Plan update for {}", org.name),
        };
        let body = format!(
            "Your organization is moving from {} to {}.\n\nStage: {stage_label}\nGrace period ends: {}\n\nRequired actions:\n{}\n\nManage billing:\n{}/dashboard/{}/settings",
            org.plan,
            target_plan,
            org.downgrade_grace_ends_at
                .map(|value| value.to_rfc3339())
                .unwrap_or_else(|| "not scheduled".to_string()),
            if actions.is_empty() {
                "- No action required.".to_string()
            } else {
                actions.iter().map(|item| format!("- {item}")).collect::<Vec<_>>().join("\n")
            },
            config.app_base_url.trim_end_matches('/'),
            org.slug
        );
        db::notification_logs::enqueue(
            pool,
            org.id,
            "downgrade_warning",
            "billing_admin",
            &recipient,
            &subject,
            &body,
        )
        .await?;
    }

    db::organizations::mark_downgrade_warning_stage(pool, org.id, next_stage).await?;
    Ok(())
}

fn next_warning_stage(
    current_stage: i32,
    started_at: DateTime<Utc>,
    now: DateTime<Utc>,
) -> Option<i32> {
    if current_stage < 1 && now >= started_at {
        return Some(1);
    }
    if current_stage < 2 && now >= started_at + chrono::Duration::days(7) {
        return Some(2);
    }
    if current_stage < 3 && now >= started_at + chrono::Duration::days(13) {
        return Some(3);
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use shared::enums::{CustomDomainStatus, SubscriptionStatus};

    fn org(plan: OrganizationPlan) -> Organization {
        Organization {
            id: uuid::Uuid::nil(),
            name: "Demo".to_string(),
            slug: "demo".to_string(),
            plan,
            logo_url: None,
            brand_color: "#000000".to_string(),
            timezone: "UTC".to_string(),
            custom_domain: None,
            custom_domain_verified_at: None,
            custom_domain_status: CustomDomainStatus::NotConfigured,
            stripe_customer_id: None,
            stripe_subscription_id: None,
            subscription_status: SubscriptionStatus::Active,
            stripe_price_id: None,
            current_period_end: None,
            cancel_at_period_end: false,
            billing_email: None,
            trial_ends_at: None,
            downgrade_target_plan: None,
            downgrade_started_at: None,
            downgrade_grace_ends_at: None,
            downgrade_state: DowngradeState::None,
            downgrade_warning_stage: 0,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    #[test]
    fn downgrade_plan_change_enters_grace_period() {
        let lifecycle = downgrade_lifecycle_for_plan_change(
            &org(OrganizationPlan::Pro),
            OrganizationPlan::Free,
            Utc::now(),
        );

        assert_eq!(lifecycle.target_plan, Some(OrganizationPlan::Free));
        assert_eq!(lifecycle.state, DowngradeState::PendingCustomerAction);
        assert_eq!(lifecycle.warning_stage, 0);
        assert!(lifecycle.grace_ends_at.is_some());
    }

    #[test]
    fn upgrade_cancels_existing_downgrade() {
        let mut current = org(OrganizationPlan::Free);
        current.downgrade_target_plan = Some(OrganizationPlan::Free);
        current.downgrade_started_at = Some(Utc::now());
        current.downgrade_grace_ends_at = Some(Utc::now());
        current.downgrade_state = DowngradeState::PendingCustomerAction;
        current.downgrade_warning_stage = 2;

        let lifecycle =
            downgrade_lifecycle_for_plan_change(&current, OrganizationPlan::Team, Utc::now());

        assert_eq!(lifecycle.state, DowngradeState::Canceled);
        assert!(lifecycle.target_plan.is_none());
    }

    #[test]
    fn warning_stage_advances_by_day_thresholds() {
        let started = Utc::now() - chrono::Duration::days(8);

        assert_eq!(next_warning_stage(0, started, Utc::now()), Some(1));
        assert_eq!(next_warning_stage(1, started, Utc::now()), Some(2));
        assert_eq!(
            next_warning_stage(2, started - chrono::Duration::days(6), Utc::now()),
            Some(3)
        );
    }

    #[test]
    fn required_actions_map_violation_codes() {
        let actions = required_actions(&[
            EntitlementViolation {
                code: "monitor_limit".to_string(),
                message: "x".to_string(),
                current_count: Some(5),
                allowed_count: Some(3),
            },
            EntitlementViolation {
                code: "custom_domain".to_string(),
                message: "x".to_string(),
                current_count: Some(1),
                allowed_count: Some(0),
            },
        ]);

        assert_eq!(actions.len(), 2);
        assert!(actions[0].contains("Reduce active monitors"));
        assert!(actions[1].contains("Remove the custom domain"));
    }
}
