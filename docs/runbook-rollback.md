# Rollback Runbook

## When to roll back

- Deploy smoke checks fail.
- `/ready` stays unhealthy.
- `/ops/summary` shows rapidly growing failed or pending delivery queues.
- Core dashboard or public status routes are unavailable after deploy.

## Rollback steps

1. Stop the rollout.
2. Re-point deployment to the previous known-good image tags or known-good environment values.
3. Restart the compose stack with the previous version.
4. Re-run:

```bash
STATUS_SLUG=<org-slug> pnpm smoke:prod
```

5. Capture `/ops/summary` and confirm queue counts stabilize.

## Rehearsal

Run a rollback rehearsal before release candidates:

```bash
STATUS_SLUG=<org-slug> pnpm rehearse:rollback
```

The rehearsal intentionally forces the API into an unhealthy state, confirms the failure is visible, recreates the known-good services, and reruns the production smoke checks.

## Database posture

- Avoid rolling back the database independently unless the migration is known to be backward compatible.
- If data integrity is at risk, restore from the last known-good backup using the backup/restore runbook.
