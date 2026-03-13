#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
API_PORT="${API_PORT:-8080}"

find_listener_pids() {
  if ! command -v lsof >/dev/null 2>&1; then
    return 0
  fi

  lsof -tiTCP:"$API_PORT" -sTCP:LISTEN 2>/dev/null | LC_ALL=C sort -u
}

find_managed_pids() {
  ps -eo pid=,command= 2>/dev/null | while read -r pid cmdline; do
    [ -n "$pid" ] || continue
    case "$cmdline" in
      *"$ROOT_DIR/tools/scripts/watch_api.sh"*|*"$ROOT_DIR/tools/scripts/run_api.sh"*|*"./tools/scripts/watch_api.sh"*|*"./tools/scripts/run_api.sh"*|*"tools/scripts/watch_api.sh"*|*"tools/scripts/run_api.sh"*|*"$ROOT_DIR/api/config.toml"*fishystuff_server*|*"fishystuff_server --config $ROOT_DIR/api/config.toml"*|*"$ROOT_DIR/target/debug/fishystuff_server"*|*"$ROOT_DIR/target/release/fishystuff_server"*|*"cargo run --manifest-path $ROOT_DIR/Cargo.toml -p fishystuff_server"*)
        printf '%s\n' "$pid"
        ;;
    esac
  done | LC_ALL=C sort -u
}

stop_pid_if_managed() {
  local pid="$1"
  [ -n "$pid" ] || return 0
  local cmdline
  cmdline="$(ps -o command= -p "$pid" 2>/dev/null || true)"
  if [ -z "$cmdline" ]; then
    return 0
  fi
  case "$cmdline" in
    *"$ROOT_DIR/tools/scripts/watch_api.sh"*|*"$ROOT_DIR/tools/scripts/run_api.sh"*|*"./tools/scripts/watch_api.sh"*|*"./tools/scripts/run_api.sh"*|*"tools/scripts/watch_api.sh"*|*"tools/scripts/run_api.sh"*|*"$ROOT_DIR/api/config.toml"*fishystuff_server*|*"fishystuff_server --config $ROOT_DIR/api/config.toml"*|*"$ROOT_DIR/target/debug/fishystuff_server"*|*"$ROOT_DIR/target/release/fishystuff_server"*|*"cargo run --manifest-path $ROOT_DIR/Cargo.toml -p fishystuff_server"*)
      echo "stopping stale API process on 127.0.0.1:$API_PORT (pid $pid)"
      kill "$pid" >/dev/null 2>&1 || true
      ;;
    *)
      echo "API port $API_PORT already in use by non-fishystuff process: ${cmdline:-pid $pid}" >&2
      exit 1
      ;;
  esac
}

stale_listener_pids="$(find_listener_pids || true)"
if [ -n "$stale_listener_pids" ]; then
  while IFS= read -r pid; do
    stop_pid_if_managed "$pid"
  done <<EOF
$stale_listener_pids
EOF
fi

managed_pids="$(find_managed_pids || true)"
if [ -n "$managed_pids" ]; then
  while IFS= read -r pid; do
    stop_pid_if_managed "$pid"
  done <<EOF
$managed_pids
EOF
fi

for _ in $(seq 1 20); do
  if [ -z "$(find_listener_pids || true)" ] && [ -z "$(find_managed_pids || true)" ]; then
    exit 0
  fi
  sleep 0.25
done

remaining_listeners="$(find_listener_pids || true)"
remaining_managed="$(find_managed_pids || true)"
if [ -n "$remaining_listeners$remaining_managed" ]; then
  while IFS= read -r pid; do
    [ -n "$pid" ] || continue
    echo "force-killing stale API process (pid $pid)"
    kill -9 "$pid" >/dev/null 2>&1 || true
  done <<EOF
$remaining_managed
EOF
fi

for _ in $(seq 1 10); do
  if [ -z "$(find_listener_pids || true)" ] && [ -z "$(find_managed_pids || true)" ]; then
    exit 0
  fi
  sleep 0.1
done

remaining_listeners="$(find_listener_pids || true)"
remaining_managed="$(find_managed_pids || true)"
if [ -n "$remaining_listeners$remaining_managed" ]; then
  echo "API process cleanup did not finish cleanly. listeners=${remaining_listeners:-none} managed=${remaining_managed:-none}" >&2
  exit 1
fi
