use axum::{
    extract::{Path, Query, State},
    routing::get,
    Json, Router,
};
use serde::Deserialize;
use serde::Serialize;
use uuid::Uuid;

use shared::enums::OrganizationPlan;
use shared::error::AppError;
use shared::models::notification_preference::{
    NotificationPreferences, UpdateNotificationPreferencesRequest,
};
use shared::models::webhook::{
    CreateWebhookConfigRequest, UpdateWebhookConfigRequest, WebhookConfig,
};

use crate::db;
use crate::middleware::org_access::OrgAccess;
use crate::state::AppState;

const ALLOWED_WEBHOOK_EVENTS: &[&str] = &[
    "incident.created",
    "incident.updated",
    "incident.resolved",
    "service.status_changed",
];

pub fn router() -> Router<AppState> {
    Router::new()
        .route(
            "/preferences",
            get(get_preferences).patch(update_preferences),
        )
        .route("/subscribers", get(list_subscribers))
        .route(
            "/subscribers/{id}",
            axum::routing::delete(delete_subscriber),
        )
        .route(
            "/subscribers/{id}/resend",
            axum::routing::post(resend_subscriber_verification),
        )
        .route("/deliveries/email", get(list_email_deliveries))
        .route(
            "/deliveries/email/{id}/retry",
            axum::routing::post(retry_email_delivery),
        )
        .route("/deliveries/webhooks", get(list_webhook_deliveries))
        .route(
            "/deliveries/webhooks/{id}/retry",
            axum::routing::post(retry_webhook_delivery),
        )
        .route("/webhooks", get(list_webhooks).post(create_webhook))
        .route(
            "/webhooks/{id}",
            get(get_webhook)
                .patch(update_webhook)
                .delete(delete_webhook),
        )
}

#[derive(Serialize)]
struct DataResponse<T: Serialize> {
    data: T,
}

#[derive(Serialize)]
struct ListResponse<T: Serialize> {
    data: Vec<T>,
    pagination: Pagination,
}

#[derive(Serialize)]
struct Pagination {
    page: i64,
    per_page: i64,
    total: i64,
}

#[derive(Serialize)]
struct MessageResponse {
    message: String,
}

#[derive(Deserialize)]
struct ListParams {
    page: Option<i64>,
    per_page: Option<i64>,
    status: Option<String>,
}

fn bounded_page(page: Option<i64>) -> i64 {
    page.unwrap_or(1).max(1)
}

fn bounded_per_page(per_page: Option<i64>, default_value: i64) -> i64 {
    per_page.unwrap_or(default_value).clamp(1, 50)
}

fn normalized_status(status: Option<String>) -> Option<String> {
    status
        .map(|value| value.trim().to_lowercase())
        .filter(|value| !value.is_empty() && value != "all")
}

async fn get_preferences(
    State(state): State<AppState>,
    org_access: OrgAccess,
) -> Result<Json<DataResponse<NotificationPreferences>>, AppError> {
    org_access.require_admin()?;

    let preferences =
        db::notification_preferences::get_or_create(&state.pool, org_access.org.id).await?;
    Ok(Json(DataResponse { data: preferences }))
}

async fn update_preferences(
    State(state): State<AppState>,
    org_access: OrgAccess,
    Json(req): Json<UpdateNotificationPreferencesRequest>,
) -> Result<Json<DataResponse<NotificationPreferences>>, AppError> {
    org_access.require_admin()?;

    if let Some(threshold) = req.uptime_alert_threshold {
        if !(0.0..=100.0).contains(&threshold) {
            return Err(AppError::Validation(
                "Uptime alert threshold must be between 0 and 100".to_string(),
            ));
        }
    }

    let preferences =
        db::notification_preferences::update(&state.pool, org_access.org.id, &req).await?;
    Ok(Json(DataResponse { data: preferences }))
}

async fn list_webhooks(
    State(state): State<AppState>,
    org_access: OrgAccess,
) -> Result<Json<DataResponse<Vec<WebhookConfig>>>, AppError> {
    org_access.require_admin()?;

    let webhooks = db::webhooks::find_by_org(&state.pool, org_access.org.id).await?;
    Ok(Json(DataResponse { data: webhooks }))
}

