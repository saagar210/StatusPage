use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

use chrono::{DateTime, Utc};
use hmac::{Hmac, Mac};
use reqwest::StatusCode;
use serde::Deserialize;
use serde_json::Value;
use sha2::Sha256;
use shared::enums::{OrganizationPlan, SubscriptionStatus};
use shared::error::AppError;
use shared::models::organization::Organization;
use uuid::Uuid;

use crate::config::Config;

const STRIPE_API_BASE: &str = "https://api.stripe.com/v1";
const STRIPE_WEBHOOK_TOLERANCE_SECS: i64 = 300;

type HmacSha256 = Hmac<Sha256>;

#[derive(Debug, Clone)]
pub struct BillingSession {
    pub url: String,
}

#[derive(Debug, Clone)]
pub struct CheckoutCompletedEvent {
    pub event_id: String,
    pub event_type: String,
    pub org_id: Option<Uuid>,
    pub customer_id: Option<String>,
    pub subscription_id: Option<String>,
    pub billing_email: Option<String>,
    pub payload: Value,
}

#[derive(Debug, Clone)]
pub struct SubscriptionUpdatedEvent {
    pub event_id: String,
    pub event_type: String,
    pub org_id: Option<Uuid>,
    pub customer_id: Option<String>,
    pub subscription_id: Option<String>,
    pub subscription_status: SubscriptionStatus,
    pub stripe_price_id: Option<String>,
    pub current_period_end: Option<DateTime<Utc>>,
    pub cancel_at_period_end: bool,
    pub billing_email: Option<String>,
    pub trial_ends_at: Option<DateTime<Utc>>,
    pub plan: OrganizationPlan,
    pub payload: Value,
}

#[derive(Debug, Clone)]
pub enum ParsedStripeWebhook {
    CheckoutCompleted(CheckoutCompletedEvent),
    SubscriptionUpdated(SubscriptionUpdatedEvent),
}

#[derive(Deserialize)]
struct StripeSessionResponse {
    url: String,
}

pub fn price_id_for_plan(config: &Config, plan: OrganizationPlan) -> Option<&str> {
    match plan {
        OrganizationPlan::Free => None,
        OrganizationPlan::Pro => config.stripe_price_pro.as_deref(),
        OrganizationPlan::Team => config.stripe_price_team.as_deref(),
    }
}

pub fn plan_from_price_id(config: &Config, price_id: &str) -> Option<OrganizationPlan> {
    if config.stripe_price_pro.as_deref() == Some(price_id) {
        Some(OrganizationPlan::Pro)
    } else if config.stripe_price_team.as_deref() == Some(price_id) {
        Some(OrganizationPlan::Team)
    } else {
        None
    }
}

pub fn verify_stripe_webhook_signature(
    webhook_secret: &str,
    signature_header: &str,
    payload: &str,
) -> Result<(), AppError> {
    let fields = parse_signature_header(signature_header)?;
    let timestamp = fields
        .get("t")
        .ok_or_else(|| AppError::Validation("Stripe signature timestamp is missing".to_string()))?;
    let timestamp: i64 = timestamp
        .parse()
        .map_err(|_| AppError::Validation("Stripe signature timestamp is invalid".to_string()))?;
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_| AppError::Internal(anyhow::anyhow!("System time is before UNIX_EPOCH")))?;
    let now = i64::try_from(now.as_secs())
        .map_err(|_| AppError::Internal(anyhow::anyhow!("System time overflow")))?;
    if (now - timestamp).abs() > STRIPE_WEBHOOK_TOLERANCE_SECS {
        return Err(AppError::Validation(
            "Stripe signature is outside the allowed tolerance".to_string(),
        ));
    }

    let expected = stripe_signature(webhook_secret, timestamp, payload)?;
    let signatures = fields
        .get("v1")
        .ok_or_else(|| AppError::Validation("Stripe v1 signature is missing".to_string()))?;

    if signatures
        .split_whitespace()
        .any(|candidate| candidate == expected)
    {
        Ok(())
    } else {
        Err(AppError::Validation(
            "Stripe signature verification failed".to_string(),
        ))
    }
}

