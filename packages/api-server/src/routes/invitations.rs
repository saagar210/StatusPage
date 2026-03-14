use axum::{
    extract::{Path, State},
    http::HeaderMap,
    routing::post,
    Json, Router,
};
use serde::Serialize;

use shared::error::AppError;

use crate::db;
use crate::middleware::auth::CurrentUser;
use crate::state::AppState;

pub fn router() -> Router<AppState> {
    Router::new().route("/{token}/accept", post(accept_invitation))
}

#[derive(Serialize)]
struct DataResponse<T: Serialize> {
    data: T,
}

#[derive(Serialize)]
struct InvitationAcceptanceResponse {
    org_slug: String,
    org_name: String,
    role: shared::enums::MemberRole,
    message: String,
}

async fn accept_invitation(
    State(state): State<AppState>,
    headers: HeaderMap,
    current_user: CurrentUser,
    Path(token): Path<String>,
) -> Result<Json<DataResponse<InvitationAcceptanceResponse>>, AppError> {
    let subject = format!(
        "{}:{}",
        crate::services::rate_limit::rate_limit_subject(&headers, "local"),
        current_user.user.email.trim().to_lowercase()
    );
    crate::services::rate_limit::enforce_rate_limit(
        &state.redis,
        "invitation_accept",
        &subject,
        10,
        std::time::Duration::from_secs(15 * 60),
    )
    .await?;

    let invitation = db::invitations::find_by_token(&state.pool, &token)
        .await?
        .ok_or_else(|| AppError::NotFound("Invitation not found".to_string()))?;
    let user_email = current_user.user.email.trim().to_lowercase();
    validate_invitation_acceptance(&invitation, &user_email)?;

    if db::members::find_by_user_and_org(&state.pool, current_user.user.id, invitation.org_id)
        .await?
        .is_none()
    {
        db::members::create(
            &state.pool,
            invitation.org_id,
            current_user.user.id,
            invitation.role,
        )
        .await?;
    }

    db::invitations::mark_accepted(&state.pool, invitation.id).await?;
    db::audit_logs::record(
        &state.pool,
        db::audit_logs::NewAuditLog {
            org_id: invitation.org_id,
            actor_user_id: Some(current_user.user.id),
            actor_type: "user",
            action: "invitation.accept",
            target_type: "invitation",
            target_id: Some(&invitation.id.to_string()),
            details: serde_json::json!({
                "email": invitation.email,
                "role": invitation.role,
            }),
        },
    )
    .await?;

    let org = db::organizations::find_by_id(&state.pool, invitation.org_id)
        .await?
        .ok_or_else(|| AppError::NotFound("Organization not found".to_string()))?;

    Ok(Json(DataResponse {
        data: InvitationAcceptanceResponse {
            org_slug: org.slug,
            org_name: org.name,
            role: invitation.role,
            message: "Invitation accepted. Your dashboard access is ready.".to_string(),
        },
    }))
}

fn validate_invitation_acceptance(
    invitation: &shared::models::invitation::Invitation,
    user_email: &str,
) -> Result<(), AppError> {
    if invitation.accepted_at.is_some() {
        return Err(AppError::Validation(
            "That invitation has already been accepted".to_string(),
        ));
    }

    if invitation.canceled_at.is_some() {
        return Err(AppError::Validation(
            "That invitation is no longer active. Ask an admin to send a new one.".to_string(),
        ));
    }

    if invitation.expires_at < chrono::Utc::now() {
        return Err(AppError::Validation(
            "That invitation has expired. Ask an admin to send a new one.".to_string(),
        ));
    }

    if user_email != invitation.email.trim().to_lowercase() {
        return Err(AppError::Forbidden(
            "Sign in with the GitHub account that matches the invited email address.".to_string(),
        ));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use shared::enums::MemberRole;
    use shared::models::invitation::Invitation;
    use uuid::Uuid;

    fn invitation(email: &str) -> Invitation {
        Invitation {
            id: Uuid::new_v4(),
            org_id: Uuid::new_v4(),
            email: email.to_string(),
            role: MemberRole::Member,
            invited_by: Uuid::new_v4(),
            token: "token".to_string(),
            expires_at: chrono::Utc::now() + chrono::Duration::days(1),
            accepted_at: None,
            canceled_at: None,
            last_sent_at: None,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        }
    }

    #[test]
    fn invitation_acceptance_allows_matching_email() {
        let invitation = invitation("owner@example.com");
        assert!(validate_invitation_acceptance(&invitation, "owner@example.com").is_ok());
    }

    #[test]
    fn invitation_acceptance_rejects_already_accepted_invites() {
        let mut invitation = invitation("owner@example.com");
        invitation.accepted_at = Some(chrono::Utc::now());
        assert!(validate_invitation_acceptance(&invitation, "owner@example.com").is_err());
    }

    #[test]
    fn invitation_acceptance_rejects_expired_invites() {
        let mut invitation = invitation("owner@example.com");
        invitation.expires_at = chrono::Utc::now() - chrono::Duration::hours(1);
        assert!(validate_invitation_acceptance(&invitation, "owner@example.com").is_err());
    }

    #[test]
    fn invitation_acceptance_rejects_canceled_invites() {
        let mut invitation = invitation("owner@example.com");
        invitation.canceled_at = Some(chrono::Utc::now());
        assert!(validate_invitation_acceptance(&invitation, "owner@example.com").is_err());
    }

    #[test]
    fn invitation_acceptance_rejects_email_mismatch() {
        let invitation = invitation("owner@example.com");
        assert!(validate_invitation_acceptance(&invitation, "other@example.com").is_err());
    }
}
