use axum::{
    extract::{Path, Query, State},
    http::HeaderMap,
    routing::{get, post},
    Json, Router,
};
use chrono::Utc;
use serde::Serialize;
use uuid::Uuid;

use shared::error::AppError;
use shared::models::invitation::InvitationWithInviter;
use shared::models::organization::EntitlementViolation;

use crate::db;
use crate::state::AppState;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/queue-health", get(queue_health))
        .route("/organizations/search", get(search_organizations))
        .route("/organizations/{slug}/support", get(organization_support))
        .route(
            "/organizations/{slug}/billing/sync",
            post(sync_billing_state),
        )
        .route(
            "/organizations/{slug}/downgrade/enforce",
            post(enforce_downgrade),
        )
        .route(
            "/organizations/{slug}/downgrade/cancel",
            post(cancel_downgrade),
        )
        .route(
            "/organizations/{slug}/invitations/{id}/resend",
            post(resend_invitation_email),
        )
        .route(
            "/organizations/{slug}/retry/email/{id}",
            post(retry_email_delivery),
        )
        .route(
            "/organizations/{slug}/retry/webhook/{id}",
            post(retry_webhook_delivery),
        )
}

#[derive(Serialize)]
struct DataResponse<T: Serialize> {
    data: T,
}

#[derive(Serialize)]
struct QueueHealthResponse {
    pending_email_deliveries: i64,
    failed_email_deliveries: i64,
    pending_webhook_deliveries: i64,
    failed_webhook_deliveries: i64,
    recent_billing_events: i64,
    pending_invitation_emails: i64,
    pending_downgrade_warnings: i64,
    organizations_in_grace: i64,
}

#[derive(Serialize)]
struct SupportOrganizationResponse {
    organization: SupportOrganization,
    queue_health: OrgQueueHealth,
    entitlement_violations: Vec<EntitlementViolation>,
    required_actions: Vec<String>,
    invitations: Vec<InvitationWithInviter>,
    recent_billing_events: Vec<db::billing_events::BillingEventEntry>,
    failed_email_deliveries: Vec<db::notification_logs::NotificationLogEntry>,
    failed_webhook_deliveries: Vec<db::webhook_deliveries::WebhookDeliveryEntry>,
    recent_audit_logs: Vec<db::audit_logs::AuditLogEntry>,
}

#[derive(Serialize)]
struct SupportOrganization {
    id: Uuid,
    name: String,
    slug: String,
    plan: shared::enums::OrganizationPlan,
    subscription_status: shared::enums::SubscriptionStatus,
    stripe_customer_id: Option<String>,
    stripe_subscription_id: Option<String>,
    billing_email: Option<String>,
    custom_domain: Option<String>,
    custom_domain_verified_at: Option<chrono::DateTime<chrono::Utc>>,
    custom_domain_status: shared::enums::CustomDomainStatus,
    downgrade_target_plan: Option<shared::enums::OrganizationPlan>,
    downgrade_grace_ends_at: Option<chrono::DateTime<chrono::Utc>>,
    downgrade_state: shared::enums::DowngradeState,
    member_count: i64,
    pending_invitation_count: i64,
    subscriber_count: i64,
    webhook_count: i64,
}

#[derive(Serialize)]
struct OrgQueueHealth {
    pending_email_deliveries: i64,
    failed_email_deliveries: i64,
    pending_webhook_deliveries: i64,
    failed_webhook_deliveries: i64,
}

#[derive(Serialize)]
struct MessageResponse {
    message: String,
}

#[derive(Serialize)]
struct SupportSearchResult {
    id: Uuid,
    name: String,
    slug: String,
    plan: shared::enums::OrganizationPlan,
    subscription_status: shared::enums::SubscriptionStatus,
    billing_email: Option<String>,
    stripe_customer_id: Option<String>,
    stripe_subscription_id: Option<String>,
    downgrade_state: shared::enums::DowngradeState,
    downgrade_target_plan: Option<shared::enums::OrganizationPlan>,
    downgrade_grace_ends_at: Option<chrono::DateTime<chrono::Utc>>,
}

#[derive(serde::Deserialize)]
struct SearchParams {
    q: String,
}

