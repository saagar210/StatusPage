# Release Notes Template

## Summary

- Version:
- Release date:
- Release owner:

## Included in this release

- Core product changes:
- Operator or deployment changes:
- Notification or delivery changes:
- Documentation and runbook changes:

## Verification

- `pnpm verify`
- `pnpm verify:perf`
- `pnpm e2e:auth`
- `pnpm smoke:email`
- `pnpm smoke:webhooks`
- `STATUS_SLUG=<org-slug> pnpm rehearse:prod`
- `STATUS_SLUG=<org-slug> pnpm rehearse:rollback`

## Deployment notes

- Required environment changes:
- Migration notes:
- Backup file used before deploy:
- Rollback reference:

## Post-release checks

- Public status page:
- Dashboard login:
- `/ops/summary` snapshot:
- Delivery queue health:
- Follow-up items:
