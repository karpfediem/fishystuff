#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"

SITE_HOST="${SITE_HOST:-127.0.0.1}"
SITE_PORT="${SITE_PORT:-1990}"

cd "$ROOT_DIR/site"
exec bun run ./scripts/serve-release.mjs --root .out --host "$SITE_HOST" --port "$SITE_PORT"
