#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

if ! docker info >/dev/null 2>&1; then
  echo "Docker is required for webhook smoke validation. Start Docker or Colima first."
  exit 1
fi

export POSTGRES_PORT="${WEBHOOK_POSTGRES_PORT:-55432}"
export REDIS_PORT="${WEBHOOK_REDIS_PORT:-56379}"
export API_PORT="${WEBHOOK_API_PORT:-4400}"
export WEBHOOK_RECEIVER_PORT="${WEBHOOK_RECEIVER_PORT:-4455}"
export DATABASE_URL="${WEBHOOK_DATABASE_URL:-postgresql://statuspage:statuspage@127.0.0.1:${POSTGRES_PORT}/statuspage}"
export REDIS_URL="${WEBHOOK_REDIS_URL:-redis://127.0.0.1:${REDIS_PORT}}"
export API_URL="${WEBHOOK_API_URL:-http://127.0.0.1:${API_PORT}}"

tmpdir="$(mktemp -d)"
receiver_log="${tmpdir}/receiver.json"
receiver_pid_file="${tmpdir}/receiver.pid"
api_log="${tmpdir}/api.log"

cleanup() {
  if [[ -f "$receiver_pid_file" ]]; then
    receiver_pid="$(cat "$receiver_pid_file")"
    kill "$receiver_pid" >/dev/null 2>&1 || true
    wait "$receiver_pid" 2>/dev/null || true
  fi

  if [[ -n "${API_PID:-}" ]]; then
    kill "$API_PID" >/dev/null 2>&1 || true
    wait "$API_PID" 2>/dev/null || true
  fi

  if [[ "${KEEP_WEBHOOK_SMOKE_STACK:-false}" != "true" ]]; then
    docker compose -f docker/docker-compose.dev.yml down -v >/dev/null 2>&1 || true
  fi
}
trap cleanup EXIT

echo "Resetting local Postgres and Redis for webhook smoke..."
docker compose -f docker/docker-compose.dev.yml down -v >/dev/null 2>&1 || true

echo "Starting Postgres and Redis..."
docker compose -f docker/docker-compose.dev.yml up -d >/dev/null

postgres_container() {
  docker compose -f docker/docker-compose.dev.yml ps -q postgres
}

redis_container() {
  docker compose -f docker/docker-compose.dev.yml ps -q redis
}

until docker exec "$(postgres_container)" psql -U statuspage -d statuspage -c "SELECT 1" >/dev/null 2>&1; do
  sleep 1
done

until docker exec "$(redis_container)" redis-cli ping >/dev/null 2>&1; do
  sleep 1
done

echo "Running migrations and seed..."
pnpm run db:migrate >/dev/null
pnpm run db:seed >/dev/null

SERVICE_ID="$(
  docker exec "$(postgres_container)" \
    psql -U statuspage -d statuspage -At \
    -c "SELECT id FROM services ORDER BY created_at ASC LIMIT 1"
)"

node -e '
const fs = require("fs");
const http = require("http");
const outfile = process.argv[1];
const port = Number(process.argv[2]);
const server = http.createServer((req, res) => {
  let body = "";
  req.on("data", chunk => {
    body += chunk;
  });
  req.on("end", () => {
    fs.writeFileSync(outfile, JSON.stringify({
      method: req.method,
      url: req.url,
      headers: req.headers,
      body,
    }, null, 2));
    res.writeHead(200, { "content-type": "application/json" });
    res.end(JSON.stringify({ ok: true }));
  });
});
server.listen(port, "127.0.0.1");
' "$receiver_log" "$WEBHOOK_RECEIVER_PORT" >/dev/null 2>&1 &
echo $! > "$receiver_pid_file"

echo "Starting API server with webhook dispatcher..."
API_PORT="$API_PORT" \
CORS_ORIGIN="http://localhost:3000" \
RUN_MIGRATIONS_ON_START="false" \
cargo run -p api-server --bin api-server >"$api_log" 2>&1 &
API_PID=$!

until curl --fail --silent "$API_URL/health" >/dev/null; do
  if ! kill -0 "$API_PID" >/dev/null 2>&1; then
    echo "API server exited during startup. Recent log output:"
    tail -n 200 "$api_log" || true
    exit 1
  fi
  sleep 1
done

echo "Creating deterministic test session..."
TEST_SESSION_TOKEN="$(
  DATABASE_URL="$DATABASE_URL" pnpm --filter web exec node e2e/scripts/create-test-session.mjs
)"
COOKIE="authjs.session-token=${TEST_SESSION_TOKEN}"

echo "Creating webhook config..."
curl --fail --silent -X POST "$API_URL/api/organizations/demo/notifications/webhooks" \
  -H "Content-Type: application/json" \
  -H "Cookie: $COOKIE" \
  --data "{\"name\":\"Smoke Receiver\",\"url\":\"http://127.0.0.1:${WEBHOOK_RECEIVER_PORT}\",\"secret\":\"supersecret123\",\"event_types\":[\"incident.created\"],\"is_enabled\":true}" \
  >/dev/null

echo "Triggering incident.created event..."
curl --fail --silent -X POST "$API_URL/api/organizations/demo/incidents" \
  -H "Content-Type: application/json" \
  -H "Cookie: $COOKIE" \
  --data "{\"title\":\"Webhook Smoke Incident\",\"impact\":\"major\",\"message\":\"Smoke delivery test\",\"affected_service_ids\":[\"${SERVICE_ID}\"]}" \
  >/dev/null

for _ in $(seq 1 20); do
  if [[ -f "$receiver_log" ]]; then
    break
  fi
  sleep 1
done

if [[ ! -f "$receiver_log" ]]; then
  echo "Webhook receiver did not receive a request within the timeout."
  docker exec "$(postgres_container)" psql -U statuspage -d statuspage \
    -c "SELECT event_type, status, attempt_count, next_retry_at, error_message, response_status_code FROM webhook_deliveries ORDER BY created_at DESC LIMIT 5;" \
    || true
  tail -n 120 "$api_log" || true
  exit 1
fi

DELIVERY_STATUS="$(
  docker exec "$(postgres_container)" psql -U statuspage -d statuspage -At \
    -c "SELECT status FROM webhook_deliveries ORDER BY created_at DESC LIMIT 1"
)"

if [[ "$DELIVERY_STATUS" != "success" ]]; then
  echo "Webhook receiver got a request, but delivery status is '$DELIVERY_STATUS'."
  docker exec "$(postgres_container)" psql -U statuspage -d statuspage \
    -c "SELECT event_type, status, attempt_count, next_retry_at, error_message, response_status_code FROM webhook_deliveries ORDER BY created_at DESC LIMIT 5;" \
    || true
  tail -n 120 "$api_log" || true
  exit 1
fi

echo "Webhook smoke passed."
cat "$receiver_log"
