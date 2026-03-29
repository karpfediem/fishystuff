#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
DB_HOST="${DB_HOST:-127.0.0.1}"
DB_PORT="${DB_PORT:-3306}"
API_BIND_HOST="${API_BIND_HOST:-127.0.0.1}"
API_PORT="${API_PORT:-8080}"
SECRETSPEC_API_PROFILE="${SECRETSPEC_API_PROFILE:-api}"

cd "$ROOT_DIR"

until (echo >"/dev/tcp/$DB_HOST/$DB_PORT") >/dev/null 2>&1; do
  sleep 0.25
done

exec secretspec run --profile "$SECRETSPEC_API_PROFILE" -- \
  cargo run --manifest-path "$ROOT_DIR/Cargo.toml" -p fishystuff_server -- \
  --config "$ROOT_DIR/api/config.toml" \
  --bind "${API_BIND_HOST}:${API_PORT}"
