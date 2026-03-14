#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

if ! docker info >/dev/null 2>&1; then
  echo "Docker is required for managed smoke. Start Docker or Colima first."
  exit 1
fi

if [[ -f .env ]]; then
  set -a
  # shellcheck disable=SC1091
  source .env
  set +a
fi

export POSTGRES_PORT="${E2E_POSTGRES_PORT:-55432}"
export REDIS_PORT="${E2E_REDIS_PORT:-56379}"
export API_PORT="${E2E_API_PORT:-4400}"
export DATABASE_URL="${E2E_DATABASE_URL:-postgresql://statuspage:statuspage@127.0.0.1:${POSTGRES_PORT}/statuspage}"
export REDIS_URL="${E2E_REDIS_URL:-redis://127.0.0.1:${REDIS_PORT}}"
export INTERNAL_API_URL="${E2E_INTERNAL_API_URL:-http://127.0.0.1:${API_PORT}}"
export NEXT_PUBLIC_API_URL="${E2E_PUBLIC_API_URL:-http://127.0.0.1:${API_PORT}}"
export API_URL="${E2E_API_URL:-http://127.0.0.1:${API_PORT}}"
export NEXTAUTH_URL="${E2E_NEXTAUTH_URL:-http://127.0.0.1:3000}"
export AUTH_SECRET="${E2E_AUTH_SECRET:-playwright-secret}"
export AUTH_GITHUB_ID="${E2E_AUTH_GITHUB_ID:-playwright}"
export AUTH_GITHUB_SECRET="${E2E_AUTH_GITHUB_SECRET:-playwright}"
export INTERNAL_ADMIN_TOKEN="${INTERNAL_ADMIN_TOKEN:-managed-admin-token}"
export E2E_WITH_BACKEND="true"

API_LOG="${ROOT_DIR}/.codex/api-managed-smoke.log"
cleanup() {
  if [[ -n "${API_PID:-}" ]]; then
    kill "$API_PID" >/dev/null 2>&1 || true
    wait "$API_PID" 2>/dev/null || true
  fi

  if [[ "${KEEP_E2E_STACK:-false}" != "true" ]]; then
    docker compose -f docker/docker-compose.dev.yml down -v >/dev/null 2>&1 || true
  fi
}
trap cleanup EXIT

echo "Resetting local Postgres and Redis for managed smoke..."
docker compose -f docker/docker-compose.dev.yml down -v >/dev/null 2>&1 || true

echo "Starting Postgres and Redis..."
docker compose -f docker/docker-compose.dev.yml up -d >/dev/null

postgres_container() {
  docker compose -f docker/docker-compose.dev.yml ps -q postgres
}

redis_container() {
  docker compose -f docker/docker-compose.dev.yml ps -q redis
}

echo "Waiting for PostgreSQL..."
until docker exec "$(postgres_container)" psql -U statuspage -d statuspage -c "SELECT 1" >/dev/null 2>&1; do
  sleep 1
done

echo "Waiting for Redis..."
until docker exec "$(redis_container)" redis-cli ping >/dev/null 2>&1; do
  sleep 1
done

echo "Running migrations..."
pnpm run db:migrate >/dev/null

echo "Seeding demo data..."
pnpm run db:seed >/dev/null

echo "Starting API server..."
API_PORT="$API_PORT" \
CORS_ORIGIN="$NEXTAUTH_URL" \
RUN_MIGRATIONS_ON_START="false" \
INTERNAL_ADMIN_TOKEN="$INTERNAL_ADMIN_TOKEN" \
cargo run -p api-server --bin api-server >"$API_LOG" 2>&1 &
API_PID=$!

startup_attempts=0
until curl --fail --silent --show-error "${API_URL}/health" >/dev/null; do
  if ! kill -0 "$API_PID" >/dev/null 2>&1; then
    echo "API server exited during startup. Recent log output:"
    tail -n 200 "$API_LOG" || true
    exit 1
  fi

  startup_attempts=$((startup_attempts + 1))
  if [[ "$startup_attempts" -ge 60 ]]; then
    echo "API server did not become healthy within 60 seconds. Recent log output:"
    tail -n 200 "$API_LOG" || true
    exit 1
  fi

  sleep 1
done

echo "Creating deterministic test session..."
TEST_SESSION_TOKEN="$(
  pnpm --filter web exec node e2e/scripts/create-test-session.mjs
)"
export TEST_SESSION_TOKEN

echo "Running managed Playwright smoke..."
pnpm --filter web exec playwright test managed.spec.ts
