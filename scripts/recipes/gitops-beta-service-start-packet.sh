#!/usr/bin/env bash
set -euo pipefail

SCRIPT_PATH="$(readlink -f "${BASH_SOURCE[0]}")"
SCRIPT_DIR="$(cd "$(dirname "$SCRIPT_PATH")" && pwd)"
source "${SCRIPT_DIR}/lib/common.sh"

api_bundle="$(normalize_named_arg api_bundle "${1-auto}")"
dolt_bundle="$(normalize_named_arg dolt_bundle "${2-auto}")"
api_env_file="$(normalize_named_arg api_env_file "${3-/var/lib/fishystuff/gitops-beta/api/runtime.env}")"
dolt_env_file="$(normalize_named_arg dolt_env_file "${4-/var/lib/fishystuff/gitops-beta/dolt/beta.env}")"
summary_file="$(normalize_named_arg summary_file "${5-data/gitops/beta-current.handoff-summary.json}")"

cd "$RECIPE_REPO_ROOT"

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

tmp_dir="$(mktemp -d)"
cleanup() {
  rm -rf "$tmp_dir"
}
trap cleanup EXIT

runtime_packet_output="${tmp_dir}/runtime-env-packet.out"
if ! bash scripts/recipes/gitops-beta-runtime-env-packet.sh \
  "$api_env_file" \
  "$dolt_env_file" \
  "$api_bundle" \
  "$dolt_bundle" \
  "$summary_file" >"$runtime_packet_output" 2>"${tmp_dir}/runtime-env-packet.err"; then
  cat "${tmp_dir}/runtime-env-packet.err" >&2 || true
  exit 2
fi

runtime_packet_status="$(require_kv_value runtime_env_packet_status "$runtime_packet_output" "runtime env packet did not report status")"
if [[ "$runtime_packet_status" != "ready" ]]; then
  printf 'gitops_beta_service_start_packet_ok=true\n'
  printf 'service_start_packet_status=%s\n' "$runtime_packet_status"
  printf 'service_start_packet_api_env_file=%s\n' "$api_env_file"
  printf 'service_start_packet_dolt_env_file=%s\n' "$dolt_env_file"
  printf 'service_start_packet_api_status=%s\n' "$(kv_value runtime_env_packet_api_status "$runtime_packet_output")"
  printf 'service_start_packet_dolt_status=%s\n' "$(kv_value runtime_env_packet_dolt_status "$runtime_packet_output")"
  if [[ -n "$(kv_value runtime_env_packet_before_write_command "$runtime_packet_output")" ]]; then
    printf 'service_start_packet_before_write_command=%s\n' "$(kv_value runtime_env_packet_before_write_command "$runtime_packet_output")"
  fi
  if [[ -n "$(kv_value runtime_env_packet_host_preflight_next_required_action "$runtime_packet_output")" ]]; then
    printf 'service_start_packet_host_preflight_status=%s\n' "$(kv_value runtime_env_packet_host_preflight_status "$runtime_packet_output")"
    printf 'service_start_packet_host_preflight_next_required_action=%s\n' "$(kv_value runtime_env_packet_host_preflight_next_required_action "$runtime_packet_output")"
    printf 'service_start_packet_host_preflight_current_hostname=%s\n' "$(kv_value runtime_env_packet_host_preflight_current_hostname "$runtime_packet_output")"
    printf 'service_start_packet_host_preflight_expected_hostname=%s\n' "$(kv_value runtime_env_packet_host_preflight_expected_hostname "$runtime_packet_output")"
    printf 'service_start_packet_host_preflight_expected_hostname_match=%s\n' "$(kv_value runtime_env_packet_host_preflight_expected_hostname_match "$runtime_packet_output")"
    printf 'service_start_packet_host_preflight_resident_target=%s\n' "$(kv_value runtime_env_packet_host_preflight_resident_target "$runtime_packet_output")"
    printf 'service_start_packet_host_preflight_path_ready=%s\n' "$(kv_value runtime_env_packet_host_preflight_path_ready "$runtime_packet_output")"
    printf 'service_start_packet_host_preflight_ready=%s\n' "$(kv_value runtime_env_packet_host_preflight_ready "$runtime_packet_output")"
    printf 'service_start_packet_host_preflight_next_command_01=%s\n' "$(kv_value runtime_env_packet_host_preflight_next_command_01 "$runtime_packet_output")"
    if [[ -n "$(kv_value runtime_env_packet_host_preflight_next_note_01 "$runtime_packet_output")" ]]; then
      printf 'service_start_packet_host_preflight_next_note_01=%s\n' "$(kv_value runtime_env_packet_host_preflight_next_note_01 "$runtime_packet_output")"
    fi
  fi
  if [[ -n "$(kv_value runtime_env_packet_next_command_01 "$runtime_packet_output")" ]]; then
    printf 'service_start_packet_next_command_01=%s\n' "$(kv_value runtime_env_packet_next_command_01 "$runtime_packet_output")"
  fi
  if [[ -n "$(kv_value runtime_env_packet_next_command_02 "$runtime_packet_output")" ]]; then
    printf 'service_start_packet_next_command_02=%s\n' "$(kv_value runtime_env_packet_next_command_02 "$runtime_packet_output")"
  fi
  if [[ -n "$(kv_value runtime_env_packet_after_success_command "$runtime_packet_output")" ]]; then
    printf 'service_start_packet_after_success_command=%s\n' "$(kv_value runtime_env_packet_after_success_command "$runtime_packet_output")"
  fi
  printf 'remote_deploy_performed=false\n'
  printf 'infrastructure_mutation_performed=false\n'
  printf 'local_host_mutation_performed=false\n'
  exit 0
