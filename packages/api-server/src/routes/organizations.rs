use axum::{
    extract::{Path, State},
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use shared::enums::{MemberRole, OrganizationPlan};
use shared::error::AppError;
use shared::models::invitation::{CreateInvitationRequest, InvitationWithInviter};
use shared::models::member::{CreateMemberRequest, MemberWithUser, UpdateMemberRequest};
use shared::models::organization::{
    BillingEntitlements, CreateOrganizationRequest, EntitlementViolation, Organization,
    UpdateOrganizationRequest,
};
use shared::validation::{
    slugify, validate_brand_color, validate_custom_domain, validate_org_name, validate_slug,
    validate_timezone,
};

use crate::db;
use crate::middleware::auth::CurrentUser;
use crate::middleware::org_access::OrgAccess;
use crate::state::AppState;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", post(create_organization).get(list_organizations))
        .route("/{slug}", get(get_organization).patch(update_organization))
        .route("/{slug}/billing", get(get_billing_summary))
        .route("/{slug}/billing/checkout", post(start_checkout))
        .route("/{slug}/billing/portal", post(open_billing_portal))
        .route("/{slug}/entitlements", get(get_entitlements))
        .route("/{slug}/custom-domain/verify", post(verify_custom_domain))
        .route("/{slug}/members", get(list_members).post(add_member))
        .route(
            "/{slug}/invitations",
            get(list_invitations).post(create_invitation),
        )
        .route(
            "/{slug}/members/{member_id}",
            axum::routing::patch(update_member).delete(remove_member),
        )
        .route(
            "/{slug}/invitations/{invitation_id}",
            axum::routing::delete(delete_invitation),
        )
        .route(
            "/{slug}/invitations/{invitation_id}/resend",
            post(resend_invitation),
        )
}

#[derive(Serialize)]
struct DataResponse<T: Serialize> {
    data: T,
}

#[derive(Serialize)]
struct BillingSummary {
    billing_enabled: bool,
    portal_enabled: bool,
    checkout_enabled: bool,
    current_plan: OrganizationPlan,
    subscription_status: shared::enums::SubscriptionStatus,
    stripe_customer_id: Option<String>,
    billing_email: Option<String>,
    current_period_end: Option<chrono::DateTime<chrono::Utc>>,
    cancel_at_period_end: bool,
    available_upgrades: Vec<OrganizationPlan>,
    entitlements: BillingEntitlements,
    downgrade_target_plan: Option<OrganizationPlan>,
    downgrade_started_at: Option<chrono::DateTime<chrono::Utc>>,
    downgrade_grace_ends_at: Option<chrono::DateTime<chrono::Utc>>,
    downgrade_state: shared::enums::DowngradeState,
    entitlement_violations: Vec<EntitlementViolation>,
    required_actions: Vec<String>,
    self_serve_downgrade: bool,
}

#[derive(Deserialize)]
struct CheckoutRequest {
    plan: OrganizationPlan,
}

#[derive(Serialize)]
struct BillingSessionResponse {
    url: String,
}

#[derive(Serialize)]
struct CustomDomainVerificationResponse {
    domain: String,
    expected_target: String,
    resolved_addresses: Vec<String>,
    expected_addresses: Vec<String>,
    is_ready: bool,
    message: String,
}

async fn create_organization(
    State(state): State<AppState>,
    current_user: CurrentUser,
    Json(req): Json<CreateOrganizationRequest>,
) -> Result<(axum::http::StatusCode, Json<DataResponse<Organization>>), AppError> {
    validate_org_name(&req.name)?;

    let slug = match &req.slug {
        Some(s) => {
            validate_slug(s)?;
            s.clone()
        }
        None => {
            let generated = slugify(&req.name);
            // Validate generated slug still passes rules
            if validate_slug(&generated).is_err() {
                return Err(AppError::Validation(
                    "Could not generate a valid slug from the name. Please provide one."
                        .to_string(),
                ));
            }
            generated
        }
    };

    if db::organizations::slug_exists(&state.pool, &slug).await? {
        return Err(AppError::Conflict(format!(
            "Slug '{}' is already taken",
            slug
        )));
    }

    let org = db::organizations::create(&state.pool, &req, &slug).await?;

    // Auto-add creator as owner
    db::members::create(&state.pool, org.id, current_user.user.id, MemberRole::Owner).await?;
    db::audit_logs::record(
        &state.pool,
        db::audit_logs::NewAuditLog {
            org_id: org.id,
            actor_user_id: Some(current_user.user.id),
            actor_type: "user",
            action: "organization.create",
            target_type: "organization",
            target_id: Some(&org.id.to_string()),
            details: serde_json::json!({
                "slug": org.slug.clone(),
                "plan": org.plan,
            }),
        },
    )
    .await?;

    Ok((
        axum::http::StatusCode::CREATED,
        Json(DataResponse { data: org }),
    ))
}

