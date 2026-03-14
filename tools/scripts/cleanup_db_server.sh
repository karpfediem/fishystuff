#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
DB_PORT="${DB_PORT:-3306}"
self_pid="$$"
caller_pid="${PPID:-}"

find_listener_pids() {
  if ! command -v lsof >/dev/null 2>&1; then
    return 0
  fi

  lsof -tiTCP:"$DB_PORT" -sTCP:LISTEN 2>/dev/null | LC_ALL=C sort -u
}

find_managed_pids() {
  ps -eo pid=,command= 2>/dev/null | while read -r pid cmdline; do
    [ -n "$pid" ] || continue
    [ "$pid" = "$self_pid" ] && continue
    [ -n "$caller_pid" ] && [ "$pid" = "$caller_pid" ] && continue
    case "$cmdline" in
      *"$ROOT_DIR/tools/scripts/run_db_server.sh"*|*"./tools/scripts/run_db_server.sh"*|*"tools/scripts/run_db_server.sh"*|*"dolt sql-server --host 127.0.0.1 --port $DB_PORT"*|*"dolt sql-server --host localhost --port $DB_PORT"*)
        printf '%s\n' "$pid"
        ;;
    esac
  done | LC_ALL=C sort -u
}

stop_pid_if_managed() {
  local pid="$1"
  [ -n "$pid" ] || return 0
  [ "$pid" = "$self_pid" ] && return 0
  [ -n "$caller_pid" ] && [ "$pid" = "$caller_pid" ] && return 0
  local cmdline
  cmdline="$(ps -o command= -p "$pid" 2>/dev/null || true)"
  if [ -z "$cmdline" ]; then
    return 0
  fi
  case "$cmdline" in
    *"$ROOT_DIR/tools/scripts/run_db_server.sh"*|*"./tools/scripts/run_db_server.sh"*|*"tools/scripts/run_db_server.sh"*|*"dolt sql-server --host 127.0.0.1 --port $DB_PORT"*|*"dolt sql-server --host localhost --port $DB_PORT"*)
      echo "stopping stale DB process on 127.0.0.1:$DB_PORT (pid $pid)"
      kill "$pid" >/dev/null 2>&1 || true
      ;;
    *)
      echo "DB port $DB_PORT already in use by non-fishystuff process: ${cmdline:-pid $pid}" >&2
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
    echo "force-killing stale DB process (pid $pid)"
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
  echo "DB process cleanup did not finish cleanly. listeners=${remaining_listeners:-none} managed=${remaining_managed:-none}" >&2
  exit 1
fi
