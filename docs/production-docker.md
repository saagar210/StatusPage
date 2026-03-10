# Production Docker Scaffold

This repository now includes a production-oriented Docker stack for local operator validation, release rehearsal, and self-hosted operator drills.

## Files

- `docker/Dockerfile.web`
- `docker/Dockerfile.api-server`
- `docker/Dockerfile.monitor`
- `docker/docker-compose.prod.yml`
- `docker/Caddyfile`
- `.env.production.example`
- `scripts/smoke-prod.sh`
- `docs/architecture.md`
- `docs/runbook-backup-restore.md`
- `docs/runbook-release.md`
- `docs/runbook-rollback.md`

## First Run

1. Copy `.env.production.example` to `.env.production`.
2. Fill in the auth values and adjust hostnames or ports for your environment.
   If you want subscriber and incident emails, also fill in the SMTP and `EMAIL_FROM` values.
   If you want a custom-domain public status page, set `STATUSPAGE_HOST` to the hostname you want Caddy to serve and make sure the same hostname is saved in organization settings.
3. Build the images:

```bash
pnpm docker:prod:build
```

4. Start the stack:

```bash
pnpm docker:prod:up
```

5. Run the smoke checks:

```bash
STATUS_SLUG=<your-org-slug> pnpm smoke:prod
```

By default, the smoke script checks:

- API health at `http://localhost:4000/health`
- API readiness at `http://localhost:4000/ready`
- API operator summary at `http://localhost:4000/ops/summary`
- The web health endpoint at `http://localhost/api/health`

When `STATUS_SLUG` is set, the smoke script also verifies:

- `GET /api/public/<slug>/status` on the API
- `GET /s/<slug>` through the web and reverse proxy

That public slug check requires a real organization slug with public data. On a blank stack, use the base smoke checks first or seed data before running the slug-specific checks.

## Rehearsal Commands

Use these commands to rehearse the operator path, not just boot containers:

```bash
STATUS_SLUG=<your-org-slug> pnpm rehearse:prod
STATUS_SLUG=<your-org-slug> pnpm rehearse:rollback
pnpm backup:prod backups/statuspage-$(date +%Y%m%dT%H%M%S).sql
pnpm restore:prod backups/<backup-file>.sql
```

## Startup Model

- `postgres` and `redis` start first.
- `migrate` runs one time after PostgreSQL is healthy.
- `api-server` starts after migrations and waits for PostgreSQL and Redis.
- `web` starts after the API is healthy.
- `caddy` exposes the web app on `http://localhost` and proxies traffic to the internal web container.
- `monitor` starts after migrations and writes a heartbeat file for container health checks.

## Operator Notes

- Backups and restores use `pnpm backup:prod` and `pnpm restore:prod`.
- The restore script resets the `public` schema before replaying a SQL backup so restores stay reliable with the current partitioned-table layout.
- Release, rollback, and backup drills are documented in the runbooks listed above.
- `/ops/summary` gives an operator-friendly JSON snapshot of queue and object counts after deployment.
- The public status smoke path is optional until you seed or create a real organization.
- Email delivery is supported through standard SMTP environment variables.
- When a request host matches an organization’s configured custom domain, the web app serves that organization’s public pages directly at `/`, `/history`, `/verify`, and `/unsubscribe`.
- Live DNS, TLS, and proxy ownership are still operator concerns outside the app itself; the repo currently provides the host-aware application behavior and the Caddy entrypoint for local rehearsal.
