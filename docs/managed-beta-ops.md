# Managed Beta Ops

This runbook covers the current operator surface for the managed paid beta.

## Required environment

- `INTERNAL_ADMIN_TOKEN` must be set on the API server.
- Send the token as `x-statuspage-admin-token` when calling internal support endpoints.

## Support endpoints

- `GET /api/admin/queue-health`
  - Global queue counts for email, webhook, invitation email, downgrade-warning email, orgs in grace, and recent billing events.
- `GET /api/admin/organizations/search?q=...`
  - Search orgs by slug, name, billing email, Stripe customer ID, or Stripe subscription ID.
- `GET /api/admin/organizations/:slug/support`
  - Org plan, subscription state, downgrade state, custom-domain state, entitlement violations, required actions, invitation lifecycle, member counts, subscriber count, webhook count, recent billing events, recent audit logs, and the newest failed deliveries.
- `POST /api/admin/organizations/:slug/billing/sync`
  - Pull the current Stripe subscription into the app and re-apply billing lifecycle rules.
- `POST /api/admin/organizations/:slug/downgrade/enforce`
  - Force non-destructive downgrade enforcement immediately.
- `POST /api/admin/organizations/:slug/downgrade/cancel`
  - Cancel a pending downgrade and restore plan-limited features.
- `POST /api/admin/organizations/:slug/invitations/:id/resend`
  - Requeue invitation email delivery for a pending invite.
- `POST /api/admin/organizations/:slug/retry/email/:id`
  - Requeue a failed email delivery.
- `POST /api/admin/organizations/:slug/retry/webhook/:id`
  - Requeue a failed webhook delivery.

## Recommended support workflow

1. Check `/api/admin/queue-health` for broad queue pressure or billing-event spikes.
2. Look up the customer org with `/api/admin/organizations/:slug/support`.
3. Confirm plan, subscription status, billing email, Stripe identifiers, and custom-domain verification state.
4. Inspect recent billing events to see whether Stripe webhooks are landing and whether downgrade state matches the expected customer plan.
5. Inspect recent audit logs to confirm whether invites, webhook edits, billing syncs, downgrade actions, or custom-domain checks happened as expected.
6. Retry failed email or webhook deliveries only after confirming the underlying config is fixed.
7. If the customer reports a billing mismatch, run billing sync first, then force or cancel downgrade only if the state is still incorrect.

## Internal dashboard console

- `/dashboard/internal-support` provides a browser-based operator view over the internal support endpoints.
- Operators still need the same `INTERNAL_ADMIN_TOKEN`; the UI forwards it as the internal support header through the authenticated proxy.

## Downgrade lifecycle note

- Self-serve upgrades and billing portal management are live.
- Downgrades now enter a 14-day grace period before lower-plan limits are enforced.
- Enforcement is non-destructive:
  - excess newest monitors are disabled
  - custom domains are blocked by plan but preserved
  - outbound webhooks are disabled but preserved
