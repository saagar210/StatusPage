-- Create notification_preferences table for organization-level settings
CREATE TABLE IF NOT EXISTS notification_preferences (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    org_id UUID NOT NULL REFERENCES organizations(id) ON DELETE CASCADE UNIQUE,
    email_on_incident_created BOOLEAN NOT NULL DEFAULT TRUE,
    email_on_incident_updated BOOLEAN NOT NULL DEFAULT TRUE,
    email_on_incident_resolved BOOLEAN NOT NULL DEFAULT TRUE,
    email_on_service_status_changed BOOLEAN NOT NULL DEFAULT FALSE,
    webhook_on_incident_created BOOLEAN NOT NULL DEFAULT TRUE,
    webhook_on_incident_updated BOOLEAN NOT NULL DEFAULT TRUE,
    webhook_on_incident_resolved BOOLEAN NOT NULL DEFAULT TRUE,
    webhook_on_service_status_changed BOOLEAN NOT NULL DEFAULT TRUE,
    uptime_alert_threshold NUMERIC(5, 2) DEFAULT 95.0, -- Alert if uptime < 95%
    uptime_alert_enabled BOOLEAN NOT NULL DEFAULT TRUE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_notification_preferences_org_id ON notification_preferences(org_id);