async fn list_organizations(
    State(state): State<AppState>,
    current_user: CurrentUser,
) -> Result<Json<DataResponse<Vec<Organization>>>, AppError> {
    let orgs = db::organizations::find_by_user_id(&state.pool, current_user.user.id).await?;
    Ok(Json(DataResponse { data: orgs }))
}

async fn get_organization(
    org_access: OrgAccess,
) -> Result<Json<DataResponse<Organization>>, AppError> {
    Ok(Json(DataResponse {
        data: org_access.org,
    }))
}

async fn update_organization(
    State(state): State<AppState>,
    org_access: OrgAccess,
    Json(req): Json<UpdateOrganizationRequest>,
) -> Result<Json<DataResponse<Organization>>, AppError> {
    org_access.require_admin()?;

    if let Some(ref name) = req.name {
        validate_org_name(name)?;
    }

    if let Some(ref slug) = req.slug {
        validate_slug(slug)?;
        if slug != &org_access.org.slug && db::organizations::slug_exists(&state.pool, slug).await?
        {
            return Err(AppError::Conflict(format!(
                "Slug '{}' is already taken",
                slug
            )));
        }
    }

    if let Some(ref color) = req.brand_color {
        validate_brand_color(color)?;
    }

    if let Some(ref timezone) = req.timezone {
        validate_timezone(timezone)?;
    }

    if let Some(ref custom_domain) = req.custom_domain {
        let normalized = custom_domain
            .trim()
            .trim_end_matches('.')
            .to_ascii_lowercase();
        require_custom_domain_access(org_access.org.plan, &normalized)?;
        if !normalized.is_empty() {
            validate_custom_domain(&normalized)?;
        }

        if !normalized.is_empty()
            && db::organizations::custom_domain_exists(
                &state.pool,
                &normalized,
                Some(org_access.org.id),
            )
            .await?
        {
            return Err(AppError::Conflict(format!(
                "Custom domain '{}' is already in use",
                normalized
            )));
        }
    }

    let org = db::organizations::update(&state.pool, org_access.org.id, &req).await?;
    Ok(Json(DataResponse { data: org }))
}

async fn list_members(
    State(state): State<AppState>,
    org_access: OrgAccess,
) -> Result<Json<DataResponse<Vec<MemberWithUser>>>, AppError> {
    org_access.require_admin()?;

    let members = db::members::find_by_org(&state.pool, org_access.org.id).await?;
    Ok(Json(DataResponse { data: members }))
}

async fn list_invitations(
    State(state): State<AppState>,
    org_access: OrgAccess,
) -> Result<Json<DataResponse<Vec<InvitationWithInviter>>>, AppError> {
    org_access.require_admin()?;

    let invitations = db::invitations::list_by_org(&state.pool, org_access.org.id).await?;
    Ok(Json(DataResponse { data: invitations }))
}

async fn get_billing_summary(
    State(state): State<AppState>,
    org_access: OrgAccess,
) -> Result<Json<DataResponse<BillingSummary>>, AppError> {
    org_access.require_admin()?;

    let billing_enabled = state.config.stripe_secret_key.is_some();
    let available_upgrades = available_upgrades(&org_access.org, &state.config);
    let checkout_enabled = billing_enabled && !available_upgrades.is_empty();
    let portal_enabled = billing_enabled && org_access.org.stripe_customer_id.is_some();
    let entitlement_violations =
        crate::services::downgrade::entitlement_violations(&state.pool, &org_access.org).await?;
    let required_actions = crate::services::downgrade::required_actions(&entitlement_violations);

    Ok(Json(DataResponse {
        data: BillingSummary {
            billing_enabled,
            portal_enabled,
            checkout_enabled,
            current_plan: org_access.org.plan,
            subscription_status: org_access.org.subscription_status,
            stripe_customer_id: org_access.org.stripe_customer_id.clone(),
            billing_email: org_access.org.billing_email.clone(),
            current_period_end: org_access.org.current_period_end,
            cancel_at_period_end: org_access.org.cancel_at_period_end,
            available_upgrades,
            entitlements: BillingEntitlements::from(org_access.org.plan),
            downgrade_target_plan: org_access.org.downgrade_target_plan,
            downgrade_started_at: org_access.org.downgrade_started_at,
            downgrade_grace_ends_at: org_access.org.downgrade_grace_ends_at,
            downgrade_state: org_access.org.downgrade_state,
            entitlement_violations,
            required_actions,
            self_serve_downgrade: portal_enabled,
        },
    }))
}

