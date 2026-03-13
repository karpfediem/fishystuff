#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
source "$ROOT_DIR/tools/scripts/devenv_process_lib.sh"

DB_HOST="${DB_HOST:-127.0.0.1}"
DB_PORT="${DB_PORT:-3306}"
API_BIND_HOST="${API_BIND_HOST:-127.0.0.1}"
API_PORT="${API_PORT:-8080}"
SECRETSPEC_API_PROFILE="${SECRETSPEC_API_PROFILE:-api}"

cd "$ROOT_DIR"

devenv_notify_status "waiting for Dolt SQL server on ${DB_HOST}:${DB_PORT}"
devenv_wait_for_tcp "$DB_HOST" "$DB_PORT" "Dolt SQL server" 240 0.25

devenv_notify_status "starting API server on ${API_BIND_HOST}:${API_PORT}"
devenv_run_with_tcp_ready \
  "$API_BIND_HOST" \
  "$API_PORT" \
  "API server ready on ${API_BIND_HOST}:${API_PORT}" \
  secretspec run --profile "$SECRETSPEC_API_PROFILE" -- \
    cargo run --manifest-path "$ROOT_DIR/Cargo.toml" -p fishystuff_server -- \
    --config "$ROOT_DIR/api/config.toml" \
    --bind "${API_BIND_HOST}:${API_PORT}"
