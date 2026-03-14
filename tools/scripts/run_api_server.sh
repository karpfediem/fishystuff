#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
source "$ROOT_DIR/tools/scripts/devenv_process_lib.sh"

API_BIND_HOST="${API_BIND_HOST:-127.0.0.1}"
API_PORT="${API_PORT:-8080}"

"$ROOT_DIR/tools/scripts/cleanup_api_server.sh"
devenv_notify_status "starting API server on ${API_BIND_HOST}:${API_PORT}"
devenv_run_with_http_ready \
  "http://${API_BIND_HOST}:${API_PORT}/api/v1/meta" \
  "API server ready on ${API_BIND_HOST}:${API_PORT}" \
  "$ROOT_DIR/tools/scripts/run_api.sh"
