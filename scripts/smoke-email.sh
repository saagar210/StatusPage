#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

if ! docker info >/dev/null 2>&1; then
  echo "Docker is required for email smoke validation. Start Docker or Colima first."
  exit 1
fi

export POSTGRES_PORT="${EMAIL_POSTGRES_PORT:-55432}"
export REDIS_PORT="${EMAIL_REDIS_PORT:-56379}"
export API_PORT="${EMAIL_API_PORT:-4400}"
export MAILHOG_SMTP_PORT="${EMAIL_SMTP_PORT:-51025}"
export MAILHOG_HTTP_PORT="${EMAIL_MAILHOG_HTTP_PORT:-58025}"
export DATABASE_URL="${EMAIL_DATABASE_URL:-postgresql://statuspage:statuspage@127.0.0.1:${POSTGRES_PORT}/statuspage}"
export REDIS_URL="${EMAIL_REDIS_URL:-redis://127.0.0.1:${REDIS_PORT}}"
export API_URL="${EMAIL_API_URL:-http://127.0.0.1:${API_PORT}}"
export APP_BASE_URL="${EMAIL_APP_BASE_URL:-http://127.0.0.1:3000}"
export SMOKE_SUBSCRIBER_EMAIL="${SMOKE_SUBSCRIBER_EMAIL:-subscriber-smoke@example.com}"
export SMOKE_INCIDENT_TITLE="${SMOKE_INCIDENT_TITLE:-Subscriber Email Smoke Incident}"
export EMAIL_FROM="${SMOKE_EMAIL_FROM:-alerts@statuspage.local}"
export MAILHOG_CONTAINER_NAME="${MAILHOG_CONTAINER_NAME:-statuspage-mailhog-smoke}"

tmpdir="$(mktemp -d)"
api_log="${tmpdir}/api.log"
mailhog_messages="${tmpdir}/mailhog-messages.json"

cleanup() {
  if [[ -n "${API_PID:-}" ]]; then
    kill "$API_PID" >/dev/null 2>&1 || true
    wait "$API_PID" 2>/dev/null || true
  fi

  docker rm -f "$MAILHOG_CONTAINER_NAME" >/dev/null 2>&1 || true

  if [[ "${KEEP_EMAIL_SMOKE_STACK:-false}" != "true" ]]; then
    docker compose -f docker/docker-compose.dev.yml down -v >/dev/null 2>&1 || true
  fi

  rm -rf "$tmpdir"
}
trap cleanup EXIT

postgres_container() {
  docker compose -f docker/docker-compose.dev.yml ps -q postgres
}

redis_container() {
  docker compose -f docker/docker-compose.dev.yml ps -q redis
}

wait_for_http() {
  local url="$1"
  local label="$2"
  for _ in $(seq 1 60); do
    if curl --fail --silent "$url" >/dev/null 2>&1; then
      return 0
    fi
    sleep 1
  done

  echo "$label did not become ready in time."
  return 1
}

mailhog_find_message() {
  local subject_filter="$1"

  curl --fail --silent "http://127.0.0.1:${MAILHOG_HTTP_PORT}/api/v2/messages" >"$mailhog_messages"
  SUBJECT_FILTER="$subject_filter" MAILHOG_MESSAGES_FILE="$mailhog_messages" node <<'NODE'
const fs = require("fs");

const file = process.env.MAILHOG_MESSAGES_FILE;
const subjectFilter = process.env.SUBJECT_FILTER || "";
const payload = JSON.parse(fs.readFileSync(file, "utf8"));
const items = Array.isArray(payload.items) ? payload.items : [];

function getSubject(item) {
  return item?.Content?.Headers?.Subject?.[0]
    || item?.MIME?.Headers?.Subject?.[0]
    || "";
}

function getBody(item) {
  const raw = item?.Raw?.Data;
  if (typeof raw === "string" && raw.length > 0) {
    return raw.replace(/=\r?\n/g, "").replace(/=3D/gi, "=");
  }

  const content = item?.Content?.Body;
  if (typeof content === "string" && content.length > 0) {
    return content.replace(/=\r?\n/g, "").replace(/=3D/gi, "=");
  }

  const parts = item?.MIME?.Parts;
  if (Array.isArray(parts)) {
    const textPart = parts.find((part) =>
      String(part?.Headers?.["Content-Type"]?.[0] || "").includes("text/plain")
    );
    if (textPart?.Body) {
      return textPart.Body;
    }
  }

  return "";
}

const match = items.find((item) => getSubject(item).includes(subjectFilter));
if (!match) {
  process.exit(1);
}

process.stdout.write(JSON.stringify({
  subject: getSubject(match),
  body: getBody(match),
}, null, 2));
NODE
}

wait_for_message() {
  local subject_filter="$1"

  for _ in $(seq 1 40); do
    if message_json="$(mailhog_find_message "$subject_filter" 2>/dev/null)"; then
      printf '%s' "$message_json"
      return 0
    fi
    sleep 1
  done

  return 1
}

extract_verification_token() {
  local message_json="$1"

  MESSAGE_JSON="$message_json" node <<'NODE'
const payload = JSON.parse(process.env.MESSAGE_JSON || "{}");
const body = payload.body || "";
const match = body.match(/verify\?token=([A-Za-z0-9-]+)/);
if (!match) {
  process.exit(1);
}
process.stdout.write(match[1]);
NODE
}

echo "Resetting local Postgres and Redis for email smoke..."
docker compose -f docker/docker-compose.dev.yml down -v >/dev/null 2>&1 || true

