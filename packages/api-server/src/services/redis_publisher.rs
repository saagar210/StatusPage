use redis::aio::ConnectionManager;
use redis::AsyncCommands;
use serde::Serialize;
use uuid::Uuid;

use shared::enums::{IncidentStatus, ServiceStatus};

/// Real-time event publisher using Redis pub/sub
///
/// Channel format: org:{org_id}:{event_type}
/// - org:{org_id}:service:status - Service status changes
/// - org:{org_id}:incident:created - New incidents
/// - org:{org_id}:incident:updated - Incident timeline updates

#[derive(Clone)]
pub struct RedisPublisher {
    redis: ConnectionManager,
}

#[derive(Serialize)]
pub struct ServiceStatusEvent {
    pub service_id: Uuid,
    pub service_name: String,
    pub old_status: ServiceStatus,
    pub new_status: ServiceStatus,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

#[derive(Serialize)]
pub struct IncidentCreatedEvent {
    pub incident_id: Uuid,
    pub title: String,
    pub status: IncidentStatus,
    pub impact: String,
    pub affected_services: Vec<Uuid>,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

#[derive(Serialize)]
pub struct IncidentUpdatedEvent {
    pub incident_id: Uuid,
    pub update_id: Uuid,
    pub status: IncidentStatus,
    pub message: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

impl RedisPublisher {
    pub fn new(redis: ConnectionManager) -> Self {
        Self { redis }
    }

    /// Publish service status change event
    pub async fn publish_service_status_change(
        &self,
        org_id: Uuid,
        event: ServiceStatusEvent,
    ) -> Result<(), redis::RedisError> {
        let channel = format!("org:{}:service:status", org_id);
        let payload = serde_json::to_string(&event).unwrap_or_default();

        let mut conn = self.redis.clone();
        conn.publish::<_, _, ()>(channel, payload).await?;

        tracing::debug!(
            "Published service status change: {} -> {}",
            event.service_name,
            event.new_status
        );

        Ok(())
    }

    /// Publish incident created event
    pub async fn publish_incident_created(
        &self,
        org_id: Uuid,
        event: IncidentCreatedEvent,
    ) -> Result<(), redis::RedisError> {
        let channel = format!("org:{}:incident:created", org_id);
        let payload = serde_json::to_string(&event).unwrap_or_default();

        let mut conn = self.redis.clone();
        conn.publish::<_, _, ()>(channel, payload).await?;

        tracing::info!(
            "Published incident created: {} (id: {})",
            event.title,
            event.incident_id
        );

        Ok(())
    }

    /// Publish incident updated event
    pub async fn publish_incident_updated(
        &self,
        org_id: Uuid,
        event: IncidentUpdatedEvent,
    ) -> Result<(), redis::RedisError> {
        let channel = format!("org:{}:incident:updated", org_id);
        let payload = serde_json::to_string(&event).unwrap_or_default();

        let mut conn = self.redis.clone();
        conn.publish::<_, _, ()>(channel, payload).await?;

        tracing::debug!(
            "Published incident update: {} (status: {})",
            event.incident_id,
            event.status
        );

        Ok(())
    }

    /// Publish generic event to organization channel
    pub async fn publish_event(
        &self,
        org_id: Uuid,
        event_type: &str,
        payload: serde_json::Value,
    ) -> Result<(), redis::RedisError> {
        let channel = format!("org:{}:{}", org_id, event_type);
        let payload_str = serde_json::to_string(&payload).unwrap_or_default();

        let mut conn = self.redis.clone();
        conn.publish::<_, _, ()>(channel, payload_str).await?;

        tracing::debug!("Published event: {} to org: {}", event_type, org_id);

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_service_status_event_serialization() {
        let event = ServiceStatusEvent {
            service_id: Uuid::new_v4(),
            service_name: "API Service".to_string(),
            old_status: ServiceStatus::Operational,
            new_status: ServiceStatus::DegradedPerformance,
            timestamp: chrono::Utc::now(),
        };

        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("API Service"));
        assert!(json.contains("operational"));
        assert!(json.contains("degraded"));
    }

    #[test]
    fn test_incident_created_event_serialization() {
        let event = IncidentCreatedEvent {
            incident_id: Uuid::new_v4(),
            title: "Database Outage".to_string(),
            status: IncidentStatus::Investigating,
            impact: "critical".to_string(),
            affected_services: vec![Uuid::new_v4()],
            timestamp: chrono::Utc::now(),
        };

        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("Database Outage"));
        assert!(json.contains("investigating"));
    }
}
