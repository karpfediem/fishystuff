#!/usr/bin/env bash
set -euo pipefail

SCRIPT_PATH="$(readlink -f "${BASH_SOURCE[0]}")"
SCRIPT_DIR="$(cd "$(dirname "$SCRIPT_PATH")" && pwd)"
source "${SCRIPT_DIR}/lib/common.sh"

api_bundle="$(normalize_named_arg api_bundle "${1-auto}")"
dolt_bundle="$(normalize_named_arg dolt_bundle "${2-auto}")"
api_env_file="$(normalize_named_arg api_env_file "${3-/var/lib/fishystuff/gitops-beta/api/runtime.env}")"
dolt_env_file="$(normalize_named_arg dolt_env_file "${4-/var/lib/fishystuff/gitops-beta/dolt/beta.env}")"

cd "$RECIPE_REPO_ROOT"

require_command() {
  local command_name="$1"
  if ! command -v "$command_name" >/dev/null 2>&1; then
    echo "missing required command: ${command_name}" >&2
    exit 127
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

require_runtime_env_matches_bundle() {
  local service="$1"
  local env_file="$2"
  local target="$3"
  local fixture_override="${FISHYSTUFF_GITOPS_BETA_SERVICE_START_PLAN_ALLOW_ENV_FILE_FIXTURE:-}"

  if [[ "$env_file" == "$target" ]]; then
    return
  fi
  if [[ "$fixture_override" == "1" && "$env_file" == /tmp/* ]]; then
    return
  fi

  echo "beta ${service} runtime env file does not match beta service bundle target" >&2
  echo "env file: ${env_file}" >&2
  echo "target:   ${target}" >&2
  exit 2
}

run_check() {
  local label="$1"
  local output="$2"
  shift 2

  if ! "$@" >"$output"; then
    echo "beta service start plan ${label} check failed" >&2
    cat "$output" >&2 || true
    exit 2
  fi
}

require_command awk
require_command mktemp

tmp_dir="$(mktemp -d)"
cleanup() {
  rm -rf "$tmp_dir"
}
trap cleanup EXIT

api_env_output="${tmp_dir}/api-runtime-env.out"
dolt_env_output="${tmp_dir}/dolt-runtime-env.out"
api_bundle_output="${tmp_dir}/api-bundle.out"
dolt_bundle_output="${tmp_dir}/dolt-bundle.out"

run_check api-runtime-env "$api_env_output" \
  bash scripts/recipes/gitops-check-beta-runtime-env.sh api "$api_env_file"
run_check dolt-runtime-env "$dolt_env_output" \
  bash scripts/recipes/gitops-check-beta-runtime-env.sh dolt "$dolt_env_file"
run_check api-service-bundle "$api_bundle_output" \
  bash scripts/recipes/gitops-check-beta-service-bundle.sh api "$api_bundle"
run_check dolt-service-bundle "$dolt_bundle_output" \
  bash scripts/recipes/gitops-check-beta-service-bundle.sh dolt "$dolt_bundle"

require_kv_equals remote_deploy_performed "$api_env_output" false
require_kv_equals infrastructure_mutation_performed "$api_env_output" false
require_kv_equals local_host_mutation_performed "$api_env_output" false
require_kv_equals remote_deploy_performed "$dolt_env_output" false
require_kv_equals infrastructure_mutation_performed "$dolt_env_output" false
require_kv_equals local_host_mutation_performed "$dolt_env_output" false
require_kv_equals remote_deploy_performed "$api_bundle_output" false
require_kv_equals infrastructure_mutation_performed "$api_bundle_output" false
require_kv_equals remote_deploy_performed "$dolt_bundle_output" false
require_kv_equals infrastructure_mutation_performed "$dolt_bundle_output" false

api_bundle_path="$(require_kv_value gitops_beta_service_bundle_ok "$api_bundle_output" "API bundle check did not report a bundle path")"
dolt_bundle_path="$(require_kv_value gitops_beta_service_bundle_ok "$dolt_bundle_output" "Dolt bundle check did not report a bundle path")"
api_unit_name="$(require_kv_value gitops_beta_service_bundle_unit_name "$api_bundle_output" "API bundle check did not report a unit name")"
dolt_unit_name="$(require_kv_value gitops_beta_service_bundle_unit_name "$dolt_bundle_output" "Dolt bundle check did not report a unit name")"
api_unit_sha256="$(require_kv_value gitops_beta_service_bundle_systemd_unit_sha256 "$api_bundle_output" "API bundle check did not report a unit hash")"
dolt_unit_sha256="$(require_kv_value gitops_beta_service_bundle_systemd_unit_sha256 "$dolt_bundle_output" "Dolt bundle check did not report a unit hash")"
api_unit_source="$(require_kv_value gitops_beta_service_bundle_systemd_unit "$api_bundle_output" "API bundle check did not report a unit source")"
dolt_unit_source="$(require_kv_value gitops_beta_service_bundle_systemd_unit "$dolt_bundle_output" "Dolt bundle check did not report a unit source")"
api_runtime_env_target="$(require_kv_value gitops_beta_service_bundle_runtime_env_target "$api_bundle_output" "API bundle check did not report a runtime env target")"
dolt_runtime_env_target="$(require_kv_value gitops_beta_service_bundle_runtime_env_target "$dolt_bundle_output" "Dolt bundle check did not report a runtime env target")"
api_release_env_target="$(require_kv_value gitops_beta_service_bundle_release_env_target "$api_bundle_output" "API bundle check did not report a release env target")"
api_runtime_env_ok="$(require_kv_value gitops_beta_runtime_env_ok "$api_env_output" "API runtime env check did not report an env path")"
dolt_runtime_env_ok="$(require_kv_value gitops_beta_runtime_env_ok "$dolt_env_output" "Dolt runtime env check did not report an env path")"

if [[ "$api_unit_name" != "fishystuff-beta-api.service" ]]; then
  echo "API bundle reported a non-beta unit: ${api_unit_name}" >&2
  exit 2
fi
if [[ "$dolt_unit_name" != "fishystuff-beta-dolt.service" ]]; then
  echo "Dolt bundle reported a non-beta unit: ${dolt_unit_name}" >&2
  exit 2
fi

require_runtime_env_matches_bundle api "$api_runtime_env_ok" "$api_runtime_env_target"
require_runtime_env_matches_bundle dolt "$dolt_runtime_env_ok" "$dolt_runtime_env_target"

printf 'gitops_beta_service_start_plan_ok=true\n'
printf 'gitops_beta_service_start_plan_api_runtime_env=%s\n' "$api_runtime_env_ok"
printf 'gitops_beta_service_start_plan_dolt_runtime_env=%s\n' "$dolt_runtime_env_ok"
printf 'gitops_beta_service_start_plan_api_bundle=%s\n' "$api_bundle_path"
printf 'gitops_beta_service_start_plan_dolt_bundle=%s\n' "$dolt_bundle_path"
printf 'gitops_beta_service_start_plan_api_unit=%s\n' "$api_unit_name"
printf 'gitops_beta_service_start_plan_dolt_unit=%s\n' "$dolt_unit_name"
printf 'gitops_beta_service_start_plan_api_unit_source=%s\n' "$api_unit_source"
printf 'gitops_beta_service_start_plan_dolt_unit_source=%s\n' "$dolt_unit_source"
printf 'gitops_beta_service_start_plan_api_unit_sha256=%s\n' "$api_unit_sha256"
printf 'gitops_beta_service_start_plan_dolt_unit_sha256=%s\n' "$dolt_unit_sha256"
printf 'gitops_beta_service_start_plan_api_runtime_env_target=%s\n' "$api_runtime_env_target"
printf 'gitops_beta_service_start_plan_api_release_env_target=%s\n' "$api_release_env_target"
printf 'gitops_beta_service_start_plan_dolt_runtime_env_target=%s\n' "$dolt_runtime_env_target"
printf 'read_only_readiness_check_01=just gitops-beta-check-runtime-env service=api env_file=%s\n' "$api_env_file"
printf 'read_only_readiness_check_02=just gitops-beta-check-runtime-env service=dolt env_file=%s\n' "$dolt_env_file"
printf 'read_only_readiness_check_03=just gitops-beta-check-service-bundle service=api bundle=%s\n' "$api_bundle_path"
printf 'read_only_readiness_check_04=just gitops-beta-check-service-bundle service=dolt bundle=%s\n' "$dolt_bundle_path"
printf 'read_only_readiness_check_05=verify API runtime env target %s is operator-owned and API release env target %s is GitOps-owned\n' "$api_runtime_env_target" "$api_release_env_target"
printf 'refusal_condition_01=do not run on a host that is not the intended beta host\n'
printf 'refusal_condition_02=do not proceed unless the checked API runtime env points at the beta loopback Dolt SQL port\n'
printf 'refusal_condition_03=do not proceed unless the checked API runtime env contains only beta public site/CDN origins\n'
printf 'refusal_condition_04=do not proceed unless the reviewed unit hashes below match current local checks\n'
printf 'refusal_condition_05=do not proceed unless the beta Dolt unit is started before the beta API unit\n'
printf 'guarded_host_action_01=FISHYSTUFF_GITOPS_ENABLE_BETA_DOLT_INSTALL=1 FISHYSTUFF_GITOPS_ENABLE_BETA_DOLT_RESTART=1 FISHYSTUFF_GITOPS_BETA_DOLT_UNIT_SHA256=%s just gitops-beta-install-service service=dolt bundle=%s\n' "$dolt_unit_sha256" "$dolt_bundle_path"
printf 'guarded_host_action_02=systemctl is-active --quiet %s\n' "$dolt_unit_name"
printf 'guarded_host_action_03=FISHYSTUFF_GITOPS_ENABLE_BETA_API_INSTALL=1 FISHYSTUFF_GITOPS_ENABLE_BETA_API_RESTART=1 FISHYSTUFF_GITOPS_BETA_API_UNIT_SHA256=%s just gitops-beta-install-service service=api bundle=%s\n' "$api_unit_sha256" "$api_bundle_path"
printf 'guarded_host_action_04=systemctl is-active --quiet %s\n' "$api_unit_name"
printf 'post_start_read_only_check_01=curl -fsS http://127.0.0.1:18192/api/v1/meta\n'
printf 'post_start_read_only_check_02=verify beta API meta reports the GitOps release identity after a beta local apply updates %s\n' "$api_release_env_target"
printf 'remote_deploy_performed=false\n'
printf 'infrastructure_mutation_performed=false\n'
printf 'local_host_mutation_performed=false\n'
