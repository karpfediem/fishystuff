#!/usr/bin/env bash
set -euo pipefail

if [ "$#" -lt 1 ] || [ "$#" -gt 2 ]; then
  echo "usage: $0 <scenario> [output-json]" >&2
  exit 2
fi

scenario="$1"
output="${2:-target/perf/${scenario}.json}"
trace_path="${PERF_TRACE_CHROME_PATH:-}"

mkdir -p "$(dirname "$output")"

cmd=(
  cargo run
  -p fishystuff_ui_bevy
  --profile profiling
  --bin profile_harness
  --
  --scenario "$scenario"
  --output "$output"
  --headless
)

if [ -n "$trace_path" ]; then
  mkdir -p "$(dirname "$trace_path")"
  cmd+=(--trace-chrome "$trace_path")
fi

run_cmd() {
  if command -v cargo >/dev/null 2>&1; then
    if [ -z "${DISPLAY:-}" ]; then
      if command -v xvfb-run >/dev/null 2>&1; then
        xvfb-run -a env \
          LIBGL_ALWAYS_SOFTWARE=1 \
          MESA_LOADER_DRIVER_OVERRIDE=llvmpipe \
          WGPU_BACKEND=gl \
          "${cmd[@]}"
      else
        devenv shell -- xvfb-run -a env \
          LIBGL_ALWAYS_SOFTWARE=1 \
          MESA_LOADER_DRIVER_OVERRIDE=llvmpipe \
          WGPU_BACKEND=gl \
          "${cmd[@]}"
      fi
    else
      "${cmd[@]}"
    fi
    return
  fi

  if [ -z "${DISPLAY:-}" ]; then
    devenv shell -- xvfb-run -a env \
      LIBGL_ALWAYS_SOFTWARE=1 \
      MESA_LOADER_DRIVER_OVERRIDE=llvmpipe \
      WGPU_BACKEND=gl \
      "${cmd[@]}"
  else
    devenv shell -- "${cmd[@]}"
  fi
}

run_cmd

tools/scripts/perf-top-spans.sh "$output"
