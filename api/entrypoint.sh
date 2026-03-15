#!/usr/bin/env bash
set -euo pipefail

log() {
  printf '[api-entrypoint] %s\n' "$*" >&2
}

require_env() {
  local name="$1"
  if [ -z "${!name:-}" ]; then
    log "missing required environment variable: $name"
    exit 1
  fi
}

random_hex() {
  od -vAn -N16 -tx1 /dev/urandom | tr -d ' \n'
}

stop_pid() {
  local pid="${1:-}"
  if [ -n "$pid" ] && kill -0 "$pid" 2>/dev/null; then
    kill "$pid" 2>/dev/null || true
    wait "$pid" 2>/dev/null || true
  fi
}

ensure_runtime_dirs() {
  # dockerTools images do not guarantee a precreated /tmp. Dolt / Go temp-file
  # paths assume it exists, so create it explicitly on boot.
  mkdir -p /tmp /var/tmp "$DOLT_DATA_ROOT"
  chmod 1777 /tmp /var/tmp
  export TMPDIR=/tmp
}

wait_for_sql() {
  local host="$1"
  local port="$2"
  local timeout_secs="$3"
  local start_ts
  start_ts="$(date +%s)"

  while true; do
    if dolt --host "$host" --port "$port" --no-tls sql -q "select 1" >/dev/null 2>&1; then
      return 0
    fi

    if [ $(( "$(date +%s)" - start_ts )) -ge "$timeout_secs" ]; then
      log "timed out waiting for Dolt SQL server on ${host}:${port}"
      return 1
    fi

    sleep 1
  done
}

write_sql_server_config() {
  local path="$1"
  local host="$2"
  local read_only="$3"

  cat >"$path" <<EOF
log_level: info
behavior:
  read_only: ${read_only}
listener:
  host: ${host}
  port: ${DOLT_SQL_PORT}
data_dir: ${DOLT_DATA_ROOT}
cfg_dir: ${DOLT_CFG_DIR}
privilege_file: ${DOLT_PRIVILEGE_FILE}
branch_control_file: ${DOLT_BRANCH_CONTROL_FILE}
EOF
}

clone_remote_repo() {
  local clone_cmd=(
    dolt clone
    --branch "$DOLT_REMOTE_BRANCH"
    --single-branch
    --depth "$DOLT_CLONE_DEPTH"
    "$DOLT_REMOTE_URL"
    "$DOLT_DATABASE_NAME"
  )

  log "cloning ${DOLT_REMOTE_URL} branch ${DOLT_REMOTE_BRANCH} into ${DOLT_REPO_DIR}"
  (
    cd "$DOLT_DATA_ROOT"
    "${clone_cmd[@]}"
  )
  log "clone complete"
}

sync_existing_repo() {
  log "using existing local Dolt clone at ${DOLT_REPO_DIR}"

  if [ "${DOLT_PULL_ON_BOOT}" != "true" ]; then
    log "boot-time Dolt sync disabled; using local clone as-is"
    return 0
  fi

  (
    cd "$DOLT_REPO_DIR"

    log "fetching origin/${DOLT_REMOTE_BRANCH}"
    if ! dolt fetch origin "$DOLT_REMOTE_BRANCH"; then
      log "fetch failed; continuing with existing local clone"
      exit 0
    fi

    if ! dolt checkout "$DOLT_REMOTE_BRANCH" >/dev/null 2>&1; then
      if ! dolt checkout -b "$DOLT_REMOTE_BRANCH" "origin/${DOLT_REMOTE_BRANCH}" >/dev/null 2>&1; then
        log "could not switch to branch ${DOLT_REMOTE_BRANCH}; continuing with current local branch"
        exit 0
      fi
    fi

    log "pulling origin/${DOLT_REMOTE_BRANCH}"
    if ! dolt pull origin "$DOLT_REMOTE_BRANCH"; then
      log "pull failed; continuing with existing local clone"
      exit 0
    fi

    log "pull complete"
  )
}

ensure_local_repo() {
  mkdir -p "$DOLT_DATA_ROOT"

  if [ -d "${DOLT_REPO_DIR}/.dolt" ]; then
    sync_existing_repo
    return 0
  fi

  rm -rf "$DOLT_REPO_DIR"
  clone_remote_repo
}

