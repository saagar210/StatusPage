#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

require_cmd() {
  if ! command -v "$1" >/dev/null 2>&1; then
    echo "Missing required command: $1"
    exit 1
  fi
}

require_cmd node
require_cmd pnpm
require_cmd cargo

if [[ ! -f .env && -f .env.example ]]; then
  echo "Creating .env from .env.example..."
  cp .env.example .env

  if command -v openssl >/dev/null 2>&1; then
    auth_secret="$(openssl rand -base64 32 | tr -d '\n')"
    perl -0pi -e "s|AUTH_SECRET=<random-32-char-string>|AUTH_SECRET=${auth_secret}|" .env
    echo "Generated AUTH_SECRET in .env."
  else
    echo "OpenSSL not found. Please set AUTH_SECRET manually in .env."
  fi
fi

echo "Installing JavaScript dependencies..."
pnpm install --frozen-lockfile

echo "Fetching Rust dependencies..."
cargo fetch --locked

if command -v docker >/dev/null 2>&1; then
  if docker compose version >/dev/null 2>&1; then
    echo "Docker and docker compose are available."
  else
    echo "Docker is installed but docker compose is unavailable."
  fi
else
  echo "Docker is not installed. Local database, Redis, and deployment smoke checks will stay unavailable until Docker is added."
fi

echo "Warming the minimum workspace state..."
pnpm exec turbo --version >/dev/null
pnpm --filter web exec next --version >/dev/null
cargo metadata --format-version 1 >/dev/null

if [[ -f .env ]]; then
  echo "Local env file present at .env."
  echo "Manual follow-up: fill in AUTH_GITHUB_ID and AUTH_GITHUB_SECRET before login flows."
fi

echo "Local environment setup complete."
