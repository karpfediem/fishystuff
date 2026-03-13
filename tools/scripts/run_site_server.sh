#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
source "$ROOT_DIR/tools/scripts/devenv_process_lib.sh"

SITE_HOST="${SITE_HOST:-127.0.0.1}"
SITE_PORT="${SITE_PORT:-1990}"

cd "$ROOT_DIR/site"
devenv_notify_status "starting local site server on ${SITE_HOST}:${SITE_PORT}"
devenv_run_with_tcp_ready \
  "$SITE_HOST" \
  "$SITE_PORT" \
  "site server ready at http://${SITE_HOST}:${SITE_PORT}/" \
  bun run ./scripts/serve-release.mjs --root .out --host "$SITE_HOST" --port "$SITE_PORT"