bootstrap_sql_user() {
  local bootstrap_config="$1"
  local bootstrap_host="localhost"
  local bootstrap_pid=""

  write_sql_server_config "$bootstrap_config" "$bootstrap_host" "false"

  log "starting bootstrap Dolt SQL server"
  dolt sql-server --config "$bootstrap_config" &
  bootstrap_pid="$!"

  wait_for_sql "$bootstrap_host" "$DOLT_SQL_PORT" "$DOLT_SQL_START_TIMEOUT_SECS"

  log "creating local read-only SQL user ${DOLT_API_SQL_USER}"
  dolt --host "$bootstrap_host" --port "$DOLT_SQL_PORT" --no-tls sql -q "
CREATE USER IF NOT EXISTS '${DOLT_API_SQL_USER}'@'127.0.0.1' IDENTIFIED BY '${DOLT_API_SQL_PASSWORD}';
ALTER USER '${DOLT_API_SQL_USER}'@'127.0.0.1' IDENTIFIED BY '${DOLT_API_SQL_PASSWORD}';
GRANT SELECT ON *.* TO '${DOLT_API_SQL_USER}'@'127.0.0.1';
"

  stop_pid "$bootstrap_pid"
}

start_runtime_sql_server() {
  local runtime_config="$1"

  write_sql_server_config "$runtime_config" "$DOLT_SQL_HOST" "$DOLT_SQL_READ_ONLY"

  log "starting Dolt SQL server on ${DOLT_SQL_HOST}:${DOLT_SQL_PORT}"
  dolt sql-server --config "$runtime_config" &
  DOLT_PID="$!"

  wait_for_sql "$DOLT_SQL_HOST" "$DOLT_SQL_PORT" "$DOLT_SQL_START_TIMEOUT_SECS"
}

start_api() {
  export FISHYSTUFF_DATABASE_URL="mysql://${DOLT_API_SQL_USER}:${DOLT_API_SQL_PASSWORD}@${DOLT_SQL_HOST}:${DOLT_SQL_PORT}/${DOLT_DATABASE_NAME}"

  log "starting fishystuff_server on ${FISHYSTUFF_BIND}"
  fishystuff_server \
    --config "$API_CONFIG_PATH" \
    --bind "$FISHYSTUFF_BIND" &
  API_PID="$!"
}

cleanup() {
  stop_pid "${API_PID:-}"
  stop_pid "${DOLT_PID:-}"
}

require_env DOLT_REMOTE_URL

API_CONFIG_PATH="${API_CONFIG_PATH:-/app/api/config.toml}"
FISHYSTUFF_BIND="${FISHYSTUFF_BIND:-0.0.0.0:8080}"

DOLT_DATA_ROOT="${DOLT_DATA_ROOT:-/data}"
DOLT_DATABASE_NAME="${DOLT_DATABASE_NAME:-fishystuff}"
DOLT_REPO_DIR="${DOLT_DATA_ROOT}/${DOLT_DATABASE_NAME}"
DOLT_REMOTE_BRANCH="${DOLT_REMOTE_BRANCH:-main}"
DOLT_CLONE_DEPTH="${DOLT_CLONE_DEPTH:-1}"
DOLT_PULL_ON_BOOT="${DOLT_PULL_ON_BOOT:-true}"

DOLT_SQL_HOST="${DOLT_SQL_HOST:-127.0.0.1}"
DOLT_SQL_PORT="${DOLT_SQL_PORT:-3306}"
DOLT_SQL_READ_ONLY="${DOLT_SQL_READ_ONLY:-true}"
DOLT_SQL_START_TIMEOUT_SECS="${DOLT_SQL_START_TIMEOUT_SECS:-60}"

DOLT_API_SQL_USER="${DOLT_API_SQL_USER:-fishystuff_api}"
DOLT_API_SQL_PASSWORD="${DOLT_API_SQL_PASSWORD:-$(random_hex)}"

DOLT_CFG_DIR="${DOLT_DATA_ROOT}/.doltcfg"
DOLT_PRIVILEGE_FILE="${DOLT_CFG_DIR}/privileges.db"
DOLT_BRANCH_CONTROL_FILE="${DOLT_CFG_DIR}/branch_control.db"
BOOTSTRAP_SQL_CONFIG="${DOLT_DATA_ROOT}/sql-bootstrap.yaml"
RUNTIME_SQL_CONFIG="${DOLT_DATA_ROOT}/sql-server.yaml"

API_PID=""
DOLT_PID=""

trap cleanup EXIT INT TERM

ensure_runtime_dirs
ensure_local_repo
mkdir -p "$DOLT_CFG_DIR"
bootstrap_sql_user "$BOOTSTRAP_SQL_CONFIG"
start_runtime_sql_server "$RUNTIME_SQL_CONFIG"
start_api

set +e
wait -n "$API_PID" "$DOLT_PID"
status="$?"
set -e

cleanup
exit "$status"
