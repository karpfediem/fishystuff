#!/usr/bin/env bash
set -euo pipefail

SCRIPT_PATH="$(readlink -f "${BASH_SOURCE[0]}")"
SCRIPT_DIR="$(cd "$(dirname "$SCRIPT_PATH")" && pwd)"
source "${SCRIPT_DIR}/lib/common.sh"

target="$(normalize_named_arg target "${1:-${FISHYSTUFF_BETA_RESIDENT_TARGET:-}}")"
expected_hostname="$(normalize_named_arg expected_hostname "${2:-site-nbg1-beta}")"
summary_file="$(normalize_named_arg summary_file "${3:-data/gitops/beta-current.handoff-summary.json}")"
ssh_bin="$(normalize_named_arg ssh_bin "${4:-${FISHYSTUFF_GITOPS_SSH_BIN:-ssh}}")"

cd "$RECIPE_REPO_ROOT"

fail() {
  echo "$1" >&2
  exit 2
}

require_env_value() {
  local name="$1"
  local expected="$2"
  local value="${!name-}"

  if [[ "$value" != "$expected" ]]; then
    fail "gitops-beta-remote-start-services requires ${name}=${expected}"
  fi
}

require_env_nonempty() {
  local name="$1"
  local value="${!name-}"

  if [[ -z "$value" ]]; then
    fail "gitops-beta-remote-start-services requires ${name}"
  fi
}