async fn start_checkout(
    State(state): State<AppState>,
    org_access: OrgAccess,
    Json(req): Json<CheckoutRequest>,
) -> Result<Json<DataResponse<BillingSessionResponse>>, AppError> {
    org_access.require_admin()?;
    ensure_upgrade_allowed(&org_access.org, &state.config, req.plan)?;

    let price_id = crate::services::billing::price_id_for_plan(&state.config, req.plan)
        .ok_or_else(|| {
            AppError::Validation("That upgrade is not configured for this deployment".to_string())
        })?;

    let email = org_access.user.email.trim().to_lowercase();
    let session = crate::services::billing::create_checkout_session(
        &state.config,
        &org_access.org,
        &email,
        req.plan,
    )
    .await?;
    db::organizations::mark_checkout_pending(&state.pool, org_access.org.id, price_id, &email)
        .await?;
    db::audit_logs::record(
        &state.pool,
        db::audit_logs::NewAuditLog {
            org_id: org_access.org.id,
            actor_user_id: Some(org_access.user.id),
            actor_type: "user",
            action: "billing.checkout.start",
            target_type: "organization",
            target_id: Some(&org_access.org.id.to_string()),
            details: serde_json::json!({
                "requested_plan": req.plan,
                "billing_email": email,
            }),
        },
    )
    .await?;

    Ok(Json(DataResponse {
        data: BillingSessionResponse { url: session.url },
    }))
}

async fn open_billing_portal(
    State(state): State<AppState>,
    org_access: OrgAccess,
) -> Result<Json<DataResponse<BillingSessionResponse>>, AppError> {
    org_access.require_admin()?;

    let session =
        crate::services::billing::create_portal_session(&state.config, &org_access.org).await?;
    db::audit_logs::record(
        &state.pool,
        db::audit_logs::NewAuditLog {
            org_id: org_access.org.id,
            actor_user_id: Some(org_access.user.id),
            actor_type: "user",
            action: "billing.portal.open",
            target_type: "organization",
            target_id: Some(&org_access.org.id.to_string()),
            details: serde_json::json!({}),
        },
    )
    .await?;

    Ok(Json(DataResponse {
        data: BillingSessionResponse { url: session.url },
    }))
}

async fn get_entitlements(
    org_access: OrgAccess,
) -> Result<Json<DataResponse<BillingEntitlements>>, AppError> {
    Ok(Json(DataResponse {
        data: BillingEntitlements::from(org_access.org.plan),
    }))
}

async fn verify_custom_domain(
    State(state): State<AppState>,
    org_access: OrgAccess,
) -> Result<Json<DataResponse<CustomDomainVerificationResponse>>, AppError> {
    org_access.require_admin()?;

    let domain = org_access
        .org
        .custom_domain
        .clone()
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| {
            AppError::Validation("Set a custom domain before running verification".to_string())
        })?;
    require_custom_domain_access(org_access.org.plan, &domain)?;

    let expected_target = managed_host_target(&state.config)?;
    let resolved_addresses = resolve_host_addresses(&domain).await?;
    let expected_addresses = resolve_host_addresses(&expected_target).await?;
    let is_ready = !resolved_addresses.is_empty()
        && !expected_addresses.is_empty()
        && resolved_addresses
            .iter()
            .any(|address| expected_addresses.contains(address));

    let message = if is_ready {
        db::organizations::mark_custom_domain_verified(&state.pool, org_access.org.id).await?;
        "Custom domain resolves to the managed target and is ready to use.".to_string()
    } else if resolved_addresses.is_empty() {
        "We could not resolve that domain yet. Add the DNS record and try again.".to_string()
    } else if expected_addresses.is_empty() {
        "Managed target lookup is not available in this environment yet.".to_string()
    } else {
        "The domain resolves, but not to the current managed target yet.".to_string()
    };

    db::audit_logs::record(
        &state.pool,
        db::audit_logs::NewAuditLog {
            org_id: org_access.org.id,
            actor_user_id: Some(org_access.user.id),
            actor_type: "user",
            action: "custom_domain.verify",
            target_type: "organization",
            target_id: Some(&org_access.org.id.to_string()),
            details: serde_json::json!({
                "domain": domain.clone(),
                "expected_target": expected_target.clone(),
                "resolved_addresses": resolved_addresses.clone(),
                "expected_addresses": expected_addresses.clone(),
                "is_ready": is_ready,
            }),
        },
    )
    .await?;

    Ok(Json(DataResponse {
        data: CustomDomainVerificationResponse {
            domain,
            expected_target,
            resolved_addresses,
            expected_addresses,
            is_ready,
            message,
        },
    }))
}

