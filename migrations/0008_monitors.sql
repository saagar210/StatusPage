CREATE TABLE monitors (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    service_id UUID NOT NULL REFERENCES services(id) ON DELETE CASCADE,
    org_id UUID NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    monitor_type VARCHAR(20) NOT NULL
        CHECK (monitor_type IN ('http', 'tcp', 'dns', 'ping')),
    config JSONB NOT NULL DEFAULT '{}',
    interval_seconds INT NOT NULL DEFAULT 60
        CHECK (interval_seconds >= 30 AND interval_seconds <= 300),
    timeout_ms INT NOT NULL DEFAULT 10000
        CHECK (timeout_ms >= 1000 AND timeout_ms <= 30000),
    failure_threshold INT NOT NULL DEFAULT 3
        CHECK (failure_threshold >= 1 AND failure_threshold <= 10),
    is_active BOOLEAN NOT NULL DEFAULT true,
    consecutive_failures INT NOT NULL DEFAULT 0,
    last_checked_at TIMESTAMPTZ,
    last_response_time_ms INT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_monitors_org ON monitors(org_id);
CREATE INDEX idx_monitors_active ON monitors(id) WHERE is_active = true;
CREATE INDEX idx_monitors_service ON monitors(service_id);