require_command_or_executable() {
  local command_name="$1"
  local label="$2"

  if [[ "$command_name" == */* ]]; then
    if [[ ! -x "$command_name" ]]; then
      fail "${label} is not executable: ${command_name}"
    fi
    return
  fi
  if ! command -v "$command_name" >/dev/null 2>&1; then
    echo "missing required command: ${command_name}" >&2
    exit 127
  fi
}

require_beta_deploy_profile() {
  local active_profile="${FISHYSTUFF_OPERATOR_SECRETSPEC_PROFILE:-}"

  case "$active_profile" in
    beta-deploy)
      ;;
    production-deploy | prod-deploy | production)
      fail "gitops-beta-remote-start-services must not run with production SecretSpec profile active: ${active_profile}"
      ;;
    *)
      fail "gitops-beta-remote-start-services requires FISHYSTUFF_OPERATOR_SECRETSPEC_PROFILE=beta-deploy"
      ;;
  esac
}

require_safe_target() {
  local value="$1"
  local user=""
  local host=""

  if [[ -z "$value" ]]; then
    fail "target is required; use target=root@<fresh-beta-ip>"
  fi
  if [[ "$value" != *@* ]]; then
    fail "target must be user@IPv4, got: ${value}"
  fi
  user="${value%@*}"
  host="${value#*@}"
  if [[ "$user" != "root" ]]; then
    fail "fresh beta service start currently expects root SSH, got user: ${user}"
  fi
  if [[ ! "$host" =~ ^([0-9]{1,3}\.){3}[0-9]{1,3}$ ]]; then
    fail "target host must be an IPv4 address, got: ${host}"
  fi
  if [[ "$host" == "178.104.230.121" ]]; then
    fail "target points at the previous beta host; use the fresh replacement IP"
  fi
}

summary_value() {
  local query="$1"
  jq -er "$query" "$summary_file"
}

require_summary_equals() {
  local label="$1"
  local query="$2"
  local expected="$3"
  local value=""

  value="$(summary_value "$query")"
  if [[ "$value" != "$expected" ]]; then
    fail "handoff summary ${label} must be ${expected}, got: ${value}"
  fi
}

require_store_path() {
  local label="$1"
  local value="$2"
  local fixture_override="${FISHYSTUFF_GITOPS_BETA_REMOTE_SERVICE_START_ALLOW_BUNDLE_FIXTURE:-}"

  if [[ "$fixture_override" == "1" && "$value" == /tmp/* ]]; then
    if [[ ! -e "$value" ]]; then
      fail "${label} fixture path does not exist locally: ${value}"
    fi
    return
  fi

  if [[ "$value" != /nix/store/* ]]; then
    fail "${label} must be a /nix/store path, got: ${value}"
  fi
  if [[ ! -e "$value" ]]; then
    fail "${label} does not exist locally: ${value}"
  fi
}

kv_value() {
  local key="$1"
  local file="$2"
  awk -F= -v key="$key" '$1 == key { print substr($0, length(key) + 2); exit }' "$file"
}

require_kv_value() {
  local key="$1"
  local file="$2"
  local message="$3"
  local value=""

  value="$(kv_value "$key" "$file")"
  require_value "$value" "$message"
  printf '%s' "$value"
}

run_bundle_check() {
  local service="$1"
  local bundle="$2"
  local output="$3"

  if ! bash scripts/recipes/gitops-check-beta-service-bundle.sh "$service" "$bundle" >"$output"; then
    echo "beta ${service} service bundle check failed" >&2
    cat "$output" >&2 || true
    exit 2
  fi
}

require_env_value FISHYSTUFF_GITOPS_ENABLE_BETA_REMOTE_SERVICE_START 1
require_env_value FISHYSTUFF_GITOPS_ENABLE_BETA_REMOTE_DOLT_INSTALL 1
require_env_value FISHYSTUFF_GITOPS_ENABLE_BETA_REMOTE_DOLT_RESTART 1
require_env_value FISHYSTUFF_GITOPS_ENABLE_BETA_REMOTE_API_INSTALL 1
require_env_value FISHYSTUFF_GITOPS_ENABLE_BETA_REMOTE_API_RESTART 1
require_env_value FISHYSTUFF_GITOPS_BETA_REMOTE_SERVICE_TARGET "$target"
require_env_nonempty FISHYSTUFF_GITOPS_BETA_DOLT_UNIT_SHA256
require_env_nonempty FISHYSTUFF_GITOPS_BETA_API_UNIT_SHA256
require_beta_deploy_profile
assert_deployment_configuration_safe beta
assert_beta_infra_cluster_dns_scope_safe
require_safe_target "$target"
require_command_or_executable jq jq
require_command_or_executable awk awk
require_command_or_executable "$ssh_bin" ssh_bin

if [[ ! -f "$summary_file" ]]; then
  fail "handoff summary does not exist: ${summary_file}"
fi

require_summary_equals schema '.schema' fishystuff.gitops.current-handoff.v1
require_summary_equals cluster '.cluster' beta
require_summary_equals environment '.environment.name' beta
require_summary_equals mode '.mode' validate
require_summary_equals closure_paths_verified '.checks.closure_paths_verified | tostring' true
require_summary_equals gitops_unify_passed '.checks.gitops_unify_passed | tostring' true
require_summary_equals summary_remote_deploy_performed '.checks.remote_deploy_performed | tostring' false
require_summary_equals summary_infrastructure_mutation_performed '.checks.infrastructure_mutation_performed | tostring' false

release_id="$(summary_value '.active_release.release_id')"
dolt_commit="$(summary_value '.active_release.dolt_commit')"
api_bundle="$(summary_value '.active_release.closures.api')"
dolt_bundle="$(summary_value '.active_release.closures.dolt_service')"

require_store_path api_bundle "$api_bundle"
require_store_path dolt_bundle "$dolt_bundle"

tmp_dir="$(mktemp -d)"
tmp_key="$(create_temp_ssh_key_from_env /tmp/fishystuff-beta-service-start-key.XXXXXX)"
known_hosts="$(mktemp /tmp/fishystuff-beta-service-start-known-hosts.XXXXXX)"
cleanup() {
  rm -rf "$tmp_dir"
  rm -f "$tmp_key" "$known_hosts"
}
trap cleanup EXIT

api_bundle_output="${tmp_dir}/api-bundle.out"
dolt_bundle_output="${tmp_dir}/dolt-bundle.out"
run_bundle_check api "$api_bundle" "$api_bundle_output"
run_bundle_check dolt "$dolt_bundle" "$dolt_bundle_output"

api_unit_name="$(require_kv_value gitops_beta_service_bundle_unit_name "$api_bundle_output" "API bundle check did not report a unit name")"
dolt_unit_name="$(require_kv_value gitops_beta_service_bundle_unit_name "$dolt_bundle_output" "Dolt bundle check did not report a unit name")"
api_unit_source="$(require_kv_value gitops_beta_service_bundle_systemd_unit "$api_bundle_output" "API bundle check did not report a unit source")"
dolt_unit_source="$(require_kv_value gitops_beta_service_bundle_systemd_unit "$dolt_bundle_output" "Dolt bundle check did not report a unit source")"
api_unit_sha256="$(require_kv_value gitops_beta_service_bundle_systemd_unit_sha256 "$api_bundle_output" "API bundle check did not report a unit hash")"
dolt_unit_sha256="$(require_kv_value gitops_beta_service_bundle_systemd_unit_sha256 "$dolt_bundle_output" "Dolt bundle check did not report a unit hash")"
api_unit_install_path="$(require_kv_value gitops_beta_service_bundle_unit_install_path "$api_bundle_output" "API bundle check did not report a unit install path")"
dolt_unit_install_path="$(require_kv_value gitops_beta_service_bundle_unit_install_path "$dolt_bundle_output" "Dolt bundle check did not report a unit install path")"
api_runtime_env_target="$(require_kv_value gitops_beta_service_bundle_runtime_env_target "$api_bundle_output" "API bundle check did not report a runtime env target")"
api_release_env_target="$(require_kv_value gitops_beta_service_bundle_release_env_target "$api_bundle_output" "API bundle check did not report a release env target")"
dolt_runtime_env_target="$(require_kv_value gitops_beta_service_bundle_runtime_env_target "$dolt_bundle_output" "Dolt bundle check did not report a runtime env target")"

if [[ "$api_unit_name" != "fishystuff-beta-api.service" ]]; then
  fail "API bundle reported a non-beta unit: ${api_unit_name}"
fi
if [[ "$dolt_unit_name" != "fishystuff-beta-dolt.service" ]]; then
  fail "Dolt bundle reported a non-beta unit: ${dolt_unit_name}"
fi
if [[ "$api_unit_sha256" != "$FISHYSTUFF_GITOPS_BETA_API_UNIT_SHA256" ]]; then
  fail "FISHYSTUFF_GITOPS_BETA_API_UNIT_SHA256 does not match checked beta API unit"
fi
if [[ "$dolt_unit_sha256" != "$FISHYSTUFF_GITOPS_BETA_DOLT_UNIT_SHA256" ]]; then
  fail "FISHYSTUFF_GITOPS_BETA_DOLT_UNIT_SHA256 does not match checked beta Dolt unit"
fi

printf 'gitops_beta_remote_start_services_checked=true\n'
printf 'deployment=beta\n'
printf 'resident_target=%s\n' "$target"
printf 'release_id=%s\n' "$release_id"
printf 'dolt_commit=%s\n' "$dolt_commit"
printf 'dolt_unit_source=%s\n' "$dolt_unit_source"
printf 'dolt_unit_sha256=%s\n' "$dolt_unit_sha256"
printf 'api_unit_source=%s\n' "$api_unit_source"
printf 'api_unit_sha256=%s\n' "$api_unit_sha256"

ssh_common=(
  -i "$tmp_key"
  -o IdentitiesOnly=yes
  -o StrictHostKeyChecking=accept-new
  -o UserKnownHostsFile="$known_hosts"
)

"$ssh_bin" "${ssh_common[@]}" "$target" bash -s -- \
  "$expected_hostname" \
  "$dolt_unit_source" \
  "$dolt_unit_sha256" \
  "$dolt_unit_install_path" \
  "$dolt_runtime_env_target" \
  "$api_unit_source" \
  "$api_unit_sha256" \
  "$api_unit_install_path" \
  "$api_runtime_env_target" \
  "$api_release_env_target" \
  "$release_id" \
  "$dolt_commit" <<'REMOTE'
set -euo pipefail

expected_hostname="$1"
dolt_unit_source="$2"
dolt_unit_sha256="$3"
dolt_unit_install_path="$4"
dolt_runtime_env_target="$5"
api_unit_source="$6"
api_unit_sha256="$7"
api_unit_install_path="$8"
api_runtime_env_target="$9"
api_release_env_target="${10}"
release_id="${11}"
dolt_commit="${12}"

fail() {
  echo "$1" >&2
  exit 2
}

require_file() {
  local label="$1"
  local path="$2"

  if [[ ! -f "$path" ]]; then
    fail "${label} missing: ${path}"
  fi
}

require_store_source() {
  local label="$1"
  local path="$2"

  case "$path" in
    /nix/store/*)
      ;;
    *)
      fail "${label} must be a /nix/store path, got: ${path}"
      ;;
  esac
  require_file "$label" "$path"
}

require_install_path() {
  local label="$1"
  local path="$2"
  local expected="$3"

  if [[ "$path" != "$expected" ]]; then
    fail "${label} install path must be ${expected}, got: ${path}"
  fi
}

require_sha256() {
  local label="$1"
  local path="$2"
  local expected="$3"
  local actual=""

  read -r actual _ < <(sha256sum "$path")
  if [[ "$actual" != "$expected" ]]; then
    fail "${label} sha256 mismatch: expected ${expected}, got ${actual}"
  fi
}

wait_tcp() {
  local label="$1"
  local host="$2"
  local port="$3"
  local unit="$4"
  local attempts="$5"
  local n=0

  while (( n < attempts )); do
    if bash -c "exec 3<>/dev/tcp/${host}/${port}" 2>/dev/null; then
      printf '%s_port_ready=%s:%s\n' "$label" "$host" "$port"
      return 0
    fi
    if ! systemctl is-active --quiet "$unit"; then
      systemctl status "$unit" --no-pager || true
      journalctl -u "$unit" --no-pager -n 120 || true
      fail "${label} unit stopped before ${host}:${port} became ready"
    fi
    n="$((n + 1))"
    sleep 1
  done
  journalctl -u "$unit" --no-pager -n 120 || true
  fail "${label} did not become ready on ${host}:${port}"
}

wait_api_meta() {
  local attempts="$1"
  local n=0
  local body=""

  if ! command -v curl >/dev/null 2>&1; then
    fail "curl is required for beta API meta readiness"
  fi

  while (( n < attempts )); do
    body="$(curl -fsS --max-time 2 http://127.0.0.1:18192/api/v1/meta 2>/dev/null || true)"
    if [[ -n "$body" && "$body" == *"$release_id"* && "$body" == *"$dolt_commit"* ]]; then
      printf 'api_meta_ready=true\n'
      printf 'api_meta_contains_release=true\n'
      printf 'api_meta_contains_dolt_commit=true\n'
      return 0
    fi
    if ! systemctl is-active --quiet fishystuff-beta-api.service; then
      systemctl status fishystuff-beta-api.service --no-pager || true
      journalctl -u fishystuff-beta-api.service --no-pager -n 120 || true
      fail "beta API unit stopped before /api/v1/meta became ready"
    fi
    n="$((n + 1))"
    sleep 1
  done
  journalctl -u fishystuff-beta-api.service --no-pager -n 120 || true
  fail "beta API /api/v1/meta did not report the expected release and Dolt commit"
}

if [[ "$(hostname)" != "$expected_hostname" ]]; then
  fail "remote hostname mismatch: expected ${expected_hostname}, got $(hostname)"
fi

require_store_source "Dolt unit source" "$dolt_unit_source"
require_store_source "API unit source" "$api_unit_source"
require_install_path "Dolt unit" "$dolt_unit_install_path" /etc/systemd/system/fishystuff-beta-dolt.service
require_install_path "API unit" "$api_unit_install_path" /etc/systemd/system/fishystuff-beta-api.service
require_sha256 "Dolt unit source" "$dolt_unit_source" "$dolt_unit_sha256"
require_sha256 "API unit source" "$api_unit_source" "$api_unit_sha256"
require_file "Dolt runtime env" "$dolt_runtime_env_target"
require_file "API runtime env" "$api_runtime_env_target"
require_file "API release env" "$api_release_env_target"

install -D -m 0644 "$dolt_unit_source" "$dolt_unit_install_path"
install -D -m 0644 "$api_unit_source" "$api_unit_install_path"
systemctl daemon-reload
systemctl restart fishystuff-beta-dolt.service
systemctl is-active --quiet fishystuff-beta-dolt.service
wait_tcp dolt_sql 127.0.0.1 3316 fishystuff-beta-dolt.service 900
systemctl restart fishystuff-beta-api.service
systemctl is-active --quiet fishystuff-beta-api.service
wait_tcp api_http 127.0.0.1 18192 fishystuff-beta-api.service 180
wait_api_meta 180

printf 'remote_hostname=%s\n' "$(hostname)"
printf 'remote_dolt_service_install_ok=fishystuff-beta-dolt.service\n'
printf 'remote_api_service_install_ok=fishystuff-beta-api.service\n'
printf 'remote_dolt_service_restart_ok=fishystuff-beta-dolt.service\n'
printf 'remote_api_service_restart_ok=fishystuff-beta-api.service\n'
printf 'remote_host_mutation_performed=true\n'
printf 'remote_deploy_performed=false\n'
printf 'infrastructure_mutation_performed=false\n'
printf 'local_host_mutation_performed=false\n'
REMOTE

printf 'gitops_beta_remote_start_services_ok=true\n'
printf 'remote_host_mutation_performed=true\n'
printf 'remote_deploy_performed=false\n'
printf 'infrastructure_mutation_performed=false\n'
printf 'local_host_mutation_performed=false\n'