async fn add_member(
    State(state): State<AppState>,
    org_access: OrgAccess,
    Json(req): Json<CreateMemberRequest>,
) -> Result<(axum::http::StatusCode, Json<DataResponse<MemberWithUser>>), AppError> {
    org_access.require_admin()?;
    require_owner_for_owner_role(&org_access.role, req.role)?;

    let email = req.email.trim().to_lowercase();
    if email.is_empty() || !email.contains('@') {
        return Err(AppError::Validation(
            "Enter the email address of an existing user".to_string(),
        ));
    }

    let user = db::users::find_by_email(&state.pool, &email)
        .await?
        .ok_or_else(|| {
            AppError::Validation(
                "That user has not signed in yet. Ask them to log in once before adding them."
                    .to_string(),
            )
        })?;

    if db::members::find_by_user_and_org(&state.pool, user.id, org_access.org.id)
        .await?
        .is_some()
    {
        return Err(AppError::Conflict(
            "That user is already a member of this organization".to_string(),
        ));
    }

    let member = db::members::create(&state.pool, org_access.org.id, user.id, req.role).await?;
    let member = db::members::find_by_org(&state.pool, org_access.org.id)
        .await?
        .into_iter()
        .find(|candidate| candidate.id == member.id)
        .ok_or_else(|| AppError::Internal(anyhow::anyhow!("member lookup failed")))?;

    Ok((
        axum::http::StatusCode::CREATED,
        Json(DataResponse { data: member }),
    ))
}

async fn create_invitation(
    State(state): State<AppState>,
    org_access: OrgAccess,
    Json(req): Json<CreateInvitationRequest>,
) -> Result<
    (
        axum::http::StatusCode,
        Json<DataResponse<InvitationWithInviter>>,
    ),
    AppError,
> {
    org_access.require_admin()?;
    require_owner_for_owner_role(&org_access.role, req.role)?;

    let email = req.email.trim().to_lowercase();
    if !looks_like_email(&email) {
        return Err(AppError::Validation(
            "Enter a valid teammate email address".to_string(),
        ));
    }

    if let Some(user) = db::users::find_by_email(&state.pool, &email).await? {
        if db::members::find_by_user_and_org(&state.pool, user.id, org_access.org.id)
            .await?
            .is_some()
        {
            return Err(AppError::Conflict(
                "That user is already a member of this organization".to_string(),
            ));
        }
    }

    if db::invitations::find_active_by_email(&state.pool, org_access.org.id, &email)
        .await?
        .is_some()
    {
        return Err(AppError::Conflict(
            "There is already a pending invitation for that email".to_string(),
        ));
    }

    let token = Uuid::new_v4().to_string();
    let invitation = db::invitations::create(
        &state.pool,
        org_access.org.id,
        &email,
        req.role,
        org_access.user.id,
        &token,
    )
    .await?;
    crate::services::email_notifications::queue_invitation_email(
        &state.pool,
        org_access.org.id,
        &state.config.app_base_url,
        &org_access.org.name,
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
            org_id: org_access.org.id,
            actor_user_id: Some(org_access.user.id),
            actor_type: "user",
            action: "invitation.create",
            target_type: "invitation",
            target_id: Some(&invitation.id.to_string()),
            details: serde_json::json!({
                "email": invitation.email.clone(),
                "role": invitation.role,
                "expires_at": invitation.expires_at,
                "delivery_status": invitation.delivery_status,
            }),
        },
    )
    .await?;

    Ok((
        axum::http::StatusCode::CREATED,
        Json(DataResponse { data: invitation }),
    ))
}

