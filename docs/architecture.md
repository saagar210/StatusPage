# Architecture Overview

## Runtime services

- `apps/web` serves the dashboard, marketing site, public status pages, and the authenticated proxy layer.
- `packages/api-server` owns the primary API, organization access rules, incident/service/monitor workflows, webhook delivery, and email queue dispatch.
- `apps/monitor` executes monitor checks, writes uptime data, auto-creates or resolves incidents, and now also emits realtime plus notification events.
- PostgreSQL is the system of record.
- Redis is used for realtime fanout and health validation.

## Key product flows

### Dashboard administration

- Authenticated users sign in through Auth.js and GitHub OAuth.
- Organization access is enforced by `OrgAccess`, which resolves membership and role for the requested org.
- Admin-only settings now cover organization branding fields, team-member management, notification preferences, subscribers, delivery activity, and webhooks.

### Public status pages

- Public status, history, uptime, subscribe, verify, and unsubscribe routes are served through the Rust API and rendered in the web app.
- Organizations with a configured custom domain can serve their public experience directly from `/`, `/history`, `/verify`, and `/unsubscribe` when the incoming host matches.
- Subscriber emails are queued in PostgreSQL and dispatched asynchronously.

### Notifications

- Subscriber emails and generic signed webhooks are both queued durably in PostgreSQL.
- The API server dispatches queued email and webhook deliveries on intervals, with retry behavior and failure tracking.
- Dashboard settings expose recent delivery outcomes, retry actions for failed deliveries, and subscriber management controls.

### Realtime

- Manual dashboard changes and monitor-driven changes publish into Redis-backed channels.
- The web app subscribes over server-sent events and refreshes affected surfaces.

## Operations surface

- `/health` checks database and Redis connectivity.
- `/ready` provides a readiness-friendly status code for deploy orchestration.
- `/ops/summary` provides an operator-friendly JSON summary of runtime counts and queue state.
