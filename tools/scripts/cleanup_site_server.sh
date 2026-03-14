#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
SITE_PORT="${SITE_PORT:-1990}"

find_listener_pids() {
  if ! command -v lsof >/dev/null 2>&1; then
    return 0
  fi

  lsof -tiTCP:"$SITE_PORT" -sTCP:LISTEN 2>/dev/null | LC_ALL=C sort -u
}

existing_pids="$(find_listener_pids || true)"
if [ -z "$existing_pids" ]; then
  exit 0
fi

while IFS= read -r pid; do
  [ -n "$pid" ] || continue
  cmdline="$(ps -o command= -p "$pid" 2>/dev/null || true)"
  if printf '%s' "$cmdline" | grep -Fq "$ROOT_DIR/site/scripts/serve-release.mjs"; then
    echo "stopping stale site server on 127.0.0.1:$SITE_PORT (pid $pid)"
    kill "$pid" >/dev/null 2>&1 || true
  else
    echo "Site port $SITE_PORT already in use by non-site process: ${cmdline:-pid $pid}" >&2
    exit 1
  fi
done <<EOF
$existing_pids
EOF

for _ in $(seq 1 20); do
  if [ -z "$(find_listener_pids || true)" ]; then
    exit 0
  fi
  sleep 0.25
done

remaining_pids="$(find_listener_pids || true)"
if [ -n "$remaining_pids" ]; then
  echo "Site port $SITE_PORT is still in use after stopping stale server(s): $remaining_pids" >&2
  exit 1
fi
