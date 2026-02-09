use axum::extract::FromRequestParts;
use axum::http::request::Parts;
use shared::enums::MemberRole;
use shared::error::AppError;
use shared::models::organization::Organization;

use crate::state::AppState;

use super::auth::CurrentUser;

#[derive(Debug, Clone)]
pub struct OrgAccess {
    pub org: Organization,
    pub role: MemberRole,
    pub user: shared::models::user::User,
}

impl OrgAccess {
    pub fn require_admin(&self) -> Result<(), AppError> {
        if self.role.is_admin_or_above() {
            Ok(())
        } else {
            Err(AppError::Forbidden(
                "Admin or owner role required".to_string(),
            ))
        }
    }

    pub fn require_owner(&self) -> Result<(), AppError> {
        if self.role == MemberRole::Owner {
            Ok(())
        } else {
            Err(AppError::Forbidden("Owner role required".to_string()))
        }
    }
}

impl FromRequestParts<AppState> for OrgAccess {
    type Rejection = AppError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        // First, extract the current user (runs auth)
        let current_user = CurrentUser::from_request_parts(parts, state).await?;

        // Extract slug from path parameters
        let slug = parts
            .extensions
            .get::<axum::extract::Path<std::collections::HashMap<String, String>>>()
            .and_then(|p| p.get("slug").cloned())
            .or_else(|| {
                // Try extracting from URI path
                extract_slug_from_path(parts.uri.path())
            })
            .ok_or_else(|| AppError::Validation("Missing organization slug".to_string()))?;

        // Query org + membership in one query
        let row = sqlx::query_as::<_, OrgMemberRow>(
            r#"
            SELECT o.id, o.name, o.slug, o.plan, o.logo_url, o.brand_color,
                   o.timezone, o.custom_domain, o.stripe_customer_id,
                   o.created_at, o.updated_at, m.role
            FROM organizations o
            JOIN members m ON m.org_id = o.id
            WHERE o.slug = $1 AND m.user_id = $2
            "#,
        )
        .bind(&slug)
        .bind(current_user.user.id)
        .fetch_optional(&state.pool)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to query org access");
            AppError::NotFound("Organization not found".to_string())
        })?
        // Return 404 (not 403) to prevent org enumeration
        .ok_or_else(|| AppError::NotFound("Organization not found".to_string()))?;

        Ok(OrgAccess {
            org: Organization {
                id: row.id,
                name: row.name,
                slug: row.slug,
                plan: row.plan,
                logo_url: row.logo_url,
                brand_color: row.brand_color,
                timezone: row.timezone,
                custom_domain: row.custom_domain,
                stripe_customer_id: row.stripe_customer_id,
                created_at: row.created_at,
                updated_at: row.updated_at,
            },
            role: row.role,
            user: current_user.user,
        })
    }
}

#[derive(sqlx::FromRow)]
struct OrgMemberRow {
    id: uuid::Uuid,
    name: String,
    slug: String,
    plan: String,
    logo_url: Option<String>,
    brand_color: String,
    timezone: String,
    custom_domain: Option<String>,
    stripe_customer_id: Option<String>,
    created_at: chrono::DateTime<chrono::Utc>,
    updated_at: chrono::DateTime<chrono::Utc>,
    role: MemberRole,
}

/// Extract the org slug from a URL path like /api/organizations/{slug}/...
fn extract_slug_from_path(path: &str) -> Option<String> {
    let parts: Vec<&str> = path.split('/').collect();
    // Find "organizations" in the path and take the next segment
    for (i, part) in parts.iter().enumerate() {
        if *part == "organizations" {
            if let Some(slug) = parts.get(i + 1) {
                if !slug.is_empty() {
                    return Some(slug.to_string());
                }
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_slug_from_path() {
        assert_eq!(
            extract_slug_from_path("/api/organizations/my-org/services"),
            Some("my-org".to_string())
        );
        assert_eq!(
            extract_slug_from_path("/api/organizations/my-org"),
            Some("my-org".to_string())
        );
        assert_eq!(extract_slug_from_path("/api/health"), None);
    }
}