async fn update_member(
    State(state): State<AppState>,
    current_user: CurrentUser,
    org_access: OrgAccess,
    Path((_slug, member_id)): Path<(String, Uuid)>,
    Json(req): Json<UpdateMemberRequest>,
) -> Result<Json<DataResponse<MemberWithUser>>, AppError> {
    org_access.require_admin()?;
    require_owner_for_owner_role(&org_access.role, req.role)?;

    let existing = db::members::find_by_id(&state.pool, member_id)
        .await?
        .ok_or_else(|| AppError::NotFound("Member not found".to_string()))?;
    ensure_member_in_org(&existing, org_access.org.id)?;

    if existing.role == MemberRole::Owner && req.role != MemberRole::Owner {
        org_access.require_owner()?;
        ensure_not_last_owner(&state.pool, org_access.org.id).await?;
    }

    if existing.user_id == current_user.user.id && req.role != MemberRole::Owner {
        ensure_not_last_owner_if_self(&state.pool, org_access.org.id, existing.role)?;
    }

    db::members::update_role(&state.pool, org_access.org.id, member_id, req.role).await?;
    let member = db::members::find_by_org(&state.pool, org_access.org.id)
        .await?
        .into_iter()
        .find(|candidate| candidate.id == member_id)
        .ok_or_else(|| AppError::Internal(anyhow::anyhow!("member lookup failed")))?;

    Ok(Json(DataResponse { data: member }))
}

async fn remove_member(
    State(state): State<AppState>,
    current_user: CurrentUser,
    org_access: OrgAccess,
    Path((_slug, member_id)): Path<(String, Uuid)>,
) -> Result<axum::http::StatusCode, AppError> {
    org_access.require_admin()?;

    let existing = db::members::find_by_id(&state.pool, member_id)
        .await?
        .ok_or_else(|| AppError::NotFound("Member not found".to_string()))?;
    ensure_member_in_org(&existing, org_access.org.id)?;

    if existing.role == MemberRole::Owner {
        org_access.require_owner()?;
        ensure_not_last_owner(&state.pool, org_access.org.id).await?;
    }

    if existing.user_id == current_user.user.id && existing.role == MemberRole::Owner {
        ensure_not_last_owner(&state.pool, org_access.org.id).await?;
    }

    db::members::delete_scoped(&state.pool, org_access.org.id, member_id).await?;
    Ok(axum::http::StatusCode::NO_CONTENT)
}

async fn delete_invitation(
    State(state): State<AppState>,
    org_access: OrgAccess,
    Path((_slug, invitation_id)): Path<(String, Uuid)>,
) -> Result<axum::http::StatusCode, AppError> {
    org_access.require_admin()?;

    db::invitations::cancel_scoped(&state.pool, org_access.org.id, invitation_id).await?;
    db::audit_logs::record(
        &state.pool,
        db::audit_logs::NewAuditLog {
            org_id: org_access.org.id,
            actor_user_id: Some(org_access.user.id),
            actor_type: "user",
            action: "invitation.cancel",
            target_type: "invitation",
            target_id: Some(&invitation_id.to_string()),
            details: serde_json::json!({}),
        },
    )
    .await?;
    Ok(axum::http::StatusCode::NO_CONTENT)
}

async fn resend_invitation(
    State(state): State<AppState>,
    org_access: OrgAccess,
    Path((_slug, invitation_id)): Path<(String, Uuid)>,
) -> Result<Json<DataResponse<InvitationWithInviter>>, AppError> {
    org_access.require_admin()?;

    let invitation = db::invitations::find_by_id(&state.pool, org_access.org.id, invitation_id)
        .await?
        .ok_or_else(|| AppError::NotFound("Invitation not found".to_string()))?;

    if invitation.accepted_at.is_some()
        || invitation.canceled_at.is_some()
        || invitation.expires_at <= chrono::Utc::now()
    {
        return Err(AppError::Validation(
            "Only pending invitations can be resent".to_string(),
        ));
    }

    crate::services::email_notifications::queue_invitation_email(
        &state.pool,
        org_access.org.id,
        &state.config.app_base_url,
        &org_access.org.name,
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
            org_id: org_access.org.id,
            actor_user_id: Some(org_access.user.id),
            actor_type: "user",
            action: "invitation.resend",
            target_type: "invitation",
            target_id: Some(&invitation.id.to_string()),
            details: serde_json::json!({ "email": invitation.email }),
        },
    )
    .await?;

    let invitation = db::invitations::find_by_id(&state.pool, org_access.org.id, invitation.id)
        .await?
        .ok_or_else(|| AppError::Internal(anyhow::anyhow!("invitation lookup failed")))?;

    Ok(Json(DataResponse { data: invitation }))
}

