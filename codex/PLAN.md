# Delta Plan

## A) Executive Summary

### Current state
- Latest commit adds plan monitor cap enforcement in `packages/api-server/src/routes/monitors.rs`.
- Enforcement currently parses `Organization.plan` string at request-time (`parse_plan`), introducing avoidable runtime conversion and duplicated plan knowledge.
- Organization domain model still stores `plan` as `String` (`packages/shared/src/models/organization.rs`).
- Frontend settings page duplicates plan cap table and fetches monitor count but has no explicit load error handling (`apps/web/app/(dashboard)/dashboard/[slug]/settings/page.tsx`).
- Existing test coverage includes route-level tests for plan parsing and shared enum tests.

### Key risks
- String-to-enum conversion in route path can produce internal errors for data integrity drift.
- Duplicated cap logic between backend and frontend can diverge.
- Settings page can silently fail to show usage if monitor fetch fails.
- Scope creep into billing/subscription schema is possible; must avoid broad rewrite in this pass.

### Improvement themes (prioritized)
1. Strongly type organization plan in backend domain model.
2. Centralize and simplify monitor cap enforcement logic in route.
3. Harden settings plan/usage rendering and error paths.
4. Keep tests aligned with new invariants.

## B) Constraints & Invariants (Repo-derived)
- Must preserve existing API route shapes (`/api/organizations/:slug/monitors`) and response wrappers.
- Must preserve free/pro/team plan semantics from migration constraint (`migrations/0002_organizations.sql`).
- Must not weaken auth/org access checks.
- Non-goals:
  - No Stripe integration in this pass.
  - No schema migration for new tables.
  - No broad frontend redesign.

## C) Proposed Changes by Theme (Prioritized)

### Theme 1: Typed organization plan
- Current approach: `Organization.plan: String`.
- Proposed: `Organization.plan: OrganizationPlan` with sqlx/serde enum.
- Why: Removes route parsing and aligns persistence with domain type.
- Tradeoff: Touches multiple modules and TS types.
- Scope: shared model + org access row + affected API/frontend types.
- Migration: none (DB already constrained values).

### Theme 2: Monitor cap enforcement cleanup
- Current approach: `parse_plan` helper in route.
- Proposed: use typed `org_access.org.plan` directly.
- Why: Fewer runtime error paths and cleaner code.
- Scope: `packages/api-server/src/routes/monitors.rs` and tests.

### Theme 3: Settings UX resilience
- Current approach: plan usage card with optimistic monitor fetch only.
- Proposed: handle fetch failures and avoid undefined limit rendering.
- Why: reduce silent UI degradation.
- Scope: settings page only.

### Theme 4: Test alignment
- Current approach: parse_plan tests.
- Proposed: replace with focused cap behavior unit tests/helpers where practical and maintain existing passing suite.

## D) File/Module Delta (Exact)
- MODIFY: `packages/shared/src/models/organization.rs` (typed plan field).
- MODIFY: `packages/api-server/src/middleware/org_access.rs` (typed plan in query row).
- MODIFY: `packages/api-server/src/routes/monitors.rs` (remove parse helper, enforce with typed enum).
- MODIFY: `apps/web/lib/types.ts` (organization plan type union).
- MODIFY: `apps/web/app/(dashboard)/dashboard/[slug]/settings/page.tsx` (usage card resilience).
- MODIFY: `codex/*.md` artifacts.

## E) Data Models & API Contracts (Delta)
- Current: API emits plan as string from DB model.
- Proposed: API still emits snake_case string via enum serde; contract remains backward-compatible for client.
- Compatibility: no breaking shape changes expected.
- Migrations: none.

## F) Implementation Sequence (Dependency-Explicit)
1. Type organization plan backend model.
   - Verify: `cargo test -p shared`, `cargo test -p api-server`.
   - Rollback: restore string type + parsing helper.
2. Refactor monitor route to typed plan enforcement.
   - Verify: `cargo test -p api-server`.
   - Rollback: restore previous route file.
3. Harden settings page loading/error handling and plan typing in TS.
   - Verify: `pnpm --filter web typecheck`, `pnpm --filter web test`.
   - Rollback: restore settings/types changes.
4. Final full verification.
   - Verify: workspace Rust tests + web typecheck/test/build.

## G) Error Handling & Edge Cases
- Unknown plan should be impossible under DB check constraint; typed model enforces this at decode layer.
- Settings monitor fetch failure should surface user-visible warning and preserve organization form usability.

## H) Integration & Testing Strategy
- Backend: rely on existing route tests + compile-time typing changes.
- Frontend: typecheck + existing tests, no weakening.
- DoD:
  - No string plan parsing helper remains in route.
  - Plan limit logic still enforced.
  - Settings page robust under partial fetch failure.

## I) Assumptions & Judgment Calls
- Assumption: DB `organizations.plan` contains only `free|pro|team`.
- Judgment call: avoid DB migration; leverage existing CHECK constraint.