pub fn parse_stripe_webhook(
    payload: &str,
    config: &Config,
) -> Result<Option<ParsedStripeWebhook>, AppError> {
    let event: Value = serde_json::from_str(payload).map_err(|error| {
        AppError::Validation(format!("Invalid Stripe webhook payload: {error}"))
    })?;

    let event_id = event
        .get("id")
        .and_then(Value::as_str)
        .ok_or_else(|| AppError::Validation("Stripe event id is missing".to_string()))?
        .to_string();
    let event_type = event
        .get("type")
        .and_then(Value::as_str)
        .ok_or_else(|| AppError::Validation("Stripe event type is missing".to_string()))?
        .to_string();
    let object = event
        .pointer("/data/object")
        .ok_or_else(|| AppError::Validation("Stripe event object is missing".to_string()))?;

    let parsed = match event_type.as_str() {
        "checkout.session.completed" => Some(ParsedStripeWebhook::CheckoutCompleted(
            CheckoutCompletedEvent {
                event_id,
                event_type,
                org_id: metadata_org_id(object)?,
                customer_id: string_field(object, "customer"),
                subscription_id: string_field(object, "subscription"),
                billing_email: object
                    .pointer("/customer_details/email")
                    .and_then(Value::as_str)
                    .map(ToOwned::to_owned),
                payload: event,
            },
        )),
        "customer.subscription.created"
        | "customer.subscription.updated"
        | "customer.subscription.deleted" => Some(ParsedStripeWebhook::SubscriptionUpdated(
            SubscriptionUpdatedEvent {
                event_id,
                event_type: event_type.clone(),
                org_id: metadata_org_id(object)?,
                customer_id: string_field(object, "customer"),
                subscription_id: string_field(object, "id"),
                subscription_status: subscription_status_for_event(&event_type, object),
                stripe_price_id: object
                    .pointer("/items/data/0/price/id")
                    .and_then(Value::as_str)
                    .map(ToOwned::to_owned),
                current_period_end: object
                    .get("current_period_end")
                    .and_then(Value::as_i64)
                    .and_then(timestamp_to_datetime),
                cancel_at_period_end: object
                    .get("cancel_at_period_end")
                    .and_then(Value::as_bool)
                    .unwrap_or(false),
                billing_email: object
                    .pointer("/metadata/billing_email")
                    .and_then(Value::as_str)
                    .map(ToOwned::to_owned),
                trial_ends_at: object
                    .get("trial_end")
                    .and_then(Value::as_i64)
                    .and_then(timestamp_to_datetime),
                plan: subscription_plan_for_event(&event_type, object, config)?,
                payload: event,
            },
        )),
        _ => None,
    };

    Ok(parsed)
}

pub async fn create_checkout_session(
    config: &Config,
    org: &Organization,
    customer_email: &str,
    plan: OrganizationPlan,
) -> Result<BillingSession, AppError> {
    let secret_key = config
        .stripe_secret_key
        .as_deref()
        .ok_or_else(|| AppError::Validation("Stripe billing is not configured".to_string()))?;
    let price_id = price_id_for_plan(config, plan).ok_or_else(|| {
        AppError::Validation("That upgrade is not configured for this deployment".to_string())
    })?;

    let settings_url = format!(
        "{}/dashboard/{}/settings",
        config.app_base_url.trim_end_matches('/'),
        org.slug
    );
    let mut params = vec![
        ("mode".to_string(), "subscription".to_string()),
        ("line_items[0][price]".to_string(), price_id.to_string()),
        ("line_items[0][quantity]".to_string(), "1".to_string()),
        (
            "success_url".to_string(),
            format!("{settings_url}?billing=success"),
        ),
        (
            "cancel_url".to_string(),
            format!("{settings_url}?billing=canceled"),
        ),
        ("allow_promotion_codes".to_string(), "true".to_string()),
        ("metadata[org_id]".to_string(), org.id.to_string()),
        ("metadata[org_slug]".to_string(), org.slug.clone()),
        ("metadata[plan]".to_string(), plan.to_string()),
        (
            "metadata[billing_email]".to_string(),
            customer_email.to_string(),
        ),
    ];

    if let Some(customer_id) = &org.stripe_customer_id {
        params.push(("customer".to_string(), customer_id.clone()));
    } else {
        params.push(("customer_email".to_string(), customer_email.to_string()));
    }

    let session = post_form(secret_key, "/checkout/sessions", &params).await?;
    Ok(BillingSession { url: session.url })
}

pub async fn create_portal_session(
    config: &Config,
    org: &Organization,
) -> Result<BillingSession, AppError> {
    let secret_key = config
        .stripe_secret_key
        .as_deref()
        .ok_or_else(|| AppError::Validation("Stripe billing is not configured".to_string()))?;
    let customer_id = org.stripe_customer_id.as_deref().ok_or_else(|| {
        AppError::Validation("This organization does not have a Stripe customer yet".to_string())
    })?;
    let return_url = format!(
        "{}/dashboard/{}/settings",
        config.app_base_url.trim_end_matches('/'),
        org.slug
    );
    let params = vec![
        ("customer".to_string(), customer_id.to_string()),
        ("return_url".to_string(), return_url),
    ];

    let session = post_form(secret_key, "/billing_portal/sessions", &params).await?;
    Ok(BillingSession { url: session.url })
}

