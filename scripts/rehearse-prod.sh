#!/usr/bin/env bash
set -euo pipefail

ENV_FILE="${STATUSPAGE_ENV_FILE:-.env.production}"
STATUS_SLUG="${STATUS_SLUG:-}"

if [[ "$ENV_FILE" != /* ]]; then
  ENV_FILE="$(pwd)/$ENV_FILE"
fi

echo "Resetting production rehearsal stack..."
docker compose --env-file "$ENV_FILE" -f docker/docker-compose.prod.yml down -v --remove-orphans

echo "Building and starting production rehearsal stack..."
docker compose --env-file "$ENV_FILE" -f docker/docker-compose.prod.yml up -d --build

echo "Running production smoke checks..."
STATUS_SLUG="$STATUS_SLUG" STATUSPAGE_ENV_FILE="$ENV_FILE" bash scripts/smoke-prod.sh

echo "Production rehearsal completed successfully."
