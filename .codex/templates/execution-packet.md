# Execution Packet Template

Use this template for each implementation batch so execution, verification, and rollback stay consistent.

## Objective
- What user-visible outcome this batch delivers

## Scope
- Files, routes, services, or commands expected to change

## Interfaces Affected
- API contracts, environment variables, scripts, schemas, routes, or jobs touched

## Implementation Tasks
1. Primary change
2. Supporting change
3. Verification change

## Verification Plan
- Commands to run
- Runtime paths to smoke manually or automatically

## Risks
- Main failure mode
- Fallback or containment plan

## Rollback Note
- What to revert or disable if the batch fails after merge

## Done Criteria
- Functional behavior complete
- Verification commands pass
- Docs or scripts updated if behavior changed
