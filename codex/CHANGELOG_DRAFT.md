# Changelog Draft

## Theme: Plan Limit Reliability
- Switched backend organization plan field to typed enum (`OrganizationPlan`) instead of raw `String`.
- Simplified monitor creation enforcement to consume typed plan directly.
- Removed runtime string parsing path from monitor route.

## Theme: Settings UX Robustness
- Tightened frontend organization plan typing to explicit union (`free | pro | team`).
- Added error handling in settings loader so org fetch failures surface clearly and monitor usage failures degrade gracefully.

## Testing / Validation
- Rust workspace tests pass.
- Web typecheck/tests pass.
- Web production build remains blocked by environment font-fetch connectivity.
