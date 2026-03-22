#!/usr/bin/env bash
set -euo pipefail

if [ "$#" -lt 1 ] || [ "$#" -gt 2 ]; then
  echo "usage: $0 <scenario> [output-json]" >&2
  exit 2
fi

scenario="$1"
output="${2:-target/perf/browser/${scenario}.json}"
script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
helper="${script_dir}/map_browser_profile.py"

mkdir -p "$(dirname "$output")"

cmd=(
  python3 "$helper"
  "$scenario"
  --url "${MAP_PROFILE_URL:-http://127.0.0.1:1990/map/}"
  --timeout-seconds "${MAP_PROFILE_TIMEOUT_SECS:-45}"
  --poll-interval-seconds "${MAP_PROFILE_POLL_INTERVAL_SECS:-0.25}"
  --output-json "$output"
)

if [ -n "${MAP_PROFILE_CAPTURE_FRAMES:-}" ]; then
  cmd+=(--capture-frames "${MAP_PROFILE_CAPTURE_FRAMES}")
fi

if command -v python3 >/dev/null 2>&1 && command -v chromium >/dev/null 2>&1; then
  "${cmd[@]}"
else
  devenv shell -- "${cmd[@]}"
fi

tools/scripts/perf-top-spans.sh "$output"
