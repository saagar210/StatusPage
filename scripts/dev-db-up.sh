#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

docker compose --env-file .env -f docker/docker-compose.dev.yml up -d

postgres_container() {
  docker compose --env-file .env -f docker/docker-compose.dev.yml ps -q postgres
}

redis_container() {
  docker compose --env-file .env -f docker/docker-compose.dev.yml ps -q redis
}

echo "Waiting for PostgreSQL..."
until docker exec "$(postgres_container)" psql -U statuspage -d statuspage -c "SELECT 1" >/dev/null 2>&1; do
  sleep 1
done

echo "Waiting for Redis..."
until docker exec "$(redis_container)" redis-cli ping >/dev/null 2>&1; do
  sleep 1
done

echo "Development database services are ready."
