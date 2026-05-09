#!/usr/bin/env bash
set -euo pipefail

SCRIPT_PATH="$(readlink -f "${BASH_SOURCE[0]}")"
SCRIPT_DIR="$(cd "$(dirname "$SCRIPT_PATH")" && pwd)"
source "${SCRIPT_DIR}/lib/common.sh"

api_bundle="$(normalize_named_arg api_bundle "${1-auto}")"
dolt_bundle="$(normalize_named_arg dolt_bundle "${2-auto}")"
api_env_file="$(normalize_named_arg api_env_file "${3-/var/lib/fishystuff/gitops-beta/api/runtime.env}")"
dolt_env_file="$(normalize_named_arg dolt_env_file "${4-/var/lib/fishystuff/gitops-beta/dolt/beta.env}")"
install_bin="$(normalize_named_arg install_bin "${5-${FISHYSTUFF_GITOPS_INSTALL_BIN:-install}}")"
systemctl_bin="$(normalize_named_arg systemctl_bin "${6-${FISHYSTUFF_GITOPS_SYSTEMCTL_BIN:-systemctl}}")"

cd "$RECIPE_REPO_ROOT"

require_command() {
  local command_name="$1"
  if ! command -v "$command_name" >/dev/null 2>&1; then
    echo "missing required command: ${command_name}" >&2
    exit 127
  fi
}

require_env_value() {
  local name="$1"
  local expected="$2"
  local value="${!name-}"

  if [[ "$value" != "$expected" ]]; then
    echo "gitops-beta-start-services requires ${name}=${expected}" >&2
    exit 2
  fi
}

require_env_nonempty() {
  local name="$1"
  local value="${!name-}"

  if [[ -z "$value" ]]; then
    echo "gitops-beta-start-services requires ${name}" >&2
    exit 2
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

require_kv_equals() {
  local key="$1"
  local file="$2"
  local expected="$3"
  local value=""

  value="$(require_kv_value "$key" "$file" "${key} was not reported")"
  if [[ "$value" != "$expected" ]]; then
    echo "${key} expected ${expected}, got: ${value}" >&2
    exit 2
  fi
}

run_install_gate() {
  local service="$1"
  local bundle="$2"
  local output="$3"

  if ! bash scripts/recipes/gitops-beta-install-service.sh "$service" "$bundle" "$install_bin" "$systemctl_bin" >"$output"; then
    echo "beta ${service} service install gate failed" >&2
    cat "$output" >&2 || true
    exit 2
  fi
}

require_env_value FISHYSTUFF_GITOPS_ENABLE_BETA_SERVICE_START 1
require_env_value FISHYSTUFF_GITOPS_ENABLE_BETA_DOLT_INSTALL 1
require_env_value FISHYSTUFF_GITOPS_ENABLE_BETA_DOLT_RESTART 1
require_env_value FISHYSTUFF_GITOPS_ENABLE_BETA_API_INSTALL 1
require_env_value FISHYSTUFF_GITOPS_ENABLE_BETA_API_RESTART 1
require_env_nonempty FISHYSTUFF_GITOPS_BETA_DOLT_UNIT_SHA256
require_env_nonempty FISHYSTUFF_GITOPS_BETA_API_UNIT_SHA256
require_command awk
require_command mktemp

tmp_dir="$(mktemp -d)"
cleanup() {
  rm -rf "$tmp_dir"
}
trap cleanup EXIT

plan_output="${tmp_dir}/service-start-plan.out"
dolt_install_output="${tmp_dir}/dolt-install.out"
api_install_output="${tmp_dir}/api-install.out"

if ! bash scripts/recipes/gitops-beta-service-start-plan.sh "$api_bundle" "$dolt_bundle" "$api_env_file" "$dolt_env_file" >"$plan_output"; then
  echo "beta service start plan check failed" >&2
  cat "$plan_output" >&2 || true
  exit 2
fi

require_kv_equals gitops_beta_service_start_plan_ok "$plan_output" true
require_kv_equals gitops_beta_service_start_plan_dolt_unit "$plan_output" fishystuff-beta-dolt.service
require_kv_equals gitops_beta_service_start_plan_api_unit "$plan_output" fishystuff-beta-api.service
require_kv_equals remote_deploy_performed "$plan_output" false
require_kv_equals infrastructure_mutation_performed "$plan_output" false
require_kv_equals local_host_mutation_performed "$plan_output" false

plan_dolt_bundle="$(require_kv_value gitops_beta_service_start_plan_dolt_bundle "$plan_output" "start plan did not report Dolt bundle")"
plan_api_bundle="$(require_kv_value gitops_beta_service_start_plan_api_bundle "$plan_output" "start plan did not report API bundle")"
plan_dolt_hash="$(require_kv_value gitops_beta_service_start_plan_dolt_unit_sha256 "$plan_output" "start plan did not report Dolt unit hash")"
plan_api_hash="$(require_kv_value gitops_beta_service_start_plan_api_unit_sha256 "$plan_output" "start plan did not report API unit hash")"

if [[ "$plan_dolt_hash" != "$FISHYSTUFF_GITOPS_BETA_DOLT_UNIT_SHA256" ]]; then
  echo "FISHYSTUFF_GITOPS_BETA_DOLT_UNIT_SHA256 does not match beta Dolt unit hash from start plan" >&2
  exit 2
fi
if [[ "$plan_api_hash" != "$FISHYSTUFF_GITOPS_BETA_API_UNIT_SHA256" ]]; then
  echo "FISHYSTUFF_GITOPS_BETA_API_UNIT_SHA256 does not match beta API unit hash from start plan" >&2
  exit 2
fi

run_install_gate dolt "$plan_dolt_bundle" "$dolt_install_output"
run_install_gate api "$plan_api_bundle" "$api_install_output"

printf 'gitops_beta_service_start_ok=true\n'
printf 'gitops_beta_service_start_dolt_bundle=%s\n' "$plan_dolt_bundle"
printf 'gitops_beta_service_start_api_bundle=%s\n' "$plan_api_bundle"
printf 'gitops_beta_service_start_dolt_unit_sha256=%s\n' "$plan_dolt_hash"
printf 'gitops_beta_service_start_api_unit_sha256=%s\n' "$plan_api_hash"
printf 'gitops_beta_service_start_step_01=dolt\n'
cat "$dolt_install_output"
printf 'gitops_beta_service_start_step_02=api\n'
cat "$api_install_output"
printf 'local_host_mutation_performed=true\n'
printf 'remote_deploy_performed=false\n'
printf 'infrastructure_mutation_performed=false\n'
