#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
API_BIND_HOST="${API_BIND_HOST:-127.0.0.1}"
API_PORT="${API_PORT:-8080}"
OTEL_ENABLED="${FISHYSTUFF_OTEL_ENABLED:-true}"
OTEL_SERVICE_NAME="${FISHYSTUFF_OTEL_SERVICE_NAME:-fishystuff-api-local}"
OTEL_TRACES_ENDPOINT="${FISHYSTUFF_OTEL_TRACES_ENDPOINT:-http://127.0.0.1:4820/v1/traces}"
OTEL_SAMPLE_RATIO="${FISHYSTUFF_OTEL_SAMPLE_RATIO:-0.25}"

cd "$ROOT_DIR"

exec env \
  LOG_TS_LABEL=api \
  FISHYSTUFF_OTEL_ENABLED="$OTEL_ENABLED" \
  FISHYSTUFF_OTEL_SERVICE_NAME="$OTEL_SERVICE_NAME" \
  FISHYSTUFF_OTEL_TRACES_ENDPOINT="$OTEL_TRACES_ENDPOINT" \
  FISHYSTUFF_OTEL_SAMPLE_RATIO="$OTEL_SAMPLE_RATIO" \
  bash "$ROOT_DIR/tools/scripts/with_log_timestamps.sh" \
  cargo run --manifest-path "$ROOT_DIR/Cargo.toml" -p fishystuff_server -- \
  --config "$ROOT_DIR/api/config.toml" \
  --bind "${API_BIND_HOST}:${API_PORT}"