fn require_owner_for_owner_role(
    current_role: &MemberRole,
    requested_role: MemberRole,
) -> Result<(), AppError> {
    if requested_role == MemberRole::Owner && *current_role != MemberRole::Owner {
        return Err(AppError::Forbidden(
            "Only owners can grant owner access".to_string(),
        ));
    }

    Ok(())
}

fn ensure_member_in_org(
    member: &shared::models::member::Member,
    org_id: Uuid,
) -> Result<(), AppError> {
    if member.org_id != org_id {
        return Err(AppError::NotFound("Member not found".to_string()));
    }

    Ok(())
}

async fn ensure_not_last_owner(pool: &sqlx::PgPool, org_id: Uuid) -> Result<(), AppError> {
    if db::members::count_by_role(pool, org_id, MemberRole::Owner).await? <= 1 {
        return Err(AppError::Validation(
            "This organization must keep at least one owner".to_string(),
        ));
    }

    Ok(())
}

fn ensure_not_last_owner_if_self(
    _pool: &sqlx::PgPool,
    _org_id: Uuid,
    existing_role: MemberRole,
) -> Result<(), AppError> {
    if existing_role == MemberRole::Owner {
        return Err(AppError::Validation(
            "Owners cannot demote themselves here. Transfer ownership to another member first."
                .to_string(),
        ));
    }

    Ok(())
}

fn available_upgrades(org: &Organization, config: &crate::config::Config) -> Vec<OrganizationPlan> {
    match org.plan {
        OrganizationPlan::Free => [OrganizationPlan::Pro, OrganizationPlan::Team]
            .into_iter()
            .filter(|plan| crate::services::billing::price_id_for_plan(config, *plan).is_some())
            .collect(),
        OrganizationPlan::Pro => [OrganizationPlan::Team]
            .into_iter()
            .filter(|plan| crate::services::billing::price_id_for_plan(config, *plan).is_some())
            .collect(),
        OrganizationPlan::Team => Vec::new(),
    }
}

fn ensure_upgrade_allowed(
    org: &Organization,
    config: &crate::config::Config,
    requested_plan: OrganizationPlan,
) -> Result<(), AppError> {
    if !available_upgrades(org, config).contains(&requested_plan) {
        return Err(AppError::Validation(format!(
            "You can only upgrade from {} to a higher managed beta plan.",
            plan_name(org.plan)
        )));
    }

    Ok(())
}

fn require_custom_domain_access(
    plan: OrganizationPlan,
    normalized_custom_domain: &str,
) -> Result<(), AppError> {
    if !normalized_custom_domain.is_empty() && !plan.allows_custom_domain() {
        return Err(AppError::Validation(
            "Custom domains are available on Pro and Team plans. Upgrade to connect one."
                .to_string(),
        ));
    }

    Ok(())
}

fn plan_name(plan: OrganizationPlan) -> &'static str {
    match plan {
        OrganizationPlan::Free => "Free",
        OrganizationPlan::Pro => "Pro",
        OrganizationPlan::Team => "Team",
    }
}

async fn resolve_host_addresses(host: &str) -> Result<Vec<String>, AppError> {
    let Ok(addresses) = tokio::net::lookup_host((host, 80)).await else {
        return Ok(Vec::new());
    };

    let mut unique = addresses
        .map(|address| address.ip().to_string())
        .collect::<Vec<_>>();
    unique.sort();
    unique.dedup();
    Ok(unique)
}

fn managed_host_target(config: &crate::config::Config) -> Result<String, AppError> {
    if let Some(host) = config
        .statuspage_host
        .as_ref()
        .filter(|value| !value.is_empty())
    {
        return Ok(host.clone());
    }

    extract_host_from_url(&config.app_base_url).ok_or_else(|| {
        AppError::Validation(
            "Set STATUSPAGE_HOST or use a valid APP_BASE_URL before verifying a custom domain."
                .to_string(),
        )
    })
}

