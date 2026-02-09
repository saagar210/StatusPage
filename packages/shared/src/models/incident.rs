use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::enums::{IncidentImpact, IncidentStatus};
use crate::models::incident_update::IncidentUpdate;

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Incident {
    pub id: Uuid,
    pub org_id: Uuid,
    pub title: String,
    pub status: IncidentStatus,
    pub impact: IncidentImpact,
    pub is_auto: bool,
    pub started_at: DateTime<Utc>,
    pub resolved_at: Option<DateTime<Utc>>,
    pub created_by: Option<Uuid>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize)]
pub struct IncidentWithDetails {
    #[serde(flatten)]
    pub incident: Incident,
    pub updates: Vec<IncidentUpdate>,
    pub affected_services: Vec<AffectedService>,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct AffectedService {
    pub service_id: Uuid,
    pub service_name: String,
}

#[derive(Debug, Deserialize)]
pub struct CreateIncidentRequest {
    pub title: String,
    pub status: Option<IncidentStatus>,
    pub impact: IncidentImpact,
    pub message: String,
    pub affected_service_ids: Vec<Uuid>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateIncidentRequest {
    pub title: Option<String>,
    pub status: Option<IncidentStatus>,
    pub impact: Option<IncidentImpact>,
}
