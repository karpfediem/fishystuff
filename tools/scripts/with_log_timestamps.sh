#!/usr/bin/env bash
set -euo pipefail

if [ "$#" -eq 0 ]; then
  echo "usage: with_log_timestamps.sh <command> [args...]" >&2
  exit 64
fi

label="${LOG_TS_LABEL:-process}"
log_file="${LOG_TS_FILE:-}"

if [ -n "$log_file" ]; then
  mkdir -p "$(dirname "$log_file")"
fi

emit_line() {
  local line="$1"

  printf '%s\n' "$line"
  if [ -n "$log_file" ]; then
    printf '%s\n' "$line" >> "$log_file"
  fi
}

timestamp() {
  date +"%Y-%m-%dT%H:%M:%S%z"
}

emit_line "[$(timestamp)] [$label] starting: $*"

set +e
"$@" 2>&1 | gawk -v label="$label" '
  {
    printf("[%s] [%s] %s\n", strftime("%Y-%m-%dT%H:%M:%S%z"), label, $0);
    fflush();
  }
' | if [ -n "$log_file" ]; then
  tee -a "$log_file"
else
  cat
fi
status=${PIPESTATUS[0]}
set -e

if [ "$status" -eq 0 ]; then
  emit_line "[$(timestamp)] [$label] exited successfully"
else
  emit_line "[$(timestamp)] [$label] exited with status $status"
fi

exit "$status"
