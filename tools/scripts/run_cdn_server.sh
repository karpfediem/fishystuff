#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
source "$ROOT_DIR/tools/scripts/devenv_process_lib.sh"

CDN_ROOT="${CDN_ROOT:-$ROOT_DIR/data/cdn/public}"
CDN_HOST="${CDN_HOST:-127.0.0.1}"
CDN_PORT="${CDN_PORT:-4040}"

"$ROOT_DIR/tools/scripts/cleanup_cdn_server.sh"

devenv_notify_status "starting CDN file server on ${CDN_HOST}:${CDN_PORT}"
devenv_run_with_tcp_ready \
  "$CDN_HOST" \
  "$CDN_PORT" \
  "CDN file server ready on ${CDN_HOST}:${CDN_PORT}" \
  python "$ROOT_DIR/tools/scripts/serve_cdn.py" \
    --root "$CDN_ROOT" \
    --host "$CDN_HOST" \
    --port "$CDN_PORT"