async fn list_subscribers(
    State(state): State<AppState>,
    org_access: OrgAccess,
) -> Result<Json<DataResponse<Vec<db::subscribers::SubscriberListItem>>>, AppError> {
    org_access.require_admin()?;

    let subscribers = db::subscribers::list_by_org(&state.pool, org_access.org.id).await?;
    Ok(Json(DataResponse { data: subscribers }))
}

async fn delete_subscriber(
    State(state): State<AppState>,
    org_access: OrgAccess,
    Path((_slug, id)): Path<(String, Uuid)>,
) -> Result<axum::http::StatusCode, AppError> {
    org_access.require_admin()?;

    db::subscribers::delete_by_id(&state.pool, org_access.org.id, id).await?;
    Ok(axum::http::StatusCode::NO_CONTENT)
}

async fn resend_subscriber_verification(
    State(state): State<AppState>,
    org_access: OrgAccess,
    Path((_slug, id)): Path<(String, Uuid)>,
) -> Result<Json<DataResponse<MessageResponse>>, AppError> {
    org_access.require_admin()?;

    let verification_token = Uuid::new_v4().to_string();
    let subscriber = db::subscribers::refresh_pending_verification_by_id(
        &state.pool,
        org_access.org.id,
        id,
        &verification_token,
    )
    .await?
    .ok_or_else(|| {
        AppError::Validation("Only pending subscribers can be re-verified".to_string())
    })?;

    let token = subscriber
        .verification_token
        .as_deref()
        .unwrap_or(verification_token.as_str());

    crate::services::email_notifications::queue_subscription_verification(
        &state.pool,
        org_access.org.id,
        &state.config.app_base_url,
        &org_access.org.slug,
        &org_access.org.name,
        &subscriber.email,
        token,
    )
    .await?;

    Ok(Json(DataResponse {
        data: MessageResponse {
            message: format!(
                "Queued another verification email for {}.",
                subscriber.email
            ),
        },
    }))
}

async fn list_email_deliveries(
    State(state): State<AppState>,
    org_access: OrgAccess,
    Query(params): Query<ListParams>,
) -> Result<Json<ListResponse<db::notification_logs::NotificationLogEntry>>, AppError> {
    org_access.require_admin()?;

    let page = bounded_page(params.page);
    let per_page = bounded_per_page(params.per_page, 10);
    let status = normalized_status(params.status);

    let (entries, total) = db::notification_logs::list_recent_by_org(
        &state.pool,
        org_access.org.id,
        page,
        per_page,
        status.as_deref(),
    )
    .await?;
    Ok(Json(ListResponse {
        data: entries,
        pagination: Pagination {
            page,
            per_page,
            total,
        },
    }))
}

async fn list_webhook_deliveries(
    State(state): State<AppState>,
    org_access: OrgAccess,
    Query(params): Query<ListParams>,
) -> Result<Json<ListResponse<db::webhook_deliveries::WebhookDeliveryEntry>>, AppError> {
    org_access.require_admin()?;

    let page = bounded_page(params.page);
    let per_page = bounded_per_page(params.per_page, 10);
    let status = normalized_status(params.status);

    let (entries, total) = db::webhook_deliveries::list_recent_by_org(
        &state.pool,
        org_access.org.id,
        page,
        per_page,
        status.as_deref(),
    )
    .await?;
    Ok(Json(ListResponse {
        data: entries,
        pagination: Pagination {
            page,
            per_page,
            total,
        },
    }))
}

async fn retry_email_delivery(
    State(state): State<AppState>,
    org_access: OrgAccess,
    Path((_slug, id)): Path<(String, Uuid)>,
) -> Result<Json<DataResponse<MessageResponse>>, AppError> {
    org_access.require_admin()?;

    let entry = db::notification_logs::retry_failed_by_id(&state.pool, org_access.org.id, id)
        .await?
        .ok_or_else(|| {
            AppError::Validation("Only failed email deliveries can be retried".to_string())
        })?;

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
    org_access: OrgAccess,
    Path((_slug, id)): Path<(String, Uuid)>,
) -> Result<Json<DataResponse<MessageResponse>>, AppError> {
    org_access.require_admin()?;

    let entry = db::webhook_deliveries::retry_failed_by_id(&state.pool, org_access.org.id, id)
        .await?
        .ok_or_else(|| {
            AppError::Validation("Only failed webhook deliveries can be retried".to_string())
        })?;

    Ok(Json(DataResponse {
        data: MessageResponse {
            message: format!(
                "Queued another webhook delivery attempt for {}.",
                entry.webhook_name
            ),
        },
    }))
}