pub async fn fetch_subscription_snapshot(
    config: &Config,
    org: &Organization,
) -> Result<SubscriptionUpdatedEvent, AppError> {
    let secret_key = config
        .stripe_secret_key
        .as_deref()
        .ok_or_else(|| AppError::Validation("Stripe billing is not configured".to_string()))?;
    let subscription_id = org.stripe_subscription_id.as_deref().ok_or_else(|| {
        AppError::Validation(
            "This organization does not have a Stripe subscription yet".to_string(),
        )
    })?;

    let object = get_json(secret_key, &format!("/subscriptions/{subscription_id}")).await?;

    Ok(SubscriptionUpdatedEvent {
        event_id: format!("manual-sync:{subscription_id}"),
        event_type: "customer.subscription.manual_sync".to_string(),
        org_id: Some(org.id),
        customer_id: string_field(&object, "customer"),
        subscription_id: string_field(&object, "id"),
        subscription_status: subscription_status_for_event(
            "customer.subscription.updated",
            &object,
        ),
        stripe_price_id: object
            .pointer("/items/data/0/price/id")
            .and_then(Value::as_str)
            .map(ToOwned::to_owned),
        current_period_end: object
            .get("current_period_end")
            .and_then(Value::as_i64)
            .and_then(timestamp_to_datetime),
        cancel_at_period_end: object
            .get("cancel_at_period_end")
            .and_then(Value::as_bool)
            .unwrap_or(false),
        billing_email: org.billing_email.clone(),
        trial_ends_at: object
            .get("trial_end")
            .and_then(Value::as_i64)
            .and_then(timestamp_to_datetime),
        plan: subscription_plan_for_event("customer.subscription.updated", &object, config)?,
        payload: object,
    })
}

async fn post_form(
    secret_key: &str,
    path: &str,
    params: &[(String, String)],
) -> Result<StripeSessionResponse, AppError> {
    let body = send_request(
        reqwest::Client::new()
            .post(format!("{STRIPE_API_BASE}{path}"))
            .basic_auth(secret_key, Some(""))
            .form(params),
    )
    .await?;

    serde_json::from_str(&body)
        .map_err(|error| AppError::Internal(anyhow::anyhow!("Invalid Stripe response: {error}")))
}

async fn get_json(secret_key: &str, path: &str) -> Result<Value, AppError> {
    let body = send_request(
        reqwest::Client::new()
            .get(format!("{STRIPE_API_BASE}{path}"))
            .basic_auth(secret_key, Some("")),
    )
    .await?;

    serde_json::from_str(&body)
        .map_err(|error| AppError::Internal(anyhow::anyhow!("Invalid Stripe response: {error}")))
}

async fn send_request(request: reqwest::RequestBuilder) -> Result<String, AppError> {
    let response = request
        .send()
        .await
        .map_err(|error| AppError::Internal(anyhow::anyhow!("Stripe request failed: {error}")))?;

    let status = response.status();
    let body = response.text().await.map_err(|error| {
        AppError::Internal(anyhow::anyhow!("Failed to read Stripe response: {error}"))
    })?;

    if !status.is_success() {
        return Err(AppError::Validation(stripe_error_message(status, &body)));
    }

    Ok(body)
}

fn stripe_error_message(status: StatusCode, body: &str) -> String {
    serde_json::from_str::<Value>(body)
        .ok()
        .and_then(|value| {
            value
                .pointer("/error/message")
                .and_then(Value::as_str)
                .map(ToOwned::to_owned)
        })
        .unwrap_or_else(|| format!("Stripe request failed with status {}", status.as_u16()))
}

fn metadata_org_id(object: &Value) -> Result<Option<Uuid>, AppError> {
    let Some(raw_id) = object.pointer("/metadata/org_id").and_then(Value::as_str) else {
        return Ok(None);
    };

    Uuid::parse_str(raw_id)
        .map(Some)
        .map_err(|_| AppError::Validation("Stripe metadata org_id is invalid".to_string()))
}

fn subscription_status_for_event(event_type: &str, object: &Value) -> SubscriptionStatus {
    if event_type == "customer.subscription.deleted" {
        SubscriptionStatus::Canceled
    } else {
        SubscriptionStatus::from_stripe(
            object
                .get("status")
                .and_then(Value::as_str)
                .unwrap_or("inactive"),
        )
    }
}

