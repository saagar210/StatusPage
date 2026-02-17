#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
LEAN_CACHE_DIR="${LEAN_CACHE_DIR:-$ROOT_DIR/.lean-cache}"
NEXT_LEAN_DIST_DIR="${NEXT_LEAN_DIST_DIR:-.next-lean}"

mkdir -p "$LEAN_CACHE_DIR"

export TURBO_CACHE_DIR="$LEAN_CACHE_DIR/turbo"
export CARGO_TARGET_DIR="$LEAN_CACHE_DIR/target"
export NEXT_DIST_DIR="$NEXT_LEAN_DIST_DIR"

api_pid=""
web_pid=""

cleanup() {
  local exit_code=$?

  if [ -n "$api_pid" ] && kill -0 "$api_pid" 2>/dev/null; then
    pkill -TERM -P "$api_pid" 2>/dev/null || true
    kill -TERM "$api_pid" 2>/dev/null || true
  fi

  if [ -n "$web_pid" ] && kill -0 "$web_pid" 2>/dev/null; then
    pkill -TERM -P "$web_pid" 2>/dev/null || true
    kill -TERM "$web_pid" 2>/dev/null || true
  fi

  wait "$api_pid" 2>/dev/null || true
  wait "$web_pid" 2>/dev/null || true

  "$ROOT_DIR/scripts/clean-local.sh" heavy >/dev/null || true

  exit "$exit_code"
}

trap cleanup EXIT INT TERM

cd "$ROOT_DIR"

pnpm run dev:api &
api_pid=$!

pnpm run dev:web &
web_pid=$!

while true; do
  if ! kill -0 "$api_pid" 2>/dev/null; then
    wait "$api_pid" 2>/dev/null || true
    break
  fi

  if ! kill -0 "$web_pid" 2>/dev/null; then
    wait "$web_pid" 2>/dev/null || true
    break
  fi

  sleep 1
done
