# Backup And Restore Runbook

## Backup

1. Ensure the production stack is healthy.
2. Run:

```bash
pnpm backup:prod backups/statuspage-$(date +%Y%m%dT%H%M%S).sql
```

3. Confirm the SQL file exists and is non-empty.
4. Store the artifact in your normal backup destination.

## Restore

1. Bring the application stack to maintenance mode or stop writes.
2. Restore from a known backup:

```bash
pnpm restore:prod backups/<backup-file>.sql
```

The restore script drops and recreates the `public` schema before replaying the SQL dump. This is intentional because the current schema includes partitioned tables, and the cleanest rehearsal path is a full schema reset followed by restore.

3. Run the smoke checks:

```bash
STATUS_SLUG=<your-org-slug> pnpm smoke:prod
```

4. Confirm `/ops/summary` shows expected counts for organizations, services, monitors, and delivery queues.

## Validation drill

- Run one backup and one restore rehearsal before each release candidate.
- Record the backup file used, the restore target, and the post-restore smoke results.
- If you want a full operator rehearsal, use the production deployment drill first, then run backup and restore against that live stack:

```bash
STATUS_SLUG=<your-org-slug> pnpm rehearse:prod
pnpm backup:prod backups/<backup-file>.sql
pnpm restore:prod backups/<backup-file>.sql
STATUS_SLUG=<your-org-slug> pnpm smoke:prod
```

- `pnpm smoke:prod` checks API health, readiness, `/ops/summary`, and the web health endpoint. When `STATUS_SLUG` is set, it also verifies both the public status API and the public status page.