fn subscription_plan_for_event(
    event_type: &str,
    object: &Value,
    config: &Config,
) -> Result<OrganizationPlan, AppError> {
    if event_type == "customer.subscription.deleted" {
        return Ok(OrganizationPlan::Free);
    }

    let Some(price_id) = object
        .pointer("/items/data/0/price/id")
        .and_then(Value::as_str)
    else {
        return Ok(OrganizationPlan::Free);
    };

    plan_from_price_id(config, price_id).ok_or_else(|| {
        AppError::Validation(format!(
            "Stripe price '{price_id}' is not mapped to a managed beta plan"
        ))
    })
}

fn string_field(object: &Value, key: &str) -> Option<String> {
    object
        .get(key)
        .and_then(Value::as_str)
        .map(ToOwned::to_owned)
}

fn timestamp_to_datetime(timestamp: i64) -> Option<DateTime<Utc>> {
    DateTime::from_timestamp(timestamp, 0)
}

fn parse_signature_header(signature_header: &str) -> Result<HashMap<String, String>, AppError> {
    let mut parsed = HashMap::new();
    for part in signature_header.split(',') {
        let mut segments = part.trim().splitn(2, '=');
        let key = segments.next().unwrap_or_default().trim();
        let value = segments.next().unwrap_or_default().trim();
        if !key.is_empty() && !value.is_empty() {
            parsed
                .entry(key.to_string())
                .and_modify(|existing: &mut String| {
                    existing.push(' ');
                    existing.push_str(value);
                })
                .or_insert_with(|| value.to_string());
        }
    }

    if parsed.is_empty() {
        return Err(AppError::Validation(
            "Stripe signature header is malformed".to_string(),
        ));
    }

    Ok(parsed)
}

fn stripe_signature(
    webhook_secret: &str,
    timestamp: i64,
    payload: &str,
) -> Result<String, AppError> {
    let mut mac = HmacSha256::new_from_slice(webhook_secret.as_bytes())
        .map_err(|error| AppError::Internal(anyhow::anyhow!("Failed to build HMAC: {error}")))?;
    mac.update(format!("{timestamp}.{payload}").as_bytes());
    Ok(hex::encode(mac.finalize().into_bytes()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn managed_config() -> Config {
        Config {
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
    fn plan_mapping_uses_configured_price_ids() {
        let config = managed_config();
        assert_eq!(price_id_for_plan(&config, OrganizationPlan::Free), None);
        assert_eq!(
            price_id_for_plan(&config, OrganizationPlan::Pro),
            Some("price_pro")
        );
        assert_eq!(
            plan_from_price_id(&config, "price_team"),
            Some(OrganizationPlan::Team)
        );
    }

    #[test]
    fn verify_stripe_webhook_signature_accepts_valid_header() {
        let payload = r#"{"id":"evt_123","type":"checkout.session.completed"}"#;
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;
        let signature = stripe_signature("whsec_test", timestamp, payload).unwrap();
        let header = format!("t={timestamp},v1={signature}");

        let result = verify_stripe_webhook_signature("whsec_test", &header, payload);
        assert!(result.is_ok());
    }

    #[test]
    fn verify_stripe_webhook_signature_rejects_invalid_signature() {
        let payload = r#"{"id":"evt_123","type":"checkout.session.completed"}"#;
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;
        let header = format!("t={timestamp},v1=bad_signature");

        let result = verify_stripe_webhook_signature("whsec_test", &header, payload);
        assert!(result.is_err());
    }

    #[test]
    fn parse_stripe_webhook_maps_subscription_updates() {
        let config = managed_config();
        let payload = serde_json::json!({
            "id": "evt_123",
            "type": "customer.subscription.updated",
            "data": {
                "object": {
                    "id": "sub_123",
                    "customer": "cus_123",
                    "status": "active",
                    "cancel_at_period_end": false,
                    "current_period_end": 1_700_000_000_i64,
                    "metadata": {
                        "org_id": Uuid::nil().to_string(),
                    },
                    "items": {
                        "data": [
                            {
                                "price": {
                                    "id": "price_pro"
                                }
                            }
                        ]
                    }
                }
            }
        })
        .to_string();

        let parsed = parse_stripe_webhook(&payload, &config).unwrap();
        match parsed {
            Some(ParsedStripeWebhook::SubscriptionUpdated(event)) => {
                assert_eq!(event.plan, OrganizationPlan::Pro);
                assert_eq!(event.subscription_status, SubscriptionStatus::Active);
                assert_eq!(event.customer_id.as_deref(), Some("cus_123"));
            }
            _ => panic!("expected subscription update"),
        }
    }
}