async fn queue_health(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<DataResponse<QueueHealthResponse>>, AppError> {
    require_internal_admin(&state, &headers)?;

    let pending_email_deliveries = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM notification_logs WHERE status = 'pending'",
    )
    .fetch_one(&state.pool)
    .await?;
    let failed_email_deliveries = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM notification_logs WHERE status = 'failed'",
    )
    .fetch_one(&state.pool)
    .await?;
    let pending_webhook_deliveries = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM webhook_deliveries WHERE status = 'pending'",
    )
    .fetch_one(&state.pool)
    .await?;
    let failed_webhook_deliveries = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM webhook_deliveries WHERE status = 'failed'",
    )
    .fetch_one(&state.pool)
    .await?;
    let recent_billing_events = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM billing_events WHERE processed_at > NOW() - INTERVAL '24 hours'",
    )
    .fetch_one(&state.pool)
    .await?;
    let pending_invitation_emails = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM notification_logs WHERE notification_type = 'invitation_email' AND status = 'pending'",
    )
    .fetch_one(&state.pool)
    .await?;
    let pending_downgrade_warnings = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM notification_logs WHERE notification_type = 'downgrade_warning' AND status = 'pending'",
    )
    .fetch_one(&state.pool)
    .await?;
    let organizations_in_grace = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM organizations WHERE downgrade_state = 'pending_customer_action'",
    )
    .fetch_one(&state.pool)
    .await?;

    Ok(Json(DataResponse {
        data: QueueHealthResponse {
            pending_email_deliveries,
            failed_email_deliveries,
            pending_webhook_deliveries,
            failed_webhook_deliveries,
            recent_billing_events,
            pending_invitation_emails,
            pending_downgrade_warnings,
            organizations_in_grace,
        },
    }))
}

async fn search_organizations(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(params): Query<SearchParams>,
) -> Result<Json<DataResponse<Vec<SupportSearchResult>>>, AppError> {
    require_internal_admin(&state, &headers)?;

    let results = db::organizations::search_for_support(&state.pool, &params.q, 15)
        .await?
        .into_iter()
        .map(|org| SupportSearchResult {
            id: org.id,
            name: org.name,
            slug: org.slug,
            plan: org.plan,
            subscription_status: org.subscription_status,
            billing_email: org.billing_email,
            stripe_customer_id: org.stripe_customer_id,
            stripe_subscription_id: org.stripe_subscription_id,
            downgrade_state: org.downgrade_state,
            downgrade_target_plan: org.downgrade_target_plan,
            downgrade_grace_ends_at: org.downgrade_grace_ends_at,
        })
        .collect();

    Ok(Json(DataResponse { data: results }))
}

