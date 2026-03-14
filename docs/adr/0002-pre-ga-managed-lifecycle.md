# ADR 0002: Pre-GA Managed Lifecycle

## Status

Accepted

## Context

The managed beta already supported billing upgrades, invitations, custom-domain verification, and internal operator actions. Pre-GA readiness adds a new requirement: the product must handle plan downgrades, invitation delivery, and operator recovery workflows without relying on manual SQL changes or ad hoc support playbooks.

## Decision

- Keep GitHub-only customer auth for this phase.
- Keep the token-gated `INTERNAL_ADMIN_TOKEN` operator model for this phase.
- Introduce a non-destructive downgrade lifecycle with:
  - durable downgrade target and grace-period state on organizations
  - a 14-day grace window
  - warning emails before enforcement
  - automatic enforcement that disables excess monitors, blocks custom domains, and disables outbound webhooks without deleting configuration
- Treat invite delivery as a first-class queued product email flow.
- Expand the internal support console and admin API around searchable org support, billing sync, resend, and downgrade intervention actions.

## Consequences

- Billing webhook sync becomes lifecycle-aware instead of immediately removing paid-only features.
- Managed support flows become more repeatable and auditable.
- The product is closer to pre-GA operational maturity, but still depends on live Stripe and hosted DNS validation for final external proof.
