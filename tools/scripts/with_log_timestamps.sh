#!/usr/bin/env bash
set -euo pipefail

if [ "$#" -eq 0 ]; then
  echo "usage: with_log_timestamps.sh <command> [args...]" >&2
  exit 64
fi

label="${LOG_TS_LABEL:-process}"

timestamp() {
  date +"%Y-%m-%dT%H:%M:%S%z"
}

printf '[%s] [%s] starting: %s\n' "$(timestamp)" "$label" "$*"

set +e
"$@" 2>&1 | gawk -v label="$label" '
  {
    printf("[%s] [%s] %s\n", strftime("%Y-%m-%dT%H:%M:%S%z"), label, $0);
    fflush();
  }
'
status=${PIPESTATUS[0]}
set -e

if [ "$status" -eq 0 ]; then
  printf '[%s] [%s] exited successfully\n' "$(timestamp)" "$label"
else
  printf '[%s] [%s] exited with status %d\n' "$(timestamp)" "$label" "$status"
fi

exit "$status"
