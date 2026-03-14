use axum::{
    extract::State,
    http::{HeaderMap, StatusCode},
    routing::post,
    Router,
};
use shared::enums::OrganizationPlan;
use shared::error::AppError;

use crate::db;
use crate::state::AppState;

pub fn router() -> Router<AppState> {
    Router::new().route("/stripe/webhook", post(handle_stripe_webhook))
}

async fn handle_stripe_webhook(
    State(state): State<AppState>,
    headers: HeaderMap,
    body: String,
) -> Result<StatusCode, AppError> {
    let webhook_secret = state
        .config
        .stripe_webhook_secret
        .as_deref()
        .ok_or_else(|| {
            AppError::Validation("Stripe webhook handling is not configured".to_string())
        })?;
    let signature = headers
        .get("stripe-signature")
        .and_then(|value| value.to_str().ok())
        .ok_or_else(|| AppError::Validation("Missing Stripe-Signature header".to_string()))?;

    crate::services::billing::verify_stripe_webhook_signature(webhook_secret, signature, &body)?;

    let Some(event) = crate::services::billing::parse_stripe_webhook(&body, &state.config)? else {
        return Ok(StatusCode::NO_CONTENT);
    };

    match event {
        crate::services::billing::ParsedStripeWebhook::CheckoutCompleted(event) => {
            if db::billing_events::exists(&state.pool, &event.event_id).await? {
                return Ok(StatusCode::NO_CONTENT);
            }

            if let Some(org_id) = event.org_id {
                db::organizations::sync_checkout_session(
                    &state.pool,
                    org_id,
                    event.customer_id.as_deref(),
                    event.subscription_id.as_deref(),
                    event.billing_email.as_deref(),
                )
                .await?;
                db::audit_logs::record(
                    &state.pool,
                    db::audit_logs::NewAuditLog {
                        org_id,
                        actor_user_id: None,
                        actor_type: "system",
                        action: "billing.checkout.completed",
                        target_type: "organization",
                        target_id: Some(&org_id.to_string()),
                        details: serde_json::json!({
                            "stripe_event_id": event.event_id.clone(),
                            "customer_id": event.customer_id.clone(),
                            "subscription_id": event.subscription_id.clone(),
                            "billing_email": event.billing_email.clone(),
                        }),
                    },
                )
                .await?;
            }

            db::billing_events::record(
                &state.pool,
                &event.event_id,
                &event.event_type,
                event.org_id,
                &event.payload,
            )
            .await?;
        }
        crate::services::billing::ParsedStripeWebhook::SubscriptionUpdated(event) => {
            if db::billing_events::exists(&state.pool, &event.event_id).await? {
                return Ok(StatusCode::NO_CONTENT);
            }

            let org = match resolve_billing_org(&state, event.org_id, event.customer_id.as_deref())
                .await?
            {
                Some(org) => org,
                None => {
                    tracing::warn!(
                        stripe_event_id = %event.event_id,
                        stripe_event_type = %event.event_type,
                        "Ignoring Stripe subscription event because no organization matched"
                    );
                    db::billing_events::record(
                        &state.pool,
                        &event.event_id,
                        &event.event_type,
                        None,
                        &event.payload,
                    )
                    .await?;
                    return Ok(StatusCode::NO_CONTENT);
                }
            };

            let update = db::organizations::BillingSyncUpdate {
                stripe_customer_id: event.customer_id.as_deref(),
                stripe_subscription_id: event.subscription_id.as_deref(),
                subscription_status: event.subscription_status,
                stripe_price_id: event.stripe_price_id.as_deref(),
                current_period_end: event.current_period_end,
                cancel_at_period_end: event.cancel_at_period_end,
                billing_email: event.billing_email.as_deref(),
                trial_ends_at: event.trial_ends_at,
                plan: org.plan,
            };
            let lifecycle = crate::services::downgrade::downgrade_lifecycle_for_plan_change(
                &org,
                event.plan,
                chrono::Utc::now(),
            );

            let effective_plan = match lifecycle.state {
                shared::enums::DowngradeState::PendingCustomerAction
                | shared::enums::DowngradeState::ReadyToEnforce => org.plan,
                _ => event.plan,
            };
            let update = db::organizations::BillingSyncUpdate {
                plan: effective_plan,
                ..update
            };
            let synced_org =
                db::organizations::sync_billing_state(&state.pool, org.id, &update, &lifecycle)
                    .await?;

            if event.plan != OrganizationPlan::Free {
                db::monitors::restore_plan_limited(&state.pool, org.id).await?;
                db::webhooks::restore_plan_limited(&state.pool, org.id).await?;
                if synced_org.custom_domain.is_some()
                    && synced_org.custom_domain_status
                        == shared::enums::CustomDomainStatus::BlockedByPlan
                {
                    db::organizations::set_custom_domain_status(
                        &state.pool,
                        org.id,
                        shared::enums::CustomDomainStatus::PendingVerification,
                    )
                    .await?;
                }
            }
            db::audit_logs::record(
                &state.pool,
                db::audit_logs::NewAuditLog {
                    org_id: org.id,
                    actor_user_id: None,
                    actor_type: "system",
                    action: "billing.subscription.sync",
                    target_type: "organization",
                    target_id: Some(&org.id.to_string()),
                    details: serde_json::json!({
                        "stripe_event_id": event.event_id.clone(),
                        "subscription_status": event.subscription_status,
                        "plan": event.plan,
                        "effective_plan": effective_plan,
                        "downgrade_state": lifecycle.state,
                        "downgrade_target_plan": lifecycle.target_plan,
                        "downgrade_grace_ends_at": lifecycle.grace_ends_at,
                        "cancel_at_period_end": event.cancel_at_period_end,
                    }),
                },
            )
            .await?;

            db::billing_events::record(
                &state.pool,
                &event.event_id,
                &event.event_type,
                Some(org.id),
                &event.payload,
            )
            .await?;
        }
    }

    Ok(StatusCode::NO_CONTENT)
}

async fn resolve_billing_org(
    state: &AppState,
    org_id: Option<uuid::Uuid>,
    customer_id: Option<&str>,
) -> Result<Option<shared::models::organization::Organization>, AppError> {
    if let Some(org_id) = org_id {
        if let Some(org) = db::organizations::find_by_id(&state.pool, org_id).await? {
            return Ok(Some(org));
        }
    }

    if let Some(customer_id) = customer_id {
        return db::organizations::find_by_stripe_customer_id(&state.pool, customer_id).await;
    }

    Ok(None)
}
