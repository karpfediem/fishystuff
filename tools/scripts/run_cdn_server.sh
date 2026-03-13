#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
CDN_ROOT="${CDN_ROOT:-$ROOT_DIR/data/cdn/public}"
CDN_HOST="${CDN_HOST:-127.0.0.1}"
CDN_PORT="${CDN_PORT:-4040}"

"$ROOT_DIR/tools/scripts/cleanup_cdn_server.sh"

exec python "$ROOT_DIR/tools/scripts/serve_cdn.py" \
  --root "$CDN_ROOT" \
  --host "$CDN_HOST" \
  --port "$CDN_PORT"
