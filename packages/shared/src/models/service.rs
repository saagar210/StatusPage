use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::enums::ServiceStatus;

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Service {
    pub id: Uuid,
    pub org_id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub current_status: ServiceStatus,
    pub display_order: i32,
    pub group_name: Option<String>,
    pub is_visible: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct CreateServiceRequest {
    pub name: String,
    pub description: Option<String>,
    pub group_name: Option<String>,
    pub is_visible: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateServiceRequest {
    pub name: Option<String>,
    pub description: Option<String>,
    pub current_status: Option<ServiceStatus>,
    pub group_name: Option<String>,
    pub is_visible: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct ReorderServicesRequest {
    pub service_ids: Vec<Uuid>,
}
