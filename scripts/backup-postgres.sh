#!/usr/bin/env bash
set -euo pipefail

OUTPUT_PATH="${1:-backups/statuspage-$(date +%Y%m%dT%H%M%S).sql}"
mkdir -p "$(dirname "$OUTPUT_PATH")"

ENV_FILE="${STATUSPAGE_ENV_FILE:-.env.production}"
if [[ "$ENV_FILE" != /* ]]; then
  ENV_FILE="$(pwd)/$ENV_FILE"
fi

docker compose --env-file "$ENV_FILE" -f docker/docker-compose.prod.yml exec \
  -T \
  -e PGPASSWORD="${POSTGRES_PASSWORD:-change-me}" \
  postgres \
  pg_dump \
    --no-owner \
    --no-privileges \
    -U "${POSTGRES_USER:-statuspage}" \
    "${POSTGRES_DB:-statuspage}" >"$OUTPUT_PATH"

echo "Backup written to $OUTPUT_PATH"
