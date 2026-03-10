use chrono::Utc;
use redis::aio::ConnectionManager;
use redis::AsyncCommands;
use serde::Serialize;
use shared::enums::{IncidentStatus, ServiceStatus};
use uuid::Uuid;

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

    pub async fn publish_service_status_change(
        &self,
        org_id: Uuid,
        event: ServiceStatusEvent,
    ) -> anyhow::Result<()> {
        let channel = format!("org:{}:service:status", org_id);
        let payload = serde_json::to_string(&event)?;
        let mut conn = self.redis.clone();
        conn.publish::<_, _, ()>(channel, payload).await?;
        Ok(())
    }

    pub async fn publish_incident_created(
        &self,
        org_id: Uuid,
        event: IncidentCreatedEvent,
    ) -> anyhow::Result<()> {
        let channel = format!("org:{}:incident:created", org_id);
        let payload = serde_json::to_string(&event)?;
        let mut conn = self.redis.clone();
        conn.publish::<_, _, ()>(channel, payload).await?;
        Ok(())
    }

    pub async fn publish_incident_updated(
        &self,
        org_id: Uuid,
        event: IncidentUpdatedEvent,
    ) -> anyhow::Result<()> {
        let channel = format!("org:{}:incident:updated", org_id);
        let payload = serde_json::to_string(&event)?;
        let mut conn = self.redis.clone();
        conn.publish::<_, _, ()>(channel, payload).await?;
        Ok(())
    }

    pub fn service_status_event(
        service_id: Uuid,
        service_name: String,
        old_status: ServiceStatus,
        new_status: ServiceStatus,
    ) -> ServiceStatusEvent {
        ServiceStatusEvent {
            service_id,
            service_name,
            old_status,
            new_status,
            timestamp: Utc::now(),
        }
    }

    pub fn incident_created_event(
        incident_id: Uuid,
        title: String,
        status: IncidentStatus,
        impact: String,
        affected_services: Vec<Uuid>,
    ) -> IncidentCreatedEvent {
        IncidentCreatedEvent {
            incident_id,
            title,
            status,
            impact,
            affected_services,
            timestamp: Utc::now(),
        }
    }

    pub fn incident_updated_event(
        incident_id: Uuid,
        update_id: Uuid,
        status: IncidentStatus,
        message: String,
    ) -> IncidentUpdatedEvent {
        IncidentUpdatedEvent {
            incident_id,
            update_id,
            status,
            message,
            timestamp: Utc::now(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serializes_service_status_event_with_expected_status_names() {
        let event = RedisPublisher::service_status_event(
            Uuid::new_v4(),
            "API".to_string(),
            ServiceStatus::Operational,
            ServiceStatus::MajorOutage,
        );

        let json = serde_json::to_string(&event).expect("serialize service status event");
        assert!(json.contains("\"old_status\":\"operational\""));
        assert!(json.contains("\"new_status\":\"major_outage\""));
    }

    #[test]
    fn serializes_incident_update_event_with_expected_status_name() {
        let event = RedisPublisher::incident_updated_event(
            Uuid::new_v4(),
            Uuid::new_v4(),
            IncidentStatus::Resolved,
            "Recovered".to_string(),
        );

        let json = serde_json::to_string(&event).expect("serialize incident updated event");
        assert!(json.contains("\"status\":\"resolved\""));
        assert!(json.contains("\"message\":\"Recovered\""));
    }
}
