#!/usr/bin/env bash
set -euo pipefail

MODE="${1:-heavy}"
ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

remove_paths() {
  local path
  for path in "$@"; do
    if [ -e "$path" ]; then
      rm -rf "$path"
      echo "removed $path"
    else
      echo "skip $path (missing)"
    fi
  done
}

heavy_cleanup() {
  remove_paths \
    "$ROOT_DIR/.turbo" \
    "$ROOT_DIR/target" \
    "$ROOT_DIR/.lean-cache" \
    "$ROOT_DIR/apps/web/.next" \
    "$ROOT_DIR/apps/web/.next-lean" \
    "$ROOT_DIR/apps/web/coverage" \
    "$ROOT_DIR/apps/web/test-results" \
    "$ROOT_DIR/apps/web/playwright-report" \
    "$ROOT_DIR/coverage"
}

full_cleanup() {
  heavy_cleanup
  remove_paths \
    "$ROOT_DIR/node_modules" \
    "$ROOT_DIR/apps/web/node_modules" \
    "$ROOT_DIR/packages/api-server/node_modules" \
    "$ROOT_DIR/packages/shared/node_modules" \
    "$ROOT_DIR/apps/monitor/node_modules"
}

case "$MODE" in
  heavy)
    heavy_cleanup
    ;;
  full)
    full_cleanup
    ;;
  *)
    echo "Usage: scripts/clean-local.sh [heavy|full]" >&2
    exit 1
    ;;
esac