fn extract_host_from_url(url: &str) -> Option<String> {
    let without_scheme = url.split("://").nth(1).unwrap_or(url);
    without_scheme
        .split('/')
        .next()
        .and_then(|host| host.split(':').next())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
}

fn looks_like_email(email: &str) -> bool {
    let email = email.trim();
    let Some(at_index) = email.find('@') else {
        return false;
    };

    at_index > 0 && at_index < email.len() - 3 && email[at_index + 1..].contains('.')
}

#[cfg(test)]
mod tests {
    use super::*;

    fn base_org(plan: OrganizationPlan) -> Organization {
        Organization {
            id: Uuid::nil(),
            name: "Demo".to_string(),
            slug: "demo".to_string(),
            plan,
            logo_url: None,
            brand_color: "#3B82F6".to_string(),
            timezone: "UTC".to_string(),
            custom_domain: None,
            custom_domain_verified_at: None,
            custom_domain_status: shared::enums::CustomDomainStatus::NotConfigured,
            stripe_customer_id: None,
            stripe_subscription_id: None,
            subscription_status: shared::enums::SubscriptionStatus::Inactive,
            stripe_price_id: None,
            current_period_end: None,
            cancel_at_period_end: false,
            billing_email: None,
            trial_ends_at: None,
            downgrade_target_plan: None,
            downgrade_started_at: None,
            downgrade_grace_ends_at: None,
            downgrade_state: shared::enums::DowngradeState::None,
            downgrade_warning_stage: 0,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        }
    }

    fn managed_config() -> crate::config::Config {
        crate::config::Config {
            database_url: "postgres://statuspage:statuspage@localhost:5432/statuspage".to_string(),
            redis_url: "redis://localhost:6379".to_string(),
            webhook_dispatch_interval_secs: 3,
            webhook_dispatch_batch_size: 10,
            webhook_timeout_secs: 10,
            smtp_host: None,
            smtp_port: 1025,
            smtp_username: None,
            smtp_password: None,
            smtp_secure: false,
            email_from: "alerts@example.com".to_string(),
            app_base_url: "https://app.statuspage.test".to_string(),
            email_dispatch_interval_secs: 3,
            email_dispatch_batch_size: 20,
            stripe_secret_key: Some("sk_test_123".to_string()),
            stripe_webhook_secret: Some("whsec_test".to_string()),
            stripe_price_pro: Some("price_pro".to_string()),
            stripe_price_team: Some("price_team".to_string()),
            internal_admin_token: Some("internal-admin-token".to_string()),
            downgrade_enforcement_interval_secs: 60,
            api_port: 4000,
            api_host: "127.0.0.1".to_string(),
            cors_origin: "http://localhost:3000".to_string(),
            statuspage_host: Some("statuspage.test".to_string()),
            run_migrations_on_start: false,
            run_migrations_only: false,
            log_level: "info".to_string(),
        }
    }

    #[test]
    fn available_upgrades_respects_current_plan_and_config() {
        let config = managed_config();
        assert_eq!(
            available_upgrades(&base_org(OrganizationPlan::Free), &config),
            vec![OrganizationPlan::Pro, OrganizationPlan::Team]
        );
        assert_eq!(
            available_upgrades(&base_org(OrganizationPlan::Pro), &config),
            vec![OrganizationPlan::Team]
        );
        assert!(available_upgrades(&base_org(OrganizationPlan::Team), &config).is_empty());
    }

    #[test]
    fn custom_domain_requires_paid_plan() {
        let result = require_custom_domain_access(OrganizationPlan::Free, "status.example.com");
        assert!(result.is_err());
    }

    #[test]
    fn custom_domain_allows_empty_value_for_free_plan() {
        let result = require_custom_domain_access(OrganizationPlan::Free, "");
        assert!(result.is_ok());
    }

    #[test]
    fn extract_host_from_url_handles_scheme_and_port() {
        assert_eq!(
            extract_host_from_url("https://status.example.com:443/app"),
            Some("status.example.com".to_string())
        );
        assert_eq!(
            extract_host_from_url("localhost:3000"),
            Some("localhost".to_string())
        );
    }

    #[test]
    fn looks_like_email_rejects_malformed_values() {
        assert!(looks_like_email("owner@example.com"));
        assert!(!looks_like_email("owner"));
        assert!(!looks_like_email("@example.com"));
    }
}