async fn get_webhook(
    State(state): State<AppState>,
    org_access: OrgAccess,
    Path((_slug, id)): Path<(String, Uuid)>,
) -> Result<Json<DataResponse<WebhookConfig>>, AppError> {
    org_access.require_admin()?;

    let webhook = db::webhooks::find_by_org(&state.pool, org_access.org.id)
        .await?
        .into_iter()
        .find(|candidate| candidate.id == id)
        .ok_or_else(|| AppError::NotFound("Webhook not found".to_string()))?;

    Ok(Json(DataResponse { data: webhook }))
}

async fn create_webhook(
    State(state): State<AppState>,
    org_access: OrgAccess,
    Json(req): Json<CreateWebhookConfigRequest>,
) -> Result<(axum::http::StatusCode, Json<DataResponse<WebhookConfig>>), AppError> {
    org_access.require_admin()?;
    require_webhook_feature(org_access.org.plan)?;
    validate_webhook_payload(&req.name, &req.url, &req.secret, &req.event_types)?;

    let webhook = db::webhooks::create(&state.pool, org_access.org.id, &req).await?;
    db::audit_logs::record(
        &state.pool,
        db::audit_logs::NewAuditLog {
            org_id: org_access.org.id,
            actor_user_id: Some(org_access.user.id),
            actor_type: "user",
            action: "webhook.create",
            target_type: "webhook",
            target_id: Some(&webhook.id.to_string()),
            details: serde_json::json!({
                "name": webhook.name.clone(),
                "url": webhook.url.clone(),
                "event_types": webhook.event_types.clone(),
                "is_enabled": webhook.is_enabled,
            }),
        },
    )
    .await?;
    Ok((
        axum::http::StatusCode::CREATED,
        Json(DataResponse { data: webhook }),
    ))
}

async fn update_webhook(
    State(state): State<AppState>,
    org_access: OrgAccess,
    Path((_slug, id)): Path<(String, Uuid)>,
    Json(req): Json<UpdateWebhookConfigRequest>,
) -> Result<Json<DataResponse<WebhookConfig>>, AppError> {
    org_access.require_admin()?;
    require_webhook_update_access(org_access.org.plan, &req)?;

    if let Some(name) = req.name.as_deref() {
        if name.trim().is_empty() {
            return Err(AppError::Validation("Webhook name is required".to_string()));
        }
    }

    if let Some(url) = req.url.as_deref() {
        validate_webhook_url(url)?;
    }

    if let Some(secret) = req.secret.as_deref() {
        if secret.trim().len() < 8 {
            return Err(AppError::Validation(
                "Webhook secret must be at least 8 characters".to_string(),
            ));
        }
    }

    if let Some(event_types) = req.event_types.as_ref() {
        validate_webhook_event_types(event_types)?;
    }

    let webhook = db::webhooks::update(&state.pool, id, org_access.org.id, &req).await?;
    db::audit_logs::record(
        &state.pool,
        db::audit_logs::NewAuditLog {
            org_id: org_access.org.id,
            actor_user_id: Some(org_access.user.id),
            actor_type: "user",
            action: "webhook.update",
            target_type: "webhook",
            target_id: Some(&webhook.id.to_string()),
            details: serde_json::json!({
                "name": webhook.name.clone(),
                "url": webhook.url.clone(),
                "event_types": webhook.event_types.clone(),
                "is_enabled": webhook.is_enabled,
            }),
        },
    )
    .await?;
    Ok(Json(DataResponse { data: webhook }))
}

