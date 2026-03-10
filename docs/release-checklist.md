# Self-Hosted Release Checklist

Use this checklist before calling a self-hosted release candidate ready.

## Repo gates

- `pnpm verify`
- `pnpm verify:perf`
- `pnpm e2e:auth`
- `pnpm smoke:email`
- `pnpm smoke:webhooks`

## Production rehearsals

- `STATUS_SLUG=<org-slug> pnpm rehearse:prod`
- `STATUS_SLUG=<org-slug> pnpm rehearse:rollback`
- `pnpm backup:prod backups/<backup-file>.sql`
- `pnpm restore:prod backups/<backup-file>.sql`
- `STATUS_SLUG=<org-slug> pnpm smoke:prod`

## Operator checks

- `/health` returns success.
- `/ready` returns success.
- `/ops/summary` shows expected organization, service, monitor, subscriber, email, and webhook counts.
- Dashboard login works.
- Public status page loads.
- Service or incident changes still appear through the live update path.
- Email and webhook retry queues do not show unexpected growth.

## Documentation checks

- `README.md` matches the shipped self-hosted behavior.
- `docs/production-docker.md` matches the current compose stack and env requirements.
- `docs/runbook-backup-restore.md` matches the real backup and restore path.
- `docs/runbook-rollback.md` matches the proven rollback rehearsal.
- `docs/runbook-release.md` matches the actual go-live sequence.

## Release output

- Tag the release using `v<version>`.
- Publish release notes using `docs/release-notes-template.md`.
- Capture the initial `/ops/summary` snapshot after deployment.
- Record the backup file used for the release window.
- Record any environment-specific operator notes discovered during deployment.
