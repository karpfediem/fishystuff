#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT_DIR"

pids=()

cleanup() {
  local status=$?
  trap - EXIT INT TERM

  if ((${#pids[@]})); then
    for pid in "${pids[@]}"; do
      kill -- -"${pid}" >/dev/null 2>&1 || true
    done
    wait "${pids[@]}" 2>/dev/null || true
  fi

  exit "$status"
}

trap cleanup EXIT INT TERM

echo "[watch-builds] building current outputs"
just dev-build

echo "[watch-builds] launching map, CDN, and site rebuild watchers"
setsid just dev-watch-map &
pids+=("$!")
setsid just dev-watch-cdn &
pids+=("$!")
setsid just dev-watch-site &
pids+=("$!")

echo "[watch-builds] rebuild watchers running; stop with Ctrl-C"
wait -n "${pids[@]}"
