#!/usr/bin/env bash

set -u

DEVENV_PROCESS_LIB_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
DEVENV_NOTIFY_BIN="${DEVENV_NOTIFY_BIN:-$DEVENV_PROCESS_LIB_DIR/devenv_notify.py}"

devenv_notify_ready() {
  local message="${1:-}"
  python "$DEVENV_NOTIFY_BIN" ready "$message" >/dev/null 2>&1 || true
}

devenv_notify_status() {
  local message="${1:-}"
  python "$DEVENV_NOTIFY_BIN" status "$message" >/dev/null 2>&1 || true
}

devenv_wait_for_tcp() {
  local host="$1"
  local port="$2"
  local label="${3:-$host:$port}"
  local attempts="${4:-240}"
  local sleep_seconds="${5:-0.25}"

  local attempt
  for attempt in $(seq 1 "$attempts"); do
    if (echo >"/dev/tcp/$host/$port") >/dev/null 2>&1; then
      return 0
    fi
    sleep "$sleep_seconds"
  done

  echo "timed out waiting for $label on $host:$port" >&2
  return 1
}

devenv_run_with_tcp_ready() {
  local host="$1"
  local port="$2"
  local ready_message="$3"
  shift 3

  "$@" &
  local child_pid=$!
  local child_status=0

  forward_terminate() {
    kill -TERM "$child_pid" >/dev/null 2>&1 || true
  }
  trap forward_terminate TERM INT HUP EXIT

  while kill -0 "$child_pid" >/dev/null 2>&1; do
    if (echo >"/dev/tcp/$host/$port") >/dev/null 2>&1; then
      devenv_notify_ready "$ready_message"
      wait "$child_pid"
      child_status=$?
      trap - TERM INT HUP EXIT
      return "$child_status"
    fi
    sleep 0.25
  done

  wait "$child_pid"
  child_status=$?
  trap - TERM INT HUP EXIT
  return "$child_status"
}

devenv_wait_for_http() {
  local url="$1"
  local label="${2:-$url}"
  local attempts="${3:-240}"
  local sleep_seconds="${4:-0.25}"

  local attempt
  for attempt in $(seq 1 "$attempts"); do
    if curl --silent --show-error --fail "$url" >/dev/null 2>&1; then
      return 0
    fi
    sleep "$sleep_seconds"
  done

  echo "timed out waiting for $label at $url" >&2
  return 1
}

devenv_run_with_http_ready() {
  local url="$1"
  local ready_message="$2"
  shift 2

  "$@" &
  local child_pid=$!
  local child_status=0

  forward_terminate() {
    kill -TERM "$child_pid" >/dev/null 2>&1 || true
  }
  trap forward_terminate TERM INT HUP EXIT

  while kill -0 "$child_pid" >/dev/null 2>&1; do
    if curl --silent --show-error --fail "$url" >/dev/null 2>&1; then
      devenv_notify_ready "$ready_message"
      wait "$child_pid"
      child_status=$?
      trap - TERM INT HUP EXIT
      return "$child_status"
    fi
    sleep 0.25
  done

  wait "$child_pid"
  child_status=$?
  trap - TERM INT HUP EXIT
  return "$child_status"
}

devenv_run_forever() {
  "$@" &
  local child_pid=$!
  local child_status=0

  forward_terminate() {
    kill -TERM "$child_pid" >/dev/null 2>&1 || true
  }
  trap forward_terminate TERM INT HUP EXIT

  wait "$child_pid"
  child_status=$?

  trap - TERM INT HUP EXIT
  return "$child_status"
}
