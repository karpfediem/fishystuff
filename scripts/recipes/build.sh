#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "${SCRIPT_DIR}/lib/common.sh"

cd "$RECIPE_REPO_ROOT"

format_elapsed() {
  local elapsed="${1:-0}"
  if (( elapsed >= 60 )); then
    printf '%dm%02ds' "$((elapsed / 60))" "$((elapsed % 60))"
    return
  fi
  printf '%ds' "$elapsed"
}

run_step() {
  local label="$1"
  shift

  local started="$SECONDS"
  echo "[build] starting ${label}"
  if "$@"; then
    local elapsed="$((SECONDS - started))"
    echo "[build] finished ${label} in $(format_elapsed "$elapsed")"
    return 0
  else
    local status="$?"
    local elapsed="$((SECONDS - started))"
    echo "[build] failed ${label} after $(format_elapsed "$elapsed")" >&2
    return "$status"
  fi
}

wait_step() {
  local label="$1"
  local pid="$2"
  local started="$3"

  echo "[build] waiting for ${label}"
  if wait "$pid"; then
    local elapsed="$((SECONDS - started))"
    echo "[build] finished ${label} in $(format_elapsed "$elapsed")"
    return 0
  else
    local status="$?"
    local elapsed="$((SECONDS - started))"
    echo "[build] failed ${label} after $(format_elapsed "$elapsed")" >&2
    return "$status"
  fi
}

background_pids=()

cleanup() {
  local pid=""
  for pid in "${background_pids[@]}"; do
    kill "$pid" 2>/dev/null || true
  done
}

trap cleanup EXIT

site_started="$SECONDS"
echo "[build] starting build-site in background"
just build-site &
site_pid="$!"
background_pids+=("$site_pid")

status=0
run_step "build-map" just build-map || status=1
if (( status == 0 )); then
  run_step "cdn-stage-icons" just cdn-stage-icons || status=1
else
  echo "[build] skipping cdn-stage-icons because build-map failed" >&2
fi

if ! wait_step "build-site" "$site_pid" "$site_started"; then
  status=1
fi

trap - EXIT
exit "$status"
