use axum::{
    extract::{Path, State},
    routing::{get, post},
    Json, Router,
};
use serde::Serialize;
use uuid::Uuid;

use shared::enums::MemberRole;
use shared::error::AppError;
use shared::models::member::{CreateMemberRequest, MemberWithUser, UpdateMemberRequest};
use shared::models::organization::{
    CreateOrganizationRequest, Organization, UpdateOrganizationRequest,
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
        .route("/{slug}/members", get(list_members).post(add_member))
        .route(
            "/{slug}/members/{member_id}",
            axum::routing::patch(update_member).delete(remove_member),
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
    current_plan: MemberFacingPlan,
    stripe_customer_id: Option<String>,
    available_upgrades: Vec<MemberFacingPlan>,
}

#[derive(Serialize, Clone, Copy)]
#[serde(rename_all = "snake_case")]
enum MemberFacingPlan {
    Free,
    Pro,
    Team,
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
        validate_custom_domain(&normalized)?;

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

async fn get_billing_summary(
    State(state): State<AppState>,
    org_access: OrgAccess,
) -> Result<Json<DataResponse<BillingSummary>>, AppError> {
    org_access.require_admin()?;

    let billing_enabled = state.config.stripe_secret_key.is_some();
    let checkout_enabled = billing_enabled
        && state.config.stripe_price_pro.is_some()
        && state.config.stripe_price_team.is_some();
    let portal_enabled = billing_enabled && state.config.stripe_webhook_secret.is_some();

    let current_plan = match org_access.org.plan {
        shared::enums::OrganizationPlan::Free => MemberFacingPlan::Free,
        shared::enums::OrganizationPlan::Pro => MemberFacingPlan::Pro,
        shared::enums::OrganizationPlan::Team => MemberFacingPlan::Team,
    };

    let available_upgrades = match org_access.org.plan {
        shared::enums::OrganizationPlan::Free => {
            vec![MemberFacingPlan::Pro, MemberFacingPlan::Team]
        }
        shared::enums::OrganizationPlan::Pro => vec![MemberFacingPlan::Team],
        shared::enums::OrganizationPlan::Team => Vec::new(),
    };

    Ok(Json(DataResponse {
        data: BillingSummary {
            billing_enabled,
            portal_enabled,
            checkout_enabled,
            current_plan,
            stripe_customer_id: org_access.org.stripe_customer_id.clone(),
            available_upgrades,
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
