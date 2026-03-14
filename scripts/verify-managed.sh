#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

echo "Running managed backend checks..."
cargo test -p api-server services::downgrade::tests
cargo test -p api-server routes::invitations::tests
cargo test -p api-server routes::admin::tests

echo "Running managed frontend checks..."
pnpm --filter web test
pnpm --filter web typecheck
