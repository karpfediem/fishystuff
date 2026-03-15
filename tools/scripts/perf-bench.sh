#!/usr/bin/env bash
set -euo pipefail

cmd=(
  cargo bench
  -p fishystuff_ui_bevy
  --bench perf_hotpaths
  --profile profiling
  -- --noplot
)

cmd+=("$@")

if command -v cargo >/dev/null 2>&1; then
  "${cmd[@]}"
else
  devenv shell -- "${cmd[@]}"
fi