async fn organization_support(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(slug): Path<String>,
) -> Result<Json<DataResponse<SupportOrganizationResponse>>, AppError> {
    require_internal_admin(&state, &headers)?;

    let org = db::organizations::find_by_slug(&state.pool, &slug)
        .await?
        .ok_or_else(|| AppError::NotFound("Organization not found".to_string()))?;

    let member_count =
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM members WHERE org_id = $1")
            .bind(org.id)
            .fetch_one(&state.pool)
            .await?;
    let pending_invitation_count = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM invitations WHERE org_id = $1 AND accepted_at IS NULL AND expires_at > NOW()",
    )
    .bind(org.id)
    .fetch_one(&state.pool)
    .await?;
    let subscriber_count =
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM subscribers WHERE org_id = $1")
            .bind(org.id)
            .fetch_one(&state.pool)
            .await?;
    let webhook_count =
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM webhook_configs WHERE org_id = $1")
            .bind(org.id)
            .fetch_one(&state.pool)
            .await?;
    let pending_email_deliveries = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM notification_logs WHERE org_id = $1 AND status = 'pending'",
    )
    .bind(org.id)
    .fetch_one(&state.pool)
    .await?;
    let failed_email_delivery_count = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM notification_logs WHERE org_id = $1 AND status = 'failed'",
    )
    .bind(org.id)
    .fetch_one(&state.pool)
    .await?;
    let pending_webhook_deliveries = sqlx::query_scalar::<_, i64>(
        r#"
        SELECT COUNT(*)
        FROM webhook_deliveries wd
        JOIN webhook_configs wc ON wc.id = wd.webhook_config_id
        WHERE wc.org_id = $1 AND wd.status = 'pending'
        "#,
    )
    .bind(org.id)
    .fetch_one(&state.pool)
    .await?;
    let failed_webhook_delivery_count = sqlx::query_scalar::<_, i64>(
        r#"
        SELECT COUNT(*)
        FROM webhook_deliveries wd
        JOIN webhook_configs wc ON wc.id = wd.webhook_config_id
        WHERE wc.org_id = $1 AND wd.status = 'failed'
        "#,
    )
    .bind(org.id)
    .fetch_one(&state.pool)
    .await?;
    let recent_billing_events =
        db::billing_events::list_recent_by_org(&state.pool, org.id, 10).await?;
    let invitations = db::invitations::list_by_org(&state.pool, org.id).await?;
    let (failed_email_deliveries, _) =
        db::notification_logs::list_recent_by_org(&state.pool, org.id, 1, 5, Some("failed"))
            .await?;
    let (failed_webhook_deliveries, _) =
        db::webhook_deliveries::list_recent_by_org(&state.pool, org.id, 1, 5, Some("failed"))
            .await?;
    let recent_audit_logs = db::audit_logs::list_recent_by_org(&state.pool, org.id, 10).await?;
    let entitlement_violations =
        crate::services::downgrade::entitlement_violations(&state.pool, &org).await?;
    let required_actions = crate::services::downgrade::required_actions(&entitlement_violations);

    Ok(Json(DataResponse {
        data: SupportOrganizationResponse {
            organization: SupportOrganization {
                id: org.id,
                name: org.name,
                slug: org.slug,
                plan: org.plan,
                subscription_status: org.subscription_status,
                stripe_customer_id: org.stripe_customer_id,
                stripe_subscription_id: org.stripe_subscription_id,
                billing_email: org.billing_email,
                custom_domain: org.custom_domain,
                custom_domain_verified_at: org.custom_domain_verified_at,
                custom_domain_status: org.custom_domain_status,
                downgrade_target_plan: org.downgrade_target_plan,
                downgrade_grace_ends_at: org.downgrade_grace_ends_at,
                downgrade_state: org.downgrade_state,
                member_count,
                pending_invitation_count,
                subscriber_count,
                webhook_count,
            },
            queue_health: OrgQueueHealth {
                pending_email_deliveries,
                failed_email_deliveries: failed_email_delivery_count,
                pending_webhook_deliveries,
                failed_webhook_deliveries: failed_webhook_delivery_count,
            },
            entitlement_violations,
            required_actions,
            invitations,
            recent_billing_events,
            failed_email_deliveries,
            failed_webhook_deliveries,
            recent_audit_logs,
        },
    }))
}

async fn sync_billing_state(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(slug): Path<String>,
) -> Result<Json<DataResponse<MessageResponse>>, AppError> {
    require_internal_admin(&state, &headers)?;

    let org = db::organizations::find_by_slug(&state.pool, &slug)
        .await?
        .ok_or_else(|| AppError::NotFound("Organization not found".to_string()))?;
    let snapshot =
        crate::services::billing::fetch_subscription_snapshot(&state.config, &org).await?;

    apply_billing_snapshot(&state, &org, snapshot).await?;

    db::audit_logs::record(
        &state.pool,
        db::audit_logs::NewAuditLog {
            org_id: org.id,
            actor_user_id: None,
            actor_type: "internal_admin",
            action: "billing.sync.manual",
            target_type: "organization",
            target_id: Some(&org.id.to_string()),
            details: serde_json::json!({ "slug": org.slug }),
        },
    )
    .await?;

    Ok(Json(DataResponse {
        data: MessageResponse {
            message: "Stripe billing state synced from the live subscription.".to_string(),
        },
    }))
}

async fn enforce_downgrade(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(slug): Path<String>,
) -> Result<Json<DataResponse<MessageResponse>>, AppError> {
    require_internal_admin(&state, &headers)?;

    let org = db::organizations::find_by_slug(&state.pool, &slug)
        .await?
        .ok_or_else(|| AppError::NotFound("Organization not found".to_string()))?;
    let result = crate::services::downgrade::enforce_now(&state.pool, &org).await?;
    db::audit_logs::record(
        &state.pool,
        db::audit_logs::NewAuditLog {
            org_id: org.id,
            actor_user_id: None,
            actor_type: "internal_admin",
            action: "downgrade.enforce.manual",
            target_type: "organization",
            target_id: Some(&org.id.to_string()),
            details: serde_json::json!({
                "disabled_monitor_count": result.disabled_monitor_ids.len(),
                "blocked_custom_domain": result.blocked_custom_domain,
                "disabled_webhooks": result.disabled_webhooks,
            }),
        },
    )
    .await?;

    Ok(Json(DataResponse {
        data: MessageResponse {
            message: "Downgrade enforcement completed.".to_string(),
        },
    }))
}

