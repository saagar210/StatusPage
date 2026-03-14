# Managed Beta Security Baseline

This is the current lightweight security baseline for the managed paid beta.

## Required controls

- GitHub OAuth is the only customer login path.
- Stripe webhooks must verify `Stripe-Signature`.
- Internal support endpoints must require `INTERNAL_ADMIN_TOKEN`.
- Public subscribe, verify, unsubscribe, and invitation-accept flows must keep rate limiting enabled.
- High-value org and operator actions must write to the audit log.
- Backups and rollback drills remain part of release rehearsal.

## Operator checklist

- Rotate `AUTH_SECRET`, Stripe secrets, SMTP credentials, and `INTERNAL_ADMIN_TOKEN` on a regular schedule.
- Keep database backups encrypted at rest in the chosen backup destination.
- Restrict access to production environment variables to named operators only.
- Re-run `pnpm rehearse:prod` and `pnpm rehearse:rollback` before calling a managed-beta release ready.

## Known beta limitations

- Operator access is still token-gated rather than using separate operator accounts or RBAC.
- Live Stripe and hosted DNS validation still need external staging/production proof.
