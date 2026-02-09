use axum::extract::FromRequestParts;
use axum::http::request::Parts;
use shared::error::AppError;
use shared::models::user::User;

use crate::state::AppState;

#[derive(Debug, Clone)]
pub struct CurrentUser {
    pub user: User,
}

impl FromRequestParts<AppState> for CurrentUser {
    type Rejection = AppError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        // Extract session token from cookies
        let cookie_header = parts
            .headers
            .get("cookie")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("");

        let session_token = extract_session_token(cookie_header)
            .ok_or(AppError::Unauthorized)?;

        // Query session + user from database
        let user = sqlx::query_as::<_, User>(
            r#"
            SELECT u.id, u.name, u.email, u."emailVerified", u.image, u.created_at
            FROM users u
            JOIN sessions s ON s."userId" = u.id
            WHERE s."sessionToken" = $1 AND s.expires > NOW()
            "#,
        )
        .bind(&session_token)
        .fetch_optional(&state.pool)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to query session");
            AppError::Unauthorized
        })?
        .ok_or(AppError::Unauthorized)?;

        Ok(CurrentUser { user })
    }
}

fn extract_session_token(cookie_header: &str) -> Option<String> {
    for cookie in cookie_header.split(';') {
        let cookie = cookie.trim();
        // Check both dev and production cookie names
        if let Some(value) = cookie
            .strip_prefix("authjs.session-token=")
            .or_else(|| cookie.strip_prefix("__Secure-authjs.session-token="))
        {
            return Some(value.to_string());
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_session_token_dev() {
        let header = "authjs.session-token=abc123; other=value";
        assert_eq!(extract_session_token(header), Some("abc123".to_string()));
    }

    #[test]
    fn test_extract_session_token_production() {
        let header = "__Secure-authjs.session-token=xyz789; other=value";
        assert_eq!(extract_session_token(header), Some("xyz789".to_string()));
    }

    #[test]
    fn test_extract_session_token_missing() {
        let header = "other=value; another=thing";
        assert_eq!(extract_session_token(header), None);
    }
}
