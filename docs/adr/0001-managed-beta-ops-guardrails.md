# 0001. Managed Beta Ops Guardrails

## Status
Accepted

## Context
The managed beta now supports billing, invitations, custom domains, and internal operator actions. Those workflows need better abuse protection, better support visibility, and a practical operator surface without depending on raw SQL or ad hoc scripts.

## Decision
- Use Redis-backed rate limiting for public subscribe, verify, unsubscribe, and invitation-accept flows.
- Keep an in-memory fallback so temporary Redis issues do not fully block customer-facing traffic.
- Record high-value managed-beta actions in an `audit_logs` table.
- Expose recent audit entries and failed delivery activity through the internal support APIs.
- Provide a lightweight authenticated dashboard console at `/dashboard/internal-support` that forwards the internal admin token through the existing proxy layer.

## Consequences
- Multi-instance deployments get a shared rate-limit source of truth during normal operation.
- Operators gain a first-class history of billing sync, invitation, webhook, custom-domain, and retry actions.
- The fallback limiter preserves availability during Redis trouble, but it is less precise than the primary distributed path.
- Internal support remains token-gated and still requires operator care because the browser UI handles privileged data.

## Alternatives Considered
- In-memory-only rate limiting: simpler, but weak for multi-instance managed hosting.
- API-only support tooling: functional, but slower and more error-prone for operator workflows.
- Full admin backoffice before beta: higher polish, but too much surface area for this stage.
