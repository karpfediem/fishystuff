#!/usr/bin/env bash
set -euo pipefail

output="${1:-target/smoke/map-browser.json}"
script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
helper="${script_dir}/map_browser_smoke.py"

mkdir -p "$(dirname "$output")"

cmd=(
  python3 "$helper"
  --url "${MAP_SMOKE_URL:-http://127.0.0.1:1990/map/}"
  --timeout-seconds "${MAP_SMOKE_TIMEOUT_SECS:-30}"
  --output-json "$output"
)

if command -v python3 >/dev/null 2>&1 && command -v chromium >/dev/null 2>&1; then
  "${cmd[@]}"
else
  devenv shell -- "${cmd[@]}"
fi
