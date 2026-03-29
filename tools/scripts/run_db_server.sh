#!/usr/bin/env bash
set -euo pipefail

DB_HOST="${DB_HOST:-127.0.0.1}"
DB_PORT="${DB_PORT:-3306}"

exec dolt sql-server --host "$DB_HOST" --port "$DB_PORT"
