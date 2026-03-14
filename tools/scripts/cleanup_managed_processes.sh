#!/usr/bin/env bash
set -euo pipefail

if [ "$#" -lt 2 ]; then
  echo "usage: $0 <label> <pattern> [pattern ...]" >&2
  exit 1
fi

label="$1"
shift

self_pid="$$"
caller_pid="${PPID:-}"

find_matching_pids() {
  ps -eo pid=,command= 2>/dev/null | while read -r pid cmdline; do
    [ -n "$pid" ] || continue
    [ "$pid" = "$self_pid" ] && continue
    [ -n "$caller_pid" ] && [ "$pid" = "$caller_pid" ] && continue

    for pattern in "$@"; do
      case "$cmdline" in
        *"$pattern"*)
          printf '%s\n' "$pid"
          break
          ;;
      esac
    done
  done | LC_ALL=C sort -u
}

stop_pid() {
  local pid="$1"
  [ -n "$pid" ] || return 0
  if ps -p "$pid" >/dev/null 2>&1; then
    echo "stopping stale $label (pid $pid)"
    kill "$pid" >/dev/null 2>&1 || true
  fi
}

matching_pids="$(find_matching_pids "$@" || true)"
if [ -z "$matching_pids" ]; then
  exit 0
fi

while IFS= read -r pid; do
  stop_pid "$pid"
done <<EOF
$matching_pids
EOF

for _ in $(seq 1 20); do
  if [ -z "$(find_matching_pids "$@" || true)" ]; then
    exit 0
  fi
  sleep 0.25
done

remaining_pids="$(find_matching_pids "$@" || true)"
if [ -n "$remaining_pids" ]; then
  while IFS= read -r pid; do
    [ -n "$pid" ] || continue
    echo "force-killing stale $label (pid $pid)"
    kill -9 "$pid" >/dev/null 2>&1 || true
  done <<EOF
$remaining_pids
EOF
fi
