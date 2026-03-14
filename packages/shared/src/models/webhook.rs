use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::enums::DisabledReason;

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct WebhookConfig {
    pub id: Uuid,
    pub org_id: Uuid,
    pub name: String,
    pub url: String,
    pub event_types: Vec<String>,
    pub is_enabled: bool,
    pub disabled_reason: Option<DisabledReason>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct CreateWebhookConfigRequest {
    pub name: String,
    pub url: String,
    pub secret: String,
    pub event_types: Vec<String>,
    pub is_enabled: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateWebhookConfigRequest {
    pub name: Option<String>,
    pub url: Option<String>,
    pub secret: Option<String>,
    pub event_types: Option<Vec<String>>,
    pub is_enabled: Option<bool>,
}
