use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct NotificationPreferences {
    pub id: Uuid,
    pub org_id: Uuid,
    pub email_on_incident_created: bool,
    pub email_on_incident_updated: bool,
    pub email_on_incident_resolved: bool,
    pub email_on_service_status_changed: bool,
    pub webhook_on_incident_created: bool,
    pub webhook_on_incident_updated: bool,
    pub webhook_on_incident_resolved: bool,
    pub webhook_on_service_status_changed: bool,
    pub uptime_alert_threshold: Option<f64>,
    pub uptime_alert_enabled: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateNotificationPreferencesRequest {
    pub email_on_incident_created: Option<bool>,
    pub email_on_incident_updated: Option<bool>,
    pub email_on_incident_resolved: Option<bool>,
    pub email_on_service_status_changed: Option<bool>,
    pub webhook_on_incident_created: Option<bool>,
    pub webhook_on_incident_updated: Option<bool>,
    pub webhook_on_incident_resolved: Option<bool>,
    pub webhook_on_service_status_changed: Option<bool>,
    pub uptime_alert_threshold: Option<f64>,
    pub uptime_alert_enabled: Option<bool>,
}
