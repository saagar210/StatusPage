#!/usr/bin/env bash
set -euo pipefail

ENV_FILE="${STATUSPAGE_ENV_FILE:-.env.production}"
STATUS_SLUG="${STATUS_SLUG:-}"
TEMP_ENV="$(mktemp "${TMPDIR:-/tmp}/statuspage-rollback.XXXXXX.env")"

if [[ "$ENV_FILE" != /* ]]; then
  ENV_FILE="$(pwd)/$ENV_FILE"
fi

cleanup() {
  rm -f "$TEMP_ENV"
}
trap cleanup EXIT

cp "$ENV_FILE" "$TEMP_ENV"
python3 - "$TEMP_ENV" <<'PY'
from pathlib import Path
import sys

path = Path(sys.argv[1])
lines = path.read_text().splitlines()
updated = []
replaced = False
for line in lines:
    if line.startswith("DATABASE_URL="):
        updated.append("DATABASE_URL=postgresql://statuspage:change-me@invalid-db-host:5432/statuspage")
        replaced = True
    else:
        updated.append(line)
if not replaced:
    updated.append("DATABASE_URL=postgresql://statuspage:change-me@invalid-db-host:5432/statuspage")
path.write_text("\n".join(updated) + "\n")
PY

echo "Simulating a bad API rollout..."
STATUSPAGE_ENV_FILE="$TEMP_ENV" docker compose \
  --env-file "$TEMP_ENV" \
  -f docker/docker-compose.prod.yml \
  up -d --force-recreate --no-deps api-server

echo "Waiting for unhealthy API state..."
observed_failure=0
for _ in $(seq 1 20); do
  status="$(docker inspect --format '{{if .State.Health}}{{.State.Health.Status}}{{else}}{{.State.Status}}{{end}}' docker-api-server-1)"
  if [[ "$status" == "unhealthy" || "$status" == "exited" ]]; then
    echo "Observed rollback trigger state: $status"
    observed_failure=1
    break
  fi
  sleep 2
done

if [[ "$observed_failure" -ne 1 ]]; then
  echo "Did not observe an unhealthy API state during the bad rollout simulation." >&2
  exit 1
fi

echo "Rolling back to the known-good environment..."
STATUSPAGE_ENV_FILE="$ENV_FILE" docker compose \
  --env-file "$ENV_FILE" \
  -f docker/docker-compose.prod.yml \
  up -d --force-recreate --no-deps api-server web caddy monitor

echo "Waiting for healthy services after rollback..."
for service in docker-api-server-1 docker-web-1 docker-monitor-1; do
  for _ in $(seq 1 30); do
    status="$(docker inspect --format '{{if .State.Health}}{{.State.Health.Status}}{{else}}{{.State.Status}}{{end}}' "$service")"
    if [[ "$status" == "healthy" || "$status" == "running" ]]; then
      break
    fi
    sleep 2
  done
done

echo "Running post-rollback smoke checks..."
STATUS_SLUG="$STATUS_SLUG" STATUSPAGE_ENV_FILE="$ENV_FILE" bash scripts/smoke-prod.sh

echo "Rollback rehearsal completed successfully."
