ALTER TABLE organizations
    ADD COLUMN IF NOT EXISTS stripe_subscription_id VARCHAR(255),
    ADD COLUMN IF NOT EXISTS subscription_status VARCHAR(32) NOT NULL DEFAULT 'inactive'
        CHECK (
            subscription_status IN (
                'inactive',
                'checkout_pending',
                'trialing',
                'active',
                'past_due',
                'canceled',
                'unpaid',
                'incomplete',
                'incomplete_expired'
            )
        ),
    ADD COLUMN IF NOT EXISTS stripe_price_id VARCHAR(255),
    ADD COLUMN IF NOT EXISTS current_period_end TIMESTAMPTZ,
    ADD COLUMN IF NOT EXISTS cancel_at_period_end BOOLEAN NOT NULL DEFAULT FALSE,
    ADD COLUMN IF NOT EXISTS billing_email VARCHAR(255),
    ADD COLUMN IF NOT EXISTS trial_ends_at TIMESTAMPTZ;

CREATE TABLE IF NOT EXISTS billing_events (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    stripe_event_id VARCHAR(255) NOT NULL UNIQUE,
    event_type VARCHAR(100) NOT NULL,
    org_id UUID REFERENCES organizations(id) ON DELETE SET NULL,
    payload JSONB NOT NULL,
    processed_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_billing_events_org_id
    ON billing_events(org_id)
    WHERE org_id IS NOT NULL;

CREATE INDEX IF NOT EXISTS idx_organizations_stripe_subscription_id
    ON organizations(stripe_subscription_id)
    WHERE stripe_subscription_id IS NOT NULL;

