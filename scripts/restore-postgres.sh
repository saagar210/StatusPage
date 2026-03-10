#!/usr/bin/env bash
set -euo pipefail

INPUT_PATH="${1:-}"

if [[ -z "$INPUT_PATH" || ! -f "$INPUT_PATH" ]]; then
  echo "Usage: bash scripts/restore-postgres.sh <backup.sql>"
  exit 2
fi

ENV_FILE="${STATUSPAGE_ENV_FILE:-.env.production}"
if [[ "$ENV_FILE" != /* ]]; then
  ENV_FILE="$(pwd)/$ENV_FILE"
fi

docker compose --env-file "$ENV_FILE" -f docker/docker-compose.prod.yml exec \
  -T \
  -e PGPASSWORD="${POSTGRES_PASSWORD:-change-me}" \
  postgres \
  psql \
    --set ON_ERROR_STOP=1 \
    -U "${POSTGRES_USER:-statuspage}" \
    "${POSTGRES_DB:-statuspage}" \
    -c "DROP SCHEMA IF EXISTS public CASCADE; CREATE SCHEMA public; GRANT ALL ON SCHEMA public TO ${POSTGRES_USER:-statuspage}; GRANT ALL ON SCHEMA public TO public;" >/dev/null

docker compose --env-file "$ENV_FILE" -f docker/docker-compose.prod.yml exec \
  -T \
  -e PGPASSWORD="${POSTGRES_PASSWORD:-change-me}" \
  postgres \
  psql \
    --set ON_ERROR_STOP=1 \
    -U "${POSTGRES_USER:-statuspage}" \
    "${POSTGRES_DB:-statuspage}" <"$INPUT_PATH"

echo "Restore completed from $INPUT_PATH"