echo "Starting Postgres and Redis..."
docker compose -f docker/docker-compose.dev.yml up -d >/dev/null

echo "Waiting for PostgreSQL..."
until docker exec "$(postgres_container)" psql -U statuspage -d statuspage -c "SELECT 1" >/dev/null 2>&1; do
  sleep 1
done

echo "Waiting for Redis..."
until docker exec "$(redis_container)" redis-cli ping >/dev/null 2>&1; do
  sleep 1
done

echo "Starting MailHog..."
docker rm -f "$MAILHOG_CONTAINER_NAME" >/dev/null 2>&1 || true
docker run -d \
  --name "$MAILHOG_CONTAINER_NAME" \
  -p "${MAILHOG_SMTP_PORT}:1025" \
  -p "${MAILHOG_HTTP_PORT}:8025" \
  mailhog/mailhog >/dev/null

wait_for_http "http://127.0.0.1:${MAILHOG_HTTP_PORT}/api/v2/messages" "MailHog"

echo "Running migrations and seed..."
pnpm run db:migrate >/dev/null
pnpm run db:seed >/dev/null

SERVICE_ID="$(
  docker exec "$(postgres_container)" \
    psql -U statuspage -d statuspage -At \
    -c "SELECT id FROM services ORDER BY created_at ASC LIMIT 1"
)"

echo "Starting API server with SMTP enabled..."
API_PORT="$API_PORT" \
CORS_ORIGIN="http://localhost:3000" \
RUN_MIGRATIONS_ON_START="false" \
SMTP_HOST="127.0.0.1" \
SMTP_PORT="$MAILHOG_SMTP_PORT" \
SMTP_SECURE="false" \
EMAIL_FROM="$EMAIL_FROM" \
APP_BASE_URL="$APP_BASE_URL" \
EMAIL_DISPATCH_INTERVAL_SECS="${EMAIL_DISPATCH_INTERVAL_SECS:-1}" \
EMAIL_DISPATCH_BATCH_SIZE="${EMAIL_DISPATCH_BATCH_SIZE:-20}" \
cargo run -p api-server --bin api-server >"$api_log" 2>&1 &
API_PID=$!

if ! wait_for_http "${API_URL}/health" "API server"; then
  echo "API server did not become healthy. Recent log output:"
  tail -n 200 "$api_log" || true
  exit 1
fi

echo "Creating subscriber through the public API..."
curl --fail --silent -X POST "$API_URL/api/public/demo/subscribe" \
  -H "Content-Type: application/json" \
  --data "{\"email\":\"${SMOKE_SUBSCRIBER_EMAIL}\"}" \
  >/dev/null

verification_message="$(wait_for_message "Confirm your subscription to ")" || {
  echo "Verification email did not arrive in MailHog."
  docker exec "$(postgres_container)" psql -U statuspage -d statuspage \
    -c "SELECT notification_type, recipient_email, status, attempt_count, error_message FROM notification_logs ORDER BY created_at DESC LIMIT 10;" \
    || true
  tail -n 120 "$api_log" || true
  exit 1
}

verification_token="$(extract_verification_token "$verification_message")" || {
  echo "Could not extract a verification token from the subscriber email."
  printf '%s\n' "$verification_message"
  exit 1
}

echo "Verifying subscriber..."
curl --fail --silent "$API_URL/api/public/demo/subscribers/verify?token=${verification_token}" >/dev/null

echo "Creating deterministic test session..."
TEST_SESSION_TOKEN="$(
  DATABASE_URL="$DATABASE_URL" pnpm --filter web exec node e2e/scripts/create-test-session.mjs
)"
COOKIE="authjs.session-token=${TEST_SESSION_TOKEN}"

echo "Triggering incident-created email..."
curl --fail --silent -X POST "$API_URL/api/organizations/demo/incidents" \
  -H "Content-Type: application/json" \
  -H "Cookie: $COOKIE" \
  --data "{\"title\":\"${SMOKE_INCIDENT_TITLE}\",\"impact\":\"major\",\"message\":\"Smoke subscriber email test\",\"affected_service_ids\":[\"${SERVICE_ID}\"]}" \
  >/dev/null

incident_message="$(wait_for_message "${SMOKE_INCIDENT_TITLE}")" || {
  echo "Incident notification email did not arrive in MailHog."
  docker exec "$(postgres_container)" psql -U statuspage -d statuspage \
    -c "SELECT notification_type, recipient_email, status, attempt_count, error_message FROM notification_logs ORDER BY created_at DESC LIMIT 10;" \
    || true
  tail -n 120 "$api_log" || true
  exit 1
}

DELIVERY_COUNT="$(
  docker exec "$(postgres_container)" psql -U statuspage -d statuspage -At \
    -c "SELECT COUNT(*) FROM notification_logs WHERE recipient_email = '${SMOKE_SUBSCRIBER_EMAIL}' AND status = 'sent';"
)"

if [[ "$DELIVERY_COUNT" -lt 2 ]]; then
  echo "Expected at least two sent notification log rows, found ${DELIVERY_COUNT}."
  docker exec "$(postgres_container)" psql -U statuspage -d statuspage \
    -c "SELECT notification_type, recipient_email, status, attempt_count, error_message FROM notification_logs WHERE recipient_email = '${SMOKE_SUBSCRIBER_EMAIL}' ORDER BY created_at DESC;" \
    || true
  exit 1
fi

echo "Email smoke passed."
printf '%s\n' "$verification_message"
printf '%s\n' "$incident_message"
