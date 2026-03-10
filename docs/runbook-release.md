# Release Runbook

## Pre-release gates

- `pnpm verify`
- `pnpm verify:perf`
- `pnpm e2e:auth`
- `pnpm smoke:email`
- `pnpm smoke:webhooks`
- `STATUS_SLUG=<org-slug> pnpm rehearse:prod`
- `STATUS_SLUG=<org-slug> pnpm rehearse:rollback`

## Release steps

1. Merge the release-ready branch to `main`.
2. Tag the release using `v<version>`.
3. Push the tag to trigger the release workflow.
4. Confirm published images exist for:
   - `web`
   - `api-server`
   - `monitor`
5. Deploy using the production compose stack or your hosted equivalent.
6. Run post-deploy smoke checks:

```bash
STATUS_SLUG=<org-slug> pnpm smoke:prod
```

7. Capture `/ops/summary` as the initial post-release health snapshot.

## Post-release verification

- Public status page loads.
- Dashboard login works.
- Service or incident edits still emit realtime updates.
- Email and webhook dispatch queues are healthy.
- No abnormal growth in pending or failed delivery counts.
- If you use a seeded or real public organization slug in production, confirm both the public API and public page respond successfully during smoke validation.

## Release artifacts

- Use `docs/release-checklist.md` as the go or no-go worksheet.
- Publish notes from `docs/release-notes-template.md`.
