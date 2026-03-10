## Web App

This package contains the authenticated dashboard, public status page, proxy routes, and test configuration for StatusPage.

For product setup, local development, and repo-wide verification, start with the root README:

- [`/Users/d/Projects/MoneyPRJsViaGPT/StatusPage/README.md`](/Users/d/Projects/MoneyPRJsViaGPT/StatusPage/README.md)

## Useful Commands

```bash
# Start the Next.js app
pnpm --filter web dev

# Web-only checks
pnpm --filter web lint
pnpm --filter web test
pnpm --filter web typecheck
pnpm --filter web build
```

## Notes

- Playwright configuration lives in `apps/web/playwright.config.ts`.
- Public and dashboard data requests proxy through `app/api/proxy/[...path]/route.ts`.
- Realtime support is still under active implementation; the current SSE route is an interim scaffold, not the final production architecture.