async fn delete_webhook(
    State(state): State<AppState>,
    org_access: OrgAccess,
    Path((_slug, id)): Path<(String, Uuid)>,
) -> Result<axum::http::StatusCode, AppError> {
    org_access.require_admin()?;

    db::webhooks::delete(&state.pool, id, org_access.org.id).await?;
    db::audit_logs::record(
        &state.pool,
        db::audit_logs::NewAuditLog {
            org_id: org_access.org.id,
            actor_user_id: Some(org_access.user.id),
            actor_type: "user",
            action: "webhook.delete",
            target_type: "webhook",
            target_id: Some(&id.to_string()),
            details: serde_json::json!({}),
        },
    )
    .await?;
    Ok(axum::http::StatusCode::NO_CONTENT)
}

fn validate_webhook_payload(
    name: &str,
    url: &str,
    secret: &str,
    event_types: &[String],
) -> Result<(), AppError> {
    if name.trim().is_empty() {
        return Err(AppError::Validation("Webhook name is required".to_string()));
    }

    validate_webhook_url(url)?;

    if secret.trim().len() < 8 {
        return Err(AppError::Validation(
            "Webhook secret must be at least 8 characters".to_string(),
        ));
    }

    validate_webhook_event_types(event_types)
}

fn validate_webhook_url(url: &str) -> Result<(), AppError> {
    if !(url.starts_with("https://") || url.starts_with("http://")) {
        return Err(AppError::Validation(
            "Webhook URL must start with http:// or https://".to_string(),
        ));
    }

    Ok(())
}

fn validate_webhook_event_types(event_types: &[String]) -> Result<(), AppError> {
    if event_types.is_empty() {
        return Err(AppError::Validation(
            "Select at least one webhook event".to_string(),
        ));
    }

    for event_type in event_types {
        if !ALLOWED_WEBHOOK_EVENTS.contains(&event_type.as_str()) {
            return Err(AppError::Validation(format!(
                "Unsupported webhook event '{}'",
                event_type
            )));
        }
    }

    Ok(())
}

fn require_webhook_feature(plan: OrganizationPlan) -> Result<(), AppError> {
    if !plan.allows_outbound_webhooks() {
        return Err(AppError::Validation(
            "Outbound webhooks are available on Pro and Team plans. Upgrade to add one."
                .to_string(),
        ));
    }

    Ok(())
}

fn require_webhook_update_access(
    plan: OrganizationPlan,
    req: &UpdateWebhookConfigRequest,
) -> Result<(), AppError> {
    if plan.allows_outbound_webhooks() {
        return Ok(());
    }

    let disabling_only = req.name.is_none()
        && req.url.is_none()
        && req.secret.is_none()
        && req.event_types.is_none()
        && req.is_enabled == Some(false);

    if disabling_only {
        Ok(())
    } else {
        Err(AppError::Validation(
            "Outbound webhooks are available on Pro and Team plans. Upgrade to manage them."
                .to_string(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use shared::models::webhook::UpdateWebhookConfigRequest;

    #[test]
    fn rejects_unknown_webhook_event_types() {
        let result = validate_webhook_event_types(&["incident.foo".to_string()]);
        assert!(result.is_err());
    }

    #[test]
    fn accepts_supported_webhook_event_types() {
        let result = validate_webhook_event_types(&[
            "incident.created".to_string(),
            "service.status_changed".to_string(),
        ]);
        assert!(result.is_ok());
    }

    #[test]
    fn free_plan_cannot_create_webhooks() {
        let result = require_webhook_feature(OrganizationPlan::Free);
        assert!(result.is_err());
    }

    #[test]
    fn free_plan_can_disable_existing_webhook() {
        let result = require_webhook_update_access(
            OrganizationPlan::Free,
            &UpdateWebhookConfigRequest {
                name: None,
                url: None,
                secret: None,
                event_types: None,
                is_enabled: Some(false),
            },
        );
        assert!(result.is_ok());
    }

    #[test]
    fn free_plan_cannot_enable_existing_webhook() {
        let result = require_webhook_update_access(
            OrganizationPlan::Free,
            &UpdateWebhookConfigRequest {
                name: None,
                url: None,
                secret: None,
                event_types: None,
                is_enabled: Some(true),
            },
        );
        assert!(result.is_err());
    }
}