async fn cancel_downgrade(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(slug): Path<String>,
) -> Result<Json<DataResponse<MessageResponse>>, AppError> {
    require_internal_admin(&state, &headers)?;

    let org = db::organizations::find_by_slug(&state.pool, &slug)
        .await?
        .ok_or_else(|| AppError::NotFound("Organization not found".to_string()))?;
    db::organizations::cancel_downgrade(&state.pool, org.id, org.plan).await?;
    db::monitors::restore_plan_limited(&state.pool, org.id).await?;
    db::webhooks::restore_plan_limited(&state.pool, org.id).await?;
    if org.custom_domain.is_some()
        && org.custom_domain_status == shared::enums::CustomDomainStatus::BlockedByPlan
    {
        db::organizations::set_custom_domain_status(
            &state.pool,
            org.id,
            shared::enums::CustomDomainStatus::PendingVerification,
        )
        .await?;
    }
    db::audit_logs::record(
        &state.pool,
        db::audit_logs::NewAuditLog {
            org_id: org.id,
            actor_user_id: None,
            actor_type: "internal_admin",
            action: "downgrade.cancel.manual",
            target_type: "organization",
            target_id: Some(&org.id.to_string()),
            details: serde_json::json!({ "slug": org.slug }),
        },
    )
    .await?;

    Ok(Json(DataResponse {
        data: MessageResponse {
            message: "Pending downgrade canceled and plan-limited features restored.".to_string(),
        },
    }))
}

async fn resend_invitation_email(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((slug, invitation_id)): Path<(String, Uuid)>,
) -> Result<Json<DataResponse<MessageResponse>>, AppError> {
    require_internal_admin(&state, &headers)?;

    let org = db::organizations::find_by_slug(&state.pool, &slug)
        .await?
        .ok_or_else(|| AppError::NotFound("Organization not found".to_string()))?;
    let invitation = db::invitations::find_by_id(&state.pool, org.id, invitation_id)
        .await?
        .ok_or_else(|| AppError::NotFound("Invitation not found".to_string()))?;

    if invitation.accepted_at.is_some()
        || invitation.canceled_at.is_some()
        || invitation.expires_at <= Utc::now()
    {
        return Err(AppError::Validation(
            "Only pending invitations can be resent".to_string(),
        ));
    }

    crate::services::email_notifications::queue_invitation_email(
        &state.pool,
        org.id,
        &state.config.app_base_url,
        &org.name,
        invitation.id,
        &invitation.email,
        invitation.role,
        &invitation.token,
    )
    .await?;
    db::invitations::touch_last_sent_at(&state.pool, invitation.id).await?;
    db::audit_logs::record(
        &state.pool,
        db::audit_logs::NewAuditLog {
            org_id: org.id,
            actor_user_id: None,
            actor_type: "internal_admin",
            action: "invitation.resend",
            target_type: "invitation",
            target_id: Some(&invitation.id.to_string()),
            details: serde_json::json!({ "email": invitation.email }),
        },
    )
    .await?;

    Ok(Json(DataResponse {
        data: MessageResponse {
            message: "Queued another invitation email delivery.".to_string(),
        },
    }))
}

async fn retry_email_delivery(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((slug, id)): Path<(String, Uuid)>,
) -> Result<Json<DataResponse<MessageResponse>>, AppError> {
    require_internal_admin(&state, &headers)?;

    let org = db::organizations::find_by_slug(&state.pool, &slug)
        .await?
        .ok_or_else(|| AppError::NotFound("Organization not found".to_string()))?;
    let entry = db::notification_logs::retry_failed_by_id(&state.pool, org.id, id)
        .await?
        .ok_or_else(|| {
            AppError::Validation("Only failed email deliveries can be retried".to_string())
        })?;
    db::audit_logs::record(
        &state.pool,
        db::audit_logs::NewAuditLog {
            org_id: org.id,
            actor_user_id: None,
            actor_type: "internal_admin",
            action: "delivery.email.retry",
            target_type: "notification_log",
            target_id: Some(&entry.id.to_string()),
            details: serde_json::json!({
                "recipient_email": entry.recipient_email,
            }),
        },
    )
    .await?;

    Ok(Json(DataResponse {
        data: MessageResponse {
            message: format!(
                "Queued another email delivery attempt for {}.",
                entry.recipient_email
            ),
        },
    }))
}

