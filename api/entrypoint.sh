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
  local repo_name="${1:-$DOLT_DATABASE_NAME}"
  local repo_dir="${DOLT_DATA_ROOT}/${repo_name}"
  local clone_cmd=(
    dolt clone
    --branch "$DOLT_REMOTE_BRANCH"
    --single-branch
  )

  if [ -n "${DOLT_CLONE_DEPTH:-}" ]; then
    clone_cmd+=(--depth "$DOLT_CLONE_DEPTH")
  fi

  clone_cmd+=(
    "$DOLT_REMOTE_URL"
    "$repo_name"
  )

  log "cloning ${DOLT_REMOTE_URL} branch ${DOLT_REMOTE_BRANCH} into ${repo_dir}"
  (
    cd "$DOLT_DATA_ROOT"
    "${clone_cmd[@]}"
  )
  log "clone complete"
}

ensure_repo_identity() {
  local repo_dir="${1:-$DOLT_REPO_DIR}"
  (
    cd "$repo_dir"

    if ! dolt config --local --get user.name >/dev/null 2>&1; then
      dolt config --local --add user.name "$DOLT_REPO_USER_NAME"
    fi
    if ! dolt config --local --get user.email >/dev/null 2>&1; then
      dolt config --local --add user.email "$DOLT_REPO_USER_EMAIL"
    fi
  )
}

replace_repo_with_fresh_clone() {
  local reason="$1"
  local temp_name=""
  local temp_dir=""
  local backup_dir=""

  temp_name="${DOLT_DATABASE_NAME}.reclone.$(random_hex)"
  temp_dir="${DOLT_DATA_ROOT}/${temp_name}"
  backup_dir="${DOLT_REPO_DIR}.bak.$(date +%s)"

  log "attempting fresh clone because ${reason}"
  cd "$DOLT_DATA_ROOT"
  rm -rf "$temp_dir"
  if ! clone_remote_repo "$temp_name"; then
    rm -rf "$temp_dir"
    return 1
  fi

  ensure_repo_identity "$temp_dir"
  mv "$DOLT_REPO_DIR" "$backup_dir"
  mv "$temp_dir" "$DOLT_REPO_DIR"
  rm -rf "$backup_dir"
  log "fresh clone replaced existing local repo"
}

sync_existing_repo() {
  log "using existing local Dolt clone at ${DOLT_REPO_DIR}"

  if [ "${DOLT_PULL_ON_BOOT}" != "true" ]; then
    log "boot-time Dolt sync disabled; using local clone as-is"
    return 0
  fi

  if replace_repo_with_fresh_clone "boot-time sync requested"; then
    return 0
  fi

  log "fresh clone sync failed; continuing with existing local clone"
}

ensure_local_repo() {
  mkdir -p "$DOLT_DATA_ROOT"

  if [ -d "${DOLT_REPO_DIR}/.dolt" ]; then
    sync_existing_repo
    return 0
  fi

  rm -rf "$DOLT_REPO_DIR"
  clone_remote_repo
  ensure_repo_identity
}

reset_sql_access_state() {
  if [ -f "$DOLT_PRIVILEGE_FILE" ] || [ -f "$DOLT_BRANCH_CONTROL_FILE" ]; then
    log "resetting Dolt SQL auth state under ${DOLT_CFG_DIR}"
  fi

  rm -f "$DOLT_PRIVILEGE_FILE" "$DOLT_BRANCH_CONTROL_FILE"
}

bootstrap_sql_user() {
  local bootstrap_config="$1"
  local bootstrap_host="$DOLT_SQL_HOST"
  local bootstrap_pid=""

  # The Dolt repo clone is persistent on the Fly volume, but the SQL auth files
  # are runtime-local state. Recreate them on boot so a stale privileges.db from
  # an older machine cannot block the local API user.
  reset_sql_access_state
  write_sql_server_config "$bootstrap_config" "$bootstrap_host" "false"

  log "starting bootstrap Dolt SQL server"
  dolt sql-server --config "$bootstrap_config" &
  bootstrap_pid="$!"

  wait_for_sql "$bootstrap_host" "$DOLT_SQL_PORT" "$DOLT_SQL_START_TIMEOUT_SECS"

  log "creating local read-only SQL user ${DOLT_API_SQL_USER}"
  dolt --host "$bootstrap_host" --port "$DOLT_SQL_PORT" --no-tls sql -q "
CREATE USER '${DOLT_API_SQL_USER}'@'127.0.0.1' IDENTIFIED BY '${DOLT_API_SQL_PASSWORD}';
GRANT ALL ON *.* TO '${DOLT_API_SQL_USER}'@'127.0.0.1';
CREATE USER '${DOLT_API_SQL_USER}'@'localhost' IDENTIFIED BY '${DOLT_API_SQL_PASSWORD}';
GRANT ALL ON *.* TO '${DOLT_API_SQL_USER}'@'localhost';
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
  local api_cmd=(
    fishystuff_server
    --config "$API_CONFIG_PATH"
    --bind "$FISHYSTUFF_BIND"
  )
  if [ -n "${FISHYSTUFF_REQUEST_TIMEOUT_SECS:-}" ]; then
    api_cmd+=(--request-timeout-secs "$FISHYSTUFF_REQUEST_TIMEOUT_SECS")
  fi
  "${api_cmd[@]}" &
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
DOLT_CLONE_DEPTH="${DOLT_CLONE_DEPTH:-}"
DOLT_PULL_ON_BOOT="${DOLT_PULL_ON_BOOT:-true}"
DOLT_REPO_USER_NAME="${DOLT_REPO_USER_NAME:-fishystuff api}"
DOLT_REPO_USER_EMAIL="${DOLT_REPO_USER_EMAIL:-api@fishystuff.fish}"

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
