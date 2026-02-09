CREATE TABLE services (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    org_id UUID NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    name VARCHAR(255) NOT NULL,
    description TEXT,
    current_status VARCHAR(30) NOT NULL DEFAULT 'operational'
        CHECK (current_status IN ('operational', 'degraded_performance', 'partial_outage', 'major_outage', 'under_maintenance')),
    display_order INT NOT NULL DEFAULT 0,
    group_name VARCHAR(255),
    is_visible BOOLEAN NOT NULL DEFAULT true,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_services_org_order ON services(org_id, display_order);
