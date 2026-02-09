CREATE TABLE monitor_checks (
    id BIGSERIAL,
    monitor_id UUID NOT NULL,
    status VARCHAR(10) NOT NULL CHECK (status IN ('success', 'failure', 'timeout')),
    response_time_ms INT,
    status_code INT,
    error_message TEXT,
    checked_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (id, checked_at)
) PARTITION BY RANGE (checked_at);

-- Default partition to catch any inserts that don't match a range partition
CREATE TABLE monitor_checks_default PARTITION OF monitor_checks DEFAULT;

CREATE INDEX idx_monitor_checks_monitor_time ON monitor_checks(monitor_id, checked_at DESC);
