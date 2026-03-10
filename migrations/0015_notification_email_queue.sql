ALTER TABLE notification_logs
ADD COLUMN IF NOT EXISTS body_text TEXT NOT NULL DEFAULT '',
ADD COLUMN IF NOT EXISTS attempt_count INT NOT NULL DEFAULT 0,
ADD COLUMN IF NOT EXISTS max_attempts INT NOT NULL DEFAULT 5,
ADD COLUMN IF NOT EXISTS next_retry_at TIMESTAMPTZ;

CREATE INDEX IF NOT EXISTS idx_notification_logs_retry
ON notification_logs(next_retry_at)
WHERE status = 'pending' AND next_retry_at IS NOT NULL;
