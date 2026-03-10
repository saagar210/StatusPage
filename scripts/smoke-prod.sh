#!/usr/bin/env bash
set -euo pipefail

API_URL="${API_URL:-http://localhost:4000}"
WEB_URL="${WEB_URL:-http://localhost}"
STATUS_SLUG="${STATUS_SLUG:-}"

api_check() {
  curl --fail --silent --show-error "$1" >/dev/null
}

web_check() {
  curl --fail --silent --show-error --location --insecure "$1" >/dev/null
}

echo "Checking API health at ${API_URL}/health..."
api_check "${API_URL}/health"

echo "Checking API readiness at ${API_URL}/ready..."
api_check "${API_URL}/ready"

echo "Checking API ops summary at ${API_URL}/ops/summary..."
api_check "${API_URL}/ops/summary"

echo "Checking web health at ${WEB_URL}/api/health..."
web_check "${WEB_URL}/api/health"

if [[ -n "$STATUS_SLUG" ]]; then
  echo "Checking public status API at ${API_URL}/api/public/${STATUS_SLUG}/status..."
  api_check "${API_URL}/api/public/${STATUS_SLUG}/status"

  echo "Checking public status page at ${WEB_URL}/s/${STATUS_SLUG}..."
  web_check "${WEB_URL}/s/${STATUS_SLUG}"
fi

echo "Production smoke checks passed."