async fn retry_webhook_delivery(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((slug, id)): Path<(String, Uuid)>,
) -> Result<Json<DataResponse<MessageResponse>>, AppError> {
    require_internal_admin(&state, &headers)?;

    let org = db::organizations::find_by_slug(&state.pool, &slug)
        .await?
        .ok_or_else(|| AppError::NotFound("Organization not found".to_string()))?;
    let entry = db::webhook_deliveries::retry_failed_by_id(&state.pool, org.id, id)
        .await?
        .ok_or_else(|| {
            AppError::Validation("Only failed webhook deliveries can be retried".to_string())
        })?;
    db::audit_logs::record(
        &state.pool,
        db::audit_logs::NewAuditLog {
            org_id: org.id,
            actor_user_id: None,
            actor_type: "internal_admin",
            action: "delivery.webhook.retry",
            target_type: "webhook_delivery",
            target_id: Some(&entry.id.to_string()),
            details: serde_json::json!({
                "webhook_name": entry.webhook_name,
            }),
        },
    )
    .await?;

    Ok(Json(DataResponse {
        data: MessageResponse {
            message: format!(
                "Queued another webhook delivery attempt for {}.",
                entry.webhook_name
            ),
        },
    }))
}

fn require_internal_admin(state: &AppState, headers: &HeaderMap) -> Result<(), AppError> {
    let provided = headers
        .get("x-statuspage-admin-token")
        .and_then(|value| value.to_str().ok())
        .map(str::trim);

    match internal_admin_auth_result(state.config.internal_admin_token.as_deref(), provided) {
        InternalAdminAuthResult::Authorized => Ok(()),
        InternalAdminAuthResult::MissingConfiguration => Err(AppError::Forbidden(
            "Internal admin tooling is not configured".to_string(),
        )),
        InternalAdminAuthResult::Unauthorized => Err(AppError::Unauthorized),
    }
}

async fn apply_billing_snapshot(
    state: &AppState,
    org: &shared::models::organization::Organization,
    event: crate::services::billing::SubscriptionUpdatedEvent,
) -> Result<(), AppError> {
    let lifecycle = crate::services::downgrade::downgrade_lifecycle_for_plan_change(
        org,
        event.plan,
        Utc::now(),
    );
    let effective_plan = match lifecycle.state {
        shared::enums::DowngradeState::PendingCustomerAction
        | shared::enums::DowngradeState::ReadyToEnforce => org.plan,
        _ => event.plan,
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
        plan: effective_plan,
    };
    let synced_org =
        db::organizations::sync_billing_state(&state.pool, org.id, &update, &lifecycle).await?;

    if event.plan != shared::enums::OrganizationPlan::Free {
        db::monitors::restore_plan_limited(&state.pool, org.id).await?;
        db::webhooks::restore_plan_limited(&state.pool, org.id).await?;
        if synced_org.custom_domain.is_some()
            && synced_org.custom_domain_status == shared::enums::CustomDomainStatus::BlockedByPlan
        {
            db::organizations::set_custom_domain_status(
                &state.pool,
                org.id,
                shared::enums::CustomDomainStatus::PendingVerification,
            )
            .await?;
        }
    }

    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum InternalAdminAuthResult {
    Authorized,
    MissingConfiguration,
    Unauthorized,
}

fn internal_admin_auth_result(
    configured: Option<&str>,
    provided: Option<&str>,
) -> InternalAdminAuthResult {
    match (configured.map(str::trim), provided) {
        (Some(configured), Some(provided)) if !configured.is_empty() && provided == configured => {
            InternalAdminAuthResult::Authorized
        }
        (Some(configured), _) if !configured.is_empty() => InternalAdminAuthResult::Unauthorized,
        _ => InternalAdminAuthResult::MissingConfiguration,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn internal_admin_auth_accepts_matching_token() {
        let result = internal_admin_auth_result(Some("secret-token"), Some("secret-token"));
        assert_eq!(result, InternalAdminAuthResult::Authorized);
    }

    #[test]
    fn internal_admin_auth_rejects_mismatched_token() {
        let result = internal_admin_auth_result(Some("secret-token"), Some("wrong-token"));
        assert_eq!(result, InternalAdminAuthResult::Unauthorized);
    }

    #[test]
    fn internal_admin_auth_requires_configuration() {
        let result = internal_admin_auth_result(None, Some("secret-token"));
        assert_eq!(result, InternalAdminAuthResult::MissingConfiguration);
    }
}
