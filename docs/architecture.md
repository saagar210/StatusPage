# Architecture Overview

## Runtime services

- `apps/web` serves the dashboard, marketing site, public status pages, and the authenticated proxy layer.
- `packages/api-server` owns the primary API, organization access rules, incident/service/monitor workflows, webhook delivery, email queue dispatch, and managed-beta billing sync with Stripe.
- `apps/monitor` executes monitor checks, writes uptime data, auto-creates or resolves incidents, and now also emits realtime plus notification events.
- PostgreSQL is the system of record.
- Redis is used for realtime fanout and health validation.

For local development, the repo defaults to higher host ports for PostgreSQL and
Redis so the app does not accidentally connect to an unrelated service already
running on `5432` or `6379`.

## Key product flows

### Dashboard administration

- Authenticated users sign in through Auth.js and GitHub OAuth.
- Organization access is enforced by `OrgAccess`, which resolves membership and role for the requested org.
- Admin-only settings now cover organization branding fields, managed-beta billing state, team-member management, notification preferences, subscribers, delivery activity, and webhooks.
- Paid plans are enforced in the API for monitor limits, custom domains, and outbound webhooks.
- Teammate growth now uses invitations, and acceptance requires the invited email to match the signed-in GitHub account.

### Public status pages

- Public status, history, uptime, subscribe, verify, and unsubscribe routes are served through the Rust API and rendered in the web app.
- Organizations with a configured custom domain can serve their public experience directly from `/`, `/history`, `/verify`, and `/unsubscribe` when the incoming host matches.
- Subscriber emails are queued in PostgreSQL and dispatched asynchronously.
- Public subscribe, verify, unsubscribe, and invitation-accept flows now use Redis-backed rate limiting with an in-memory fallback if Redis is temporarily unavailable.

### Notifications

- Subscriber emails and generic signed webhooks are both queued durably in PostgreSQL.
- The API server dispatches queued email and webhook deliveries on intervals, with retry behavior and failure tracking.
- Dashboard settings expose recent delivery outcomes, retry actions for failed deliveries, and subscriber management controls.

### Managed billing lifecycle

- Stripe checkout and billing portal sessions are created by the Rust API for authenticated organization admins.
- Stripe webhooks update durable organization subscription state, record received billing events for support visibility, and drive the downgrade lifecycle.
- Downgrades now keep paid-only configuration active through a 14-day grace window, send warning emails, and then enforce lower-plan limits without deleting customer data.
- The dashboard reflects plan state, downgrade state, entitlement violations, and required remediation actions.
- Custom-domain verification compares the configured custom domain against the managed target host and stores the last verified timestamp.
- Audit logs record high-value managed actions such as billing sync, invitation lifecycle, downgrade intervention, webhook changes, and custom-domain verification.

### Internal operations

- `/api/admin/*` exposes token-guarded support endpoints for queue health, searchable org support, billing sync, downgrade intervention, invitation resend, billing-event history, recent audit logs, and retrying failed deliveries.
- These endpoints are intended for internal operators only and require `INTERNAL_ADMIN_TOKEN`.
- The web dashboard now includes `/dashboard/internal-support` as a lightweight operator console over the same internal APIs.

### Realtime

- Manual dashboard changes and monitor-driven changes publish into Redis-backed channels.
- The web app subscribes over server-sent events and refreshes affected surfaces.

## Operations surface

- `/health` checks database and Redis connectivity.
- `/ready` provides a readiness-friendly status code for deploy orchestration.
- `/ops/summary` provides an operator-friendly JSON summary of runtime counts and queue state.
