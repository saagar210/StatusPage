#!/usr/bin/env bash
set -euo pipefail

API_URL="${API_URL:-http://localhost:4000}"
WEB_URL="${WEB_URL:-http://localhost}"
STATUS_SLUG="${STATUS_SLUG:-}"

retry_check() {
  local label="$1"
  local url="$2"
  local mode="${3:-api}"
  local last_error=""

  for _ in $(seq 1 20); do
    if [[ "$mode" == "web" ]]; then
      if curl --fail --silent --show-error --location --insecure "$url" >/dev/null 2>"$TMPDIR/statuspage-smoke.err"; then
        return 0
      fi
    else
      if curl --fail --silent --show-error "$url" >/dev/null 2>"$TMPDIR/statuspage-smoke.err"; then
        return 0
      fi
    fi

    last_error="$(cat "$TMPDIR/statuspage-smoke.err" 2>/dev/null || true)"
    sleep 1
  done

  echo "$label failed after retries." >&2
  if [[ -n "$last_error" ]]; then
    echo "$last_error" >&2
  fi
  return 1
}

TMPDIR="$(mktemp -d)"
trap 'rm -rf "$TMPDIR"' EXIT

echo "Checking API health at ${API_URL}/health..."
retry_check "API health" "${API_URL}/health"

echo "Checking API readiness at ${API_URL}/ready..."
retry_check "API readiness" "${API_URL}/ready"

echo "Checking API ops summary at ${API_URL}/ops/summary..."
retry_check "API ops summary" "${API_URL}/ops/summary"

echo "Checking web health at ${WEB_URL}/api/health..."
retry_check "Web health" "${WEB_URL}/api/health" web

if [[ -n "$STATUS_SLUG" ]]; then
  echo "Checking public status API at ${API_URL}/api/public/${STATUS_SLUG}/status..."
  retry_check "Public status API" "${API_URL}/api/public/${STATUS_SLUG}/status"

  echo "Checking public status page at ${WEB_URL}/s/${STATUS_SLUG}..."
  retry_check "Public status page" "${WEB_URL}/s/${STATUS_SLUG}" web
fi

echo "Production smoke checks passed."
