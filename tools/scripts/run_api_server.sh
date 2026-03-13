#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"

"$ROOT_DIR/tools/scripts/cleanup_api_server.sh"
exec "$ROOT_DIR/tools/scripts/run_api.sh"
