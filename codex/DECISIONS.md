# Decisions

## 2026-02-10
- D-001: Treat Next.js build Google Fonts fetch failure as environment limitation, not code regression.
  - Evidence: failure occurs in baseline before new edits.
  - Consequence: keep build warning tracked; do not mutate font behavior in this recovery scope.
- D-002: Keep scope to typed plan plumbing and UX resilience; do not add billing schema or Stripe integration in this pass.
  - Alternative rejected: adding subscriptions tables now would exceed recovery scope and violate small-safe-change principle.
- D-003: Preserve API response shape for `Organization.plan` as snake_case string while using backend enum internally.
  - Rationale: backward-compatible contract with stronger server typing.
