#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
source "$ROOT_DIR/tools/scripts/devenv_process_lib.sh"

DB_HOST="${DB_HOST:-127.0.0.1}"
DB_PORT="${DB_PORT:-3306}"

cd "$ROOT_DIR"
"$ROOT_DIR/tools/scripts/cleanup_db_server.sh"
devenv_notify_status "starting Dolt SQL server on ${DB_HOST}:${DB_PORT}"
devenv_run_with_tcp_ready \
  "$DB_HOST" \
  "$DB_PORT" \
  "Dolt SQL server ready on ${DB_HOST}:${DB_PORT}" \
  dolt sql-server --host "$DB_HOST" --port "$DB_PORT"
