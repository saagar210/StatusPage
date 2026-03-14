ALTER TABLE organizations
ADD COLUMN IF NOT EXISTS downgrade_target_plan VARCHAR(16)
    CHECK (downgrade_target_plan IN ('free', 'pro', 'team')),
ADD COLUMN IF NOT EXISTS downgrade_started_at TIMESTAMPTZ,
ADD COLUMN IF NOT EXISTS downgrade_grace_ends_at TIMESTAMPTZ,
ADD COLUMN IF NOT EXISTS downgrade_state VARCHAR(32) NOT NULL DEFAULT 'none'
    CHECK (downgrade_state IN ('none', 'pending_customer_action', 'ready_to_enforce', 'enforced', 'canceled')),
ADD COLUMN IF NOT EXISTS downgrade_warning_stage INT NOT NULL DEFAULT 0
    CHECK (downgrade_warning_stage BETWEEN 0 AND 4),
ADD COLUMN IF NOT EXISTS custom_domain_status VARCHAR(32) NOT NULL DEFAULT 'not_configured'
    CHECK (custom_domain_status IN ('not_configured', 'pending_verification', 'verified', 'blocked_by_plan'));

UPDATE organizations
SET custom_domain_status = CASE
    WHEN custom_domain IS NULL OR btrim(custom_domain) = '' THEN 'not_configured'
    WHEN custom_domain_verified_at IS NOT NULL THEN 'verified'
    ELSE 'pending_verification'
END
WHERE custom_domain_status = 'not_configured';

CREATE INDEX IF NOT EXISTS idx_organizations_downgrade_due
ON organizations (downgrade_grace_ends_at)
WHERE downgrade_state IN ('pending_customer_action', 'ready_to_enforce');

ALTER TABLE monitors
ADD COLUMN IF NOT EXISTS disabled_reason VARCHAR(32)
    CHECK (disabled_reason IN ('plan_limit'));

CREATE INDEX IF NOT EXISTS idx_monitors_disabled_reason
ON monitors (org_id, disabled_reason)
WHERE disabled_reason IS NOT NULL;

ALTER TABLE webhook_configs
ADD COLUMN IF NOT EXISTS disabled_reason VARCHAR(32)
    CHECK (disabled_reason IN ('plan_limit'));

CREATE INDEX IF NOT EXISTS idx_webhook_configs_disabled_reason
ON webhook_configs (org_id, disabled_reason)
WHERE disabled_reason IS NOT NULL;

ALTER TABLE invitations
ADD COLUMN IF NOT EXISTS canceled_at TIMESTAMPTZ,
ADD COLUMN IF NOT EXISTS last_sent_at TIMESTAMPTZ;

DROP INDEX IF EXISTS invitations_org_email_active_idx;
CREATE UNIQUE INDEX IF NOT EXISTS invitations_org_email_active_idx
ON invitations (org_id, lower(email))
WHERE accepted_at IS NULL AND canceled_at IS NULL;
