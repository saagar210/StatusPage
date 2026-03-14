# StatusPage.sh

Open-source status page platform with automated monitoring. The current repo is
focused on a launchable self-hosted core plus an early managed paid beta path.
Operator hardening is still underway, and the managed SaaS scope is intentionally
limited to GitHub-first auth, shared multi-tenant hosting, and self-serve upgrades.

[![CI](https://github.com/saagar210/StatusPage/actions/workflows/ci.yml/badge.svg)](https://github.com/saagar210/StatusPage/actions/workflows/ci.yml)

## Features

- **Automated Monitoring** — HTTP, TCP, DNS, and ICMP ping health checks with configurable intervals and thresholds
- **Incident Management** — Manual incident creation with status updates, timeline, and service impact tracking
- **Public Status Page** — Server-rendered status page with 90-day uptime history
- **Realtime Status Updates** — Dashboard and public pages react to incident and service changes without refresh
- **Email Subscribers + Webhooks** — Subscriber verification, SMTP delivery, signed webhook delivery, retry, and admin activity visibility
- **Authenticated Dashboard** — Manage services, incidents, and monitors through an authenticated dashboard
- **Self-Hostable Core** — MIT licensed core with a local Docker-backed development stack
- **Managed Paid Beta Billing** — Free / Pro / Team plans with self-serve upgrades, Stripe portal access, and plan-gated custom domains plus outbound webhooks
- **Auth via GitHub OAuth** — Secure authentication using Auth.js v5

## Tech Stack

- **Backend**: Rust (Axum 0.8) + PostgreSQL 16
- **Frontend**: Next.js 16 (App Router) + shadcn/ui + Tailwind CSS v4
- **Monitoring**: Standalone Rust binary with cron scheduler
- **Auth**: Auth.js v5 with GitHub OAuth
- **Monorepo**: pnpm workspaces + Cargo workspace + Turborepo

## Architecture

```
apps/
  web/          Next.js 16 dashboard + public pages
  monitor/      Rust monitoring engine (standalone binary)
packages/
  api-server/   Rust Axum REST API
  shared/       Shared Rust types (models, enums, validation)
```

- **Rust API** serves at `:4000` with REST endpoints for all CRUD operations
- **Next.js** runs at `:3000` with SSR for public pages, client-side dashboard
- **Monitor engine** runs checks on configurable intervals, updates service status, creates auto-incidents
- **Session sharing**: Rust validates Auth.js session cookies from shared PostgreSQL table
- **No CORS needed**: Next.js proxy (`/api/proxy/[...path]`) forwards to Rust API

## Getting Started

### Prerequisites

- Rust (stable)
- Node.js 20+
- pnpm 9+
- Docker (for PostgreSQL + Redis)
- GitHub OAuth App ([create one](https://github.com/settings/developers))

### Installation

1. **Clone and install dependencies**

```bash
git clone https://github.com/saagar210/StatusPage.git
cd StatusPage
pnpm install
```

2. **Set up environment variables**

```bash
cp .env.example .env
# Edit .env and fill in:
#   - DATABASE_URL
#   - AUTH_SECRET (generate with: openssl rand -base64 32)
#   - AUTH_GITHUB_ID and AUTH_GITHUB_SECRET
```

The default local `.env.example` uses high-numbered ports (`55432` for PostgreSQL,
`56379` for Redis) to avoid colliding with services you may already have running
on the usual local defaults.
Leave `AUTH_GITHUB_ID` and `AUTH_GITHUB_SECRET` blank until you have a real GitHub
OAuth app; the local helper scripts source `.env`, so placeholder values must stay
shell-safe.

3. **Start PostgreSQL and Redis**

```bash
pnpm run db:up
```

4. **Run migrations**

```bash
pnpm run db:migrate
```

5. **Seed demo data** (optional)

```bash
pnpm run db:seed
```

Creates:
- Demo organization (slug: `demo`)
- 5 services (API, Web App, Database, CDN, Email)
- 2 incidents (1 active, 1 resolved with 4 timeline updates)
- 3 monitors (HTTP x2, TCP x1)

6. **Start the dev servers**

```bash
# Terminal 1: Rust API
pnpm run dev:api

# Terminal 2: Next.js
pnpm run dev:web

# Terminal 3: Monitor engine (optional)
pnpm run dev:monitor
```

### Lean Dev Mode (low disk usage)

`pnpm run dev:lean` starts API + web using temporary build-cache locations and
cleans heavy artifacts automatically on exit.

```bash
pnpm run dev:lean
```

Tradeoff:
- Lower persistent disk usage: cleans `.next`, `target`, `.turbo`, test reports
- Slower cold starts after each restart: build artifacts are intentionally removed
- Keeps dependencies (`node_modules`) so reinstall is not required each run

7. **Visit the app**

- Landing page: http://localhost:3000
- Login: http://localhost:3000/login (GitHub OAuth)
- Public status page: http://localhost:3000/s/demo
- Dashboard: http://localhost:3000/dashboard/demo (after login)

## Development

### Setup & Verification

```bash
# Prepare dependencies and validate the local toolchain
pnpm setup:local

# Run the canonical repo verification commands
pnpm verify

# Run the managed-product verification layer
pnpm verify:managed

# Prove subscriber verification + incident email delivery locally
pnpm smoke:email

# Prove generic webhook delivery locally
pnpm smoke:webhooks

# Prove the managed onboarding + operator-support path locally
pnpm smoke:managed
```

### Managed beta billing

The hosted/managed product path currently targets a paid beta with these plan rules:

- `Free`: up to 3 monitors, no custom domain, no outbound webhooks
- `Pro`: up to 20 monitors, custom domain enabled, outbound webhooks enabled
- `Team`: unlimited monitors, custom domain enabled, outbound webhooks enabled, priority support handling

What is shipped in this repo today:

- Self-serve Stripe checkout for upgrades
- Stripe billing portal handoff for payment-method and cancellation management
- In-app plan state, subscription status, and entitlement visibility
- API enforcement for monitor limits, custom-domain access, and outbound webhook access
- Invitation-based teammate access with GitHub-email matching on acceptance
- Product-sent invitation emails plus invitation lifecycle visibility in settings and the support console
- Custom-domain verification checks against the configured managed target
- Internal operator/support endpoints plus a dashboard support console guarded by `INTERNAL_ADMIN_TOKEN`
- Audit logs for billing sync, invitation lifecycle, webhook management, custom-domain verification, and operator retries
- Pre-GA downgrade lifecycle with grace windows, warning emails, non-destructive enforcement, and operator controls

What is still intentionally limited for the beta:

- GitHub is the only customer auth provider
- Live Stripe validation in staging/production still needs real external credentials and webhook delivery
- Managed onboarding is now multi-step, but cross-session draft persistence is still out of scope
- The internal support console is intentionally token-gated rather than a separate operator identity system

### Production Scaffold

The repo now includes a production-oriented Docker path for local operator validation and rehearsal:

```bash
cp .env.production.example .env.production
pnpm docker:prod:build
pnpm docker:prod:up
STATUS_SLUG=<your-org-slug> pnpm smoke:prod
```

For a fuller operator drill:

```bash
STATUS_SLUG=<your-org-slug> pnpm rehearse:prod
STATUS_SLUG=<your-org-slug> pnpm rehearse:rollback
pnpm backup:prod backups/statuspage-$(date +%Y%m%dT%H%M%S).sql
pnpm restore:prod backups/<backup-file>.sql
```

Key files:
- `docker/docker-compose.prod.yml`
- `.env.production.example`
- `docker/Dockerfile.api-server`
- `docker/Dockerfile.monitor`
- `docker/Dockerfile.web`

### Build & Test

```bash
# Build everything
pnpm build

# Rust
cargo build --workspace
cargo test --workspace
cargo clippy --workspace -- -D warnings
cargo fmt --all

# Next.js
pnpm --filter web build
pnpm --filter web test        # Vitest (56 tests)
pnpm --filter web typecheck
pnpm e2e:auth                # deterministic authenticated Playwright flow (requires Docker)
```

### Cleanup Commands

```bash
# Remove heavy build artifacts only (keeps dependencies for faster restarts)
pnpm run clean:heavy

# Remove all reproducible local caches, including dependencies
pnpm run clean:full
```

### CI

GitHub Actions workflow runs on push/PR:
- Rust: `cargo fmt`, `clippy`, `test`
- Next.js: `lint`, `test`, `typecheck`, `build`

See [`.github/workflows/ci.yml`](.github/workflows/ci.yml)

### Production Compose Scaffold

An initial production-oriented Docker stack now lives in `docker/docker-compose.prod.yml`.

```bash
cp .env.production.example .env.production
# Fill in secrets and OAuth values in .env.production

pnpm docker:prod:build
pnpm docker:prod:up
STATUS_SLUG=<your-org-slug> pnpm smoke:prod
```

Supporting files:
- `docker/Caddyfile` for reverse proxying through Caddy
- `docker/Dockerfile.web`, `docker/Dockerfile.api-server`, `docker/Dockerfile.monitor`
- `scripts/backup-postgres.sh` and `scripts/restore-postgres.sh` for basic operator workflows
- `scripts/rehearse-prod.sh` and `scripts/rehearse-rollback.sh` for clean deployment and rollback drills

## Project Status

**Implemented today**

✅ Core infrastructure (monorepo, DB schema, migrations)
✅ Rust API with auth middleware + CRUD endpoints
✅ Next.js dashboard (onboarding, services, incidents, monitors, settings)
✅ Public status page with uptime charts and incident history
✅ Host-based custom-domain public routing for `/`, `/history`, `/verify`, and `/unsubscribe`
✅ GitHub OAuth authentication (Auth.js v5)
✅ Monitor engine with HTTP/TCP/DNS/Ping checkers
✅ Threshold-based evaluator with auto-incident creation
✅ Daily uptime rollup with 90-day history
✅ Seed data command
✅ Rust + Vitest test suites
✅ Deterministic authenticated Playwright E2E via `pnpm e2e:auth`
✅ GitHub Actions CI for core web and Rust checks
✅ Local production Docker build + smoke validation path
✅ Subscriber and delivery operations in dashboard settings

**Partially implemented / still in progress**

- [~] Realtime event delivery works through the web SSE bridge and monitor-originated events, but broader coverage and polish are still in progress
- [~] Self-hosted production Docker deployment, backup/restore, and rollback rehearsals are now documented and proven locally; live host-specific validation is still an operator task
- [~] Redis pub/sub for real-time dashboard updates
- [~] Email notifications and public subscriber verification are live for SMTP-backed installs, including dashboard visibility, resend, and retry operations; provider-specific polish is still pending
- [~] Webhooks (generic delivery, signing, retry, and dashboard retry actions are in place; provider-specific formatting and deeper drill-down are still pending)
- [ ] Multi-region monitoring
- [~] Custom domains for status pages are wired through dashboard settings, public routing, and email links; live DNS/TLS proof is still an operator task
- [x] Stripe billing integration now supports managed-beta upgrades, portal access, plan-gated entitlements, and a pre-GA downgrade lifecycle with grace-period enforcement
- [x] Invitation-based teammate access, invite email delivery, custom-domain verification, and a multi-step managed onboarding flow are now shipped in the repo
- [~] Status page branding baseline exists in organization settings; advanced themes are still pending
- [ ] SMS and provider-specific notification channels

## API Documentation

The Rust API exposes the following endpoints:

### Organizations

- `POST /api/organizations` — Create org (auto-adds user as owner)
- `GET /api/organizations` — List user's orgs
- `GET /api/organizations/:slug` — Get org details
- `PATCH /api/organizations/:slug` — Update org (admin+)
- `GET /api/organizations/:slug/members` — List team members (admin+)
- `POST /api/organizations/:slug/members` — Add an existing user to the org (admin+)
- `PATCH /api/organizations/:slug/members/:id` — Change member role (admin+)
- `DELETE /api/organizations/:slug/members/:id` — Remove a member (admin+)
- `GET /api/organizations/:slug/billing` — Billing summary, subscription state, and entitlements
- `POST /api/organizations/:slug/billing/checkout` — Start a Stripe checkout session for an allowed upgrade
- `POST /api/organizations/:slug/billing/portal` — Start a Stripe billing portal session
- `GET /api/organizations/:slug/entitlements` — Get plan-driven entitlement flags for the org
- `POST /api/organizations/:slug/custom-domain/verify` — Check whether a custom domain resolves to the managed target
- `GET /api/organizations/:slug/invitations` — List invitations and delivery state
- `POST /api/organizations/:slug/invitations` — Create a teammate invitation
- `DELETE /api/organizations/:slug/invitations/:id` — Cancel a pending invitation
- `POST /api/organizations/:slug/invitations/:id/resend` — Re-send a pending invitation email
- `POST /api/invitations/:token/accept` — Accept an invitation with the signed-in GitHub account
- `POST /api/billing/stripe/webhook` — Process Stripe checkout and subscription webhooks
- `GET /api/admin/queue-health` — Internal operator queue and billing-event summary (`x-statuspage-admin-token`)
- `GET /api/admin/organizations/search?q=...` — Internal operator managed-org search (`x-statuspage-admin-token`)
- `GET /api/admin/organizations/:slug/support` — Internal operator org support summary (`x-statuspage-admin-token`)
- `POST /api/admin/organizations/:slug/billing/sync` — Internal operator Stripe sync (`x-statuspage-admin-token`)
- `POST /api/admin/organizations/:slug/downgrade/enforce` — Internal operator downgrade enforcement (`x-statuspage-admin-token`)
- `POST /api/admin/organizations/:slug/downgrade/cancel` — Internal operator downgrade cancel (`x-statuspage-admin-token`)
- `POST /api/admin/organizations/:slug/invitations/:id/resend` — Internal operator invite resend (`x-statuspage-admin-token`)
- `POST /api/admin/organizations/:slug/retry/email/:id` — Internal operator retry for a failed email delivery (`x-statuspage-admin-token`)
- `POST /api/admin/organizations/:slug/retry/webhook/:id` — Internal operator retry for a failed webhook delivery (`x-statuspage-admin-token`)

### Services

- `POST /api/organizations/:slug/services` — Create service
- `GET /api/organizations/:slug/services` — List services
- `GET /api/organizations/:slug/services/:id` — Get service
- `PATCH /api/organizations/:slug/services/:id` — Update service
- `DELETE /api/organizations/:slug/services/:id` — Delete service
- `PATCH /api/organizations/:slug/services/reorder` — Reorder services

### Incidents

- `POST /api/organizations/:slug/incidents` — Create incident
- `GET /api/organizations/:slug/incidents` — List incidents (with pagination)
- `GET /api/organizations/:slug/incidents/:id` — Get incident with timeline
- `PATCH /api/organizations/:slug/incidents/:id` — Update incident
- `DELETE /api/organizations/:slug/incidents/:id` — Delete incident (owner only)
- `POST /api/organizations/:slug/incidents/:id/updates` — Add timeline update

### Monitors

- `POST /api/organizations/:slug/monitors` — Create monitor
- `GET /api/organizations/:slug/monitors` — List monitors with stats
- `GET /api/organizations/:slug/monitors/:id` — Get monitor detail
- `PATCH /api/organizations/:slug/monitors/:id` — Update monitor config
- `DELETE /api/organizations/:slug/monitors/:id` — Delete monitor
- `GET /api/organizations/:slug/monitors/:id/checks` — Get check history

### Public (unauthenticated)

- `GET /api/public/:slug/status` — Org info + services + active incidents
- `GET /api/public/:slug/incidents` — Incident history (paginated)
- `GET /api/public/:slug/uptime` — 90-day uptime data per service
- `POST /api/public/:slug/subscribe` — Start subscriber verification by email
- `GET /api/public/:slug/subscribers/verify?token=...` — Confirm a subscriber email
- `GET /api/public/:slug/subscribers/unsubscribe?token=...` — Unsubscribe a verified email
- `GET /api/public/resolve?host=...` — Resolve a custom domain host to a public status page organization

### Notifications (admin+)

- `GET /api/organizations/:slug/notifications/preferences` — Load notification preferences
- `PATCH /api/organizations/:slug/notifications/preferences` — Update notification preferences
- `GET /api/organizations/:slug/notifications/subscribers` — List subscribers
- `DELETE /api/organizations/:slug/notifications/subscribers/:id` — Remove subscriber
- `POST /api/organizations/:slug/notifications/subscribers/:id/resend` — Resend subscriber verification
- `GET /api/organizations/:slug/notifications/deliveries/email` — List email delivery history
- `POST /api/organizations/:slug/notifications/deliveries/email/:id/retry` — Retry a failed email delivery
- `GET /api/organizations/:slug/notifications/deliveries/webhooks` — List webhook delivery history
- `POST /api/organizations/:slug/notifications/deliveries/webhooks/:id/retry` — Retry a failed webhook delivery
- `GET /api/organizations/:slug/notifications/webhooks` — List webhook configs
- `POST /api/organizations/:slug/notifications/webhooks` — Create webhook config
- `PATCH /api/organizations/:slug/notifications/webhooks/:id` — Update webhook config
- `DELETE /api/organizations/:slug/notifications/webhooks/:id` — Delete webhook config

## Database Schema

15 migrations create the following tables:

- `users`, `accounts`, `sessions`, `verification_tokens` — Auth.js schema
- `organizations` — Tenants
- `members` — User-org membership with roles (owner, admin, member)
- `services` — Monitored services with current status
- `incidents` — Incidents with status, impact, and affected services
- `incident_updates` — Timeline updates for incidents
- `incident_services` — Junction table (incidents ↔ services)
- `monitors` — Health check configs (HTTP, TCP, DNS, Ping)
- `monitor_checks` — Check results (partitioned by month)
- `uptime_daily` — Daily rollup with calculated uptime percentage
- `webhook_configs`, `webhook_deliveries` — Configured webhook endpoints and delivery tracking
- `subscribers`, `notification_logs`, `notification_preferences` — Subscriber verification, email delivery queueing, organization notification preferences, and admin-facing delivery visibility

## Environment Variables

```bash
# Database
DATABASE_URL=postgresql://statuspage:statuspage@localhost:5432/statuspage

# Redis
REDIS_URL=redis://localhost:6379

# Auth (Next.js)
AUTH_SECRET=<random-32-char-string>
AUTH_GITHUB_ID=<github-oauth-app-id>
AUTH_GITHUB_SECRET=<github-oauth-app-secret>
NEXTAUTH_URL=http://localhost:3000

# Rust API Server
API_PORT=4000
API_HOST=0.0.0.0
CORS_ORIGIN=http://localhost:3000
LOG_LEVEL=info
APP_BASE_URL=http://localhost:3000
SMTP_HOST=
SMTP_PORT=1025
SMTP_USERNAME=
SMTP_PASSWORD=
SMTP_SECURE=false
EMAIL_FROM=alerts@statuspage.local
EMAIL_DISPATCH_INTERVAL_SECS=3
EMAIL_DISPATCH_BATCH_SIZE=20
WEBHOOK_DISPATCH_INTERVAL_SECS=3
WEBHOOK_DISPATCH_BATCH_SIZE=10
WEBHOOK_TIMEOUT_SECS=10
STRIPE_SECRET_KEY=
STRIPE_WEBHOOK_SECRET=
STRIPE_PRICE_PRO=
STRIPE_PRICE_TEAM=

# Next.js
NEXT_PUBLIC_API_URL=http://localhost:4000
INTERNAL_API_URL=http://localhost:4000
STATUSPAGE_HOST=localhost
```

## License

MIT © 2026

## Contributing

Contributions welcome! Please open an issue or PR.

1. Fork the repo
2. Create your feature branch (`git checkout -b feat/amazing-feature`)
3. Run tests (`cargo test --workspace && pnpm test`)
4. Commit your changes (`git commit -m 'feat: add amazing feature'`)
5. Push to the branch (`git push origin feat/amazing-feature`)
6. Open a Pull Request

## Acknowledgments

Built with:
- [Axum](https://github.com/tokio-rs/axum) — Rust web framework
- [Next.js](https://nextjs.org) — React framework
- [Auth.js](https://authjs.dev) — Authentication
- [shadcn/ui](https://ui.shadcn.com) — UI components
- [Tailwind CSS](https://tailwindcss.com) — Styling
- [SQLx](https://github.com/launchbadge/sqlx) — Rust SQL toolkit
- [Turborepo](https://turbo.build) — Monorepo build system
