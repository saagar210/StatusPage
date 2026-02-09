use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "VARCHAR", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum ServiceStatus {
    Operational,
    DegradedPerformance,
    PartialOutage,
    MajorOutage,
    UnderMaintenance,
}

impl ServiceStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Operational => "operational",
            Self::DegradedPerformance => "degraded_performance",
            Self::PartialOutage => "partial_outage",
            Self::MajorOutage => "major_outage",
            Self::UnderMaintenance => "under_maintenance",
        }
    }
}

impl fmt::Display for ServiceStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Operational => write!(f, "Operational"),
            Self::DegradedPerformance => write!(f, "Degraded Performance"),
            Self::PartialOutage => write!(f, "Partial Outage"),
            Self::MajorOutage => write!(f, "Major Outage"),
            Self::UnderMaintenance => write!(f, "Under Maintenance"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "VARCHAR", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum IncidentStatus {
    Investigating,
    Identified,
    Monitoring,
    Resolved,
}

impl IncidentStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Investigating => "investigating",
            Self::Identified => "identified",
            Self::Monitoring => "monitoring",
            Self::Resolved => "resolved",
        }
    }

    /// Check if transitioning from `self` to `to` is valid.
    pub fn can_transition_to(&self, to: &Self) -> bool {
        matches!(
            (self, to),
            (Self::Investigating, Self::Identified)
                | (Self::Investigating, Self::Monitoring)
                | (Self::Investigating, Self::Resolved)
                | (Self::Identified, Self::Monitoring)
                | (Self::Identified, Self::Resolved)
                | (Self::Monitoring, Self::Resolved)
                | (Self::Resolved, Self::Investigating) // reopen
                | (_, Self::Investigating) // any → investigating (reopen)
        )
    }
}

impl fmt::Display for IncidentStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Investigating => write!(f, "Investigating"),
            Self::Identified => write!(f, "Identified"),
            Self::Monitoring => write!(f, "Monitoring"),
            Self::Resolved => write!(f, "Resolved"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "VARCHAR", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum IncidentImpact {
    None,
    Minor,
    Major,
    Critical,
}

impl IncidentImpact {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::None => "none",
            Self::Minor => "minor",
            Self::Major => "major",
            Self::Critical => "critical",
        }
    }

    /// Map incident impact to a service status for cascading.
    pub fn to_service_status(&self) -> ServiceStatus {
        match self {
            Self::Critical | Self::Major => ServiceStatus::MajorOutage,
            Self::Minor => ServiceStatus::DegradedPerformance,
            Self::None => ServiceStatus::Operational,
        }
    }
}

impl fmt::Display for IncidentImpact {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::None => write!(f, "None"),
            Self::Minor => write!(f, "Minor"),
            Self::Major => write!(f, "Major"),
            Self::Critical => write!(f, "Critical"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "VARCHAR", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum MemberRole {
    Owner,
    Admin,
    Member,
}

impl MemberRole {
    pub fn is_admin_or_above(&self) -> bool {
        matches!(self, Self::Owner | Self::Admin)
    }
}

impl fmt::Display for MemberRole {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Owner => write!(f, "Owner"),
            Self::Admin => write!(f, "Admin"),
            Self::Member => write!(f, "Member"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "VARCHAR", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum MonitorType {
    Http,
    Tcp,
    Dns,
    Ping,
}

impl fmt::Display for MonitorType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Http => write!(f, "HTTP"),
            Self::Tcp => write!(f, "TCP"),
            Self::Dns => write!(f, "DNS"),
            Self::Ping => write!(f, "Ping"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "VARCHAR", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum CheckStatus {
    Success,
    Failure,
    Timeout,
}

impl fmt::Display for CheckStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Success => write!(f, "Success"),
            Self::Failure => write!(f, "Failure"),
            Self::Timeout => write!(f, "Timeout"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_service_status_serialization() {
        let status = ServiceStatus::DegradedPerformance;
        let json = serde_json::to_string(&status).unwrap();
        assert_eq!(json, r#""degraded_performance""#);
        let deserialized: ServiceStatus = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, status);
    }

    #[test]
    fn test_incident_status_transitions() {
        assert!(IncidentStatus::Investigating.can_transition_to(&IncidentStatus::Identified));
        assert!(IncidentStatus::Investigating.can_transition_to(&IncidentStatus::Resolved));
        assert!(IncidentStatus::Resolved.can_transition_to(&IncidentStatus::Investigating));
        // resolved → identified goes through investigating
        assert!(IncidentStatus::Monitoring.can_transition_to(&IncidentStatus::Resolved));
    }

    #[test]
    fn test_impact_to_service_status() {
        assert_eq!(
            IncidentImpact::Critical.to_service_status(),
            ServiceStatus::MajorOutage
        );
        assert_eq!(
            IncidentImpact::Minor.to_service_status(),
            ServiceStatus::DegradedPerformance
        );
        assert_eq!(
            IncidentImpact::None.to_service_status(),
            ServiceStatus::Operational
        );
    }

    #[test]
    fn test_member_role_admin_check() {
        assert!(MemberRole::Owner.is_admin_or_above());
        assert!(MemberRole::Admin.is_admin_or_above());
        assert!(!MemberRole::Member.is_admin_or_above());
    }
}
