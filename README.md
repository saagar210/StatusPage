# StatusPage.sh

Open-source status page platform with automated monitoring. Self-hostable with an optional hosted tier.

[![CI](https://github.com/saagar210/StatusPage/actions/workflows/ci.yml/badge.svg)](https://github.com/saagar210/StatusPage/actions/workflows/ci.yml)

## Features

- **Automated Monitoring** — HTTP, TCP, DNS, and ICMP ping health checks with configurable intervals and thresholds
- **Incident Management** — Manual incident creation with status updates, timeline, and service impact tracking
- **Public Status Page** — Beautiful, server-rendered status page with 90-day uptime history
- **Real-time Dashboard** — Manage services, incidents, and monitors through an authenticated dashboard
- **Self-Hostable** — MIT licensed, runs on Docker with PostgreSQL
- **Auth via GitHub OAuth** — Secure authentication using Auth.js v5

## Tech Stack

- **Backend**: Rust (Axum 0.8) + PostgreSQL 16
- **Frontend**: Next.js 15 (App Router) + shadcn/ui + Tailwind CSS v4
- **Monitoring**: Standalone Rust binary with cron scheduler
- **Auth**: Auth.js v5 with GitHub OAuth
- **Monorepo**: pnpm workspaces + Cargo workspace + Turborepo

## Architecture

```
apps/
  web/          Next.js 15 dashboard + public pages
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

3. **Start PostgreSQL and Redis**

```bash
pnpm run db:up
```

4. **Run migrations**

Requires [sqlx-cli](https://github.com/launchbadge/sqlx/tree/main/sqlx-cli):

```bash
cargo install sqlx-cli --no-default-features --features postgres
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
cargo run -p monitor
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
pnpm --filter web test        # Vitest (39 tests)
pnpm --filter web typecheck
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
- Next.js: `typecheck`, `build`

See [`.github/workflows/ci.yml`](.github/workflows/ci.yml)

## Project Status

**Phase 1 + 2 Complete** (20/20 steps from implementation plan)

✅ Core infrastructure (monorepo, DB schema, migrations)
✅ Rust API with auth middleware + CRUD endpoints
✅ Next.js dashboard (onboarding, services, incidents, monitors, settings)
✅ Public status page with uptime charts and incident history
✅ GitHub OAuth authentication (Auth.js v5)
✅ Monitor engine with HTTP/TCP/DNS/Ping checkers
✅ Threshold-based evaluator with auto-incident creation
✅ Daily uptime rollup with 90-day history
✅ Seed data command
✅ Test suite: 14 Rust unit tests + 39 Vitest tests
✅ Playwright E2E setup
✅ GitHub Actions CI

**Phase 3+ Roadmap** (not yet implemented)

- [ ] Redis pub/sub for real-time dashboard updates
- [ ] Email notifications (incident updates, monitor alerts)
- [ ] Webhooks (Slack, Discord, PagerDuty)
- [ ] Multi-region monitoring
- [ ] Custom domains for status pages
- [ ] Stripe billing integration
- [ ] Status page themes and branding
- [ ] Subscriber notifications (email/SMS)

## API Documentation

The Rust API exposes the following endpoints:

### Organizations

- `POST /api/organizations` — Create org (auto-adds user as owner)
- `GET /api/organizations` — List user's orgs
- `GET /api/organizations/:slug` — Get org details
- `PATCH /api/organizations/:slug` — Update org (admin+)

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

## Database Schema

11 migrations create the following tables:

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

## Environment Variables

```bash
# Database
DATABASE_URL=postgresql://statuspage:statuspage@localhost:5432/statuspage

# Redis (unused in Phase 1-2, but running)
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

# Next.js
NEXT_PUBLIC_API_URL=http://localhost:4000
INTERNAL_API_URL=http://localhost:4000
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
