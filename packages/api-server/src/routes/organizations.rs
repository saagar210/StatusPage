use axum::{
    extract::State,
    routing::{get, post},
    Json, Router,
};
use serde::Serialize;

use shared::enums::MemberRole;
use shared::error::AppError;
use shared::models::organization::{
    CreateOrganizationRequest, Organization, UpdateOrganizationRequest,
};
use shared::validation::{slugify, validate_brand_color, validate_org_name, validate_slug};

use crate::db;
use crate::middleware::auth::CurrentUser;
use crate::middleware::org_access::OrgAccess;
use crate::state::AppState;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", post(create_organization).get(list_organizations))
        .route("/:slug", get(get_organization).patch(update_organization))
}

#[derive(Serialize)]
struct DataResponse<T: Serialize> {
    data: T,
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

    let org = db::organizations::update(&state.pool, org_access.org.id, &req).await?;
    Ok(Json(DataResponse { data: org }))
}
