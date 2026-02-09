CREATE TABLE uptime_daily (
    monitor_id UUID NOT NULL,
    date DATE NOT NULL,
    total_checks INT NOT NULL DEFAULT 0,
    successful_checks INT NOT NULL DEFAULT 0,
    avg_response_time_ms FLOAT,
    min_response_time_ms INT,
    max_response_time_ms INT,
    uptime_percentage FLOAT GENERATED ALWAYS AS (
        CASE WHEN total_checks > 0 THEN (successful_checks::FLOAT / total_checks) * 100 ELSE NULL END
    ) STORED,
    PRIMARY KEY (monitor_id, date)
);
