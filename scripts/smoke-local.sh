#!/usr/bin/env bash
set -euo pipefail

API_URL="${API_URL:-http://localhost:4000}"
WEB_URL="${WEB_URL:-http://localhost:3000}"
STATUS_SLUG="${STATUS_SLUG:-demo}"

echo "Checking API health at ${API_URL}/health..."
curl --fail --silent --show-error "${API_URL}/health" >/dev/null

echo "Checking public status page at ${WEB_URL}/s/${STATUS_SLUG}..."
curl --fail --silent --show-error "${WEB_URL}/s/${STATUS_SLUG}" >/dev/null

echo "Local smoke checks passed."
