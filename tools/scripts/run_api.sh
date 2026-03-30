#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
API_BIND_HOST="${API_BIND_HOST:-127.0.0.1}"
API_PORT="${API_PORT:-8080}"
SECRETSPEC_API_PROFILE="${SECRETSPEC_API_PROFILE:-api}"

cd "$ROOT_DIR"

exec env LOG_TS_LABEL=api bash "$ROOT_DIR/tools/scripts/with_log_timestamps.sh" \
  secretspec run --profile "$SECRETSPEC_API_PROFILE" -- \
  cargo run --manifest-path "$ROOT_DIR/Cargo.toml" -p fishystuff_server -- \
  --config "$ROOT_DIR/api/config.toml" \
  --bind "${API_BIND_HOST}:${API_PORT}"