fi

plan_output="${tmp_dir}/service-start-plan.out"
if ! bash scripts/recipes/gitops-beta-service-start-plan.sh \
  "$api_bundle" \
  "$dolt_bundle" \
  "$api_env_file" \
  "$dolt_env_file" \
  "$summary_file" >"$plan_output" 2>&1; then
  cat "$plan_output" >&2 || true
  exit 2
fi

require_kv_equals gitops_beta_service_start_plan_ok "$plan_output" true
require_kv_equals remote_deploy_performed "$plan_output" false
require_kv_equals infrastructure_mutation_performed "$plan_output" false
require_kv_equals local_host_mutation_performed "$plan_output" false

bundle_source="$(require_kv_value gitops_beta_service_start_plan_bundle_source "$plan_output" "start plan did not report bundle source")"
resolved_api_bundle="$(require_kv_value gitops_beta_service_start_plan_api_bundle "$plan_output" "start plan did not report API bundle")"
resolved_dolt_bundle="$(require_kv_value gitops_beta_service_start_plan_dolt_bundle "$plan_output" "start plan did not report Dolt bundle")"
resolved_api_env="$(require_kv_value gitops_beta_service_start_plan_api_runtime_env "$plan_output" "start plan did not report API runtime env")"
resolved_dolt_env="$(require_kv_value gitops_beta_service_start_plan_dolt_runtime_env "$plan_output" "start plan did not report Dolt runtime env")"
api_unit="$(require_kv_value gitops_beta_service_start_plan_api_unit "$plan_output" "start plan did not report API unit")"
dolt_unit="$(require_kv_value gitops_beta_service_start_plan_dolt_unit "$plan_output" "start plan did not report Dolt unit")"
api_unit_sha256="$(require_kv_value gitops_beta_service_start_plan_api_unit_sha256 "$plan_output" "start plan did not report API unit hash")"
dolt_unit_sha256="$(require_kv_value gitops_beta_service_start_plan_dolt_unit_sha256 "$plan_output" "start plan did not report Dolt unit hash")"
api_runtime_env_target="$(require_kv_value gitops_beta_service_start_plan_api_runtime_env_target "$plan_output" "start plan did not report API runtime env target")"
dolt_runtime_env_target="$(require_kv_value gitops_beta_service_start_plan_dolt_runtime_env_target "$plan_output" "start plan did not report Dolt runtime env target")"
api_release_env_target="$(require_kv_value gitops_beta_service_start_plan_api_release_env_target "$plan_output" "start plan did not report API release env target")"
resolved_summary="$(kv_value gitops_beta_service_start_plan_handoff_summary "$plan_output")"
if [[ -z "$resolved_summary" ]]; then
  resolved_summary="$summary_file"
fi

start_command="FISHYSTUFF_GITOPS_ENABLE_BETA_SERVICE_START=1 FISHYSTUFF_GITOPS_ENABLE_BETA_DOLT_INSTALL=1 FISHYSTUFF_GITOPS_ENABLE_BETA_DOLT_RESTART=1 FISHYSTUFF_GITOPS_ENABLE_BETA_API_INSTALL=1 FISHYSTUFF_GITOPS_ENABLE_BETA_API_RESTART=1 FISHYSTUFF_GITOPS_BETA_DOLT_UNIT_SHA256=${dolt_unit_sha256} FISHYSTUFF_GITOPS_BETA_API_UNIT_SHA256=${api_unit_sha256} just gitops-beta-start-services api_bundle=${resolved_api_bundle} dolt_bundle=${resolved_dolt_bundle} api_env_file=${resolved_api_env} dolt_env_file=${resolved_dolt_env} summary_file=${resolved_summary}"
admission_packet_command="just gitops-beta-admission-packet summary_file=${resolved_summary} api_upstream=http://127.0.0.1:18192"

printf 'gitops_beta_service_start_packet_ok=true\n'
printf 'service_start_packet_status=ready\n'
printf 'service_start_packet_bundle_source=%s\n' "$bundle_source"
printf 'service_start_packet_handoff_summary=%s\n' "$resolved_summary"
printf 'service_start_packet_api_bundle=%s\n' "$resolved_api_bundle"
printf 'service_start_packet_dolt_bundle=%s\n' "$resolved_dolt_bundle"
printf 'service_start_packet_api_env_file=%s\n' "$resolved_api_env"
printf 'service_start_packet_dolt_env_file=%s\n' "$resolved_dolt_env"
printf 'service_start_packet_api_unit=%s\n' "$api_unit"
printf 'service_start_packet_dolt_unit=%s\n' "$dolt_unit"
printf 'service_start_packet_api_unit_sha256=%s\n' "$api_unit_sha256"
printf 'service_start_packet_dolt_unit_sha256=%s\n' "$dolt_unit_sha256"
printf 'service_start_packet_api_runtime_env_target=%s\n' "$api_runtime_env_target"
printf 'service_start_packet_dolt_runtime_env_target=%s\n' "$dolt_runtime_env_target"
printf 'service_start_packet_api_release_env_target=%s\n' "$api_release_env_target"
printf 'service_start_packet_order_01=dolt\n'
printf 'service_start_packet_order_02=api\n'
printf 'service_start_packet_note_01=run only on the intended beta host after reviewing the unit hashes\n'
printf 'service_start_packet_next_command_01=%s\n' "$start_command"
printf 'service_start_packet_after_success_command=%s\n' "$admission_packet_command"
printf 'remote_deploy_performed=false\n'
printf 'infrastructure_mutation_performed=false\n'
printf 'local_host_mutation_performed=false\n'
