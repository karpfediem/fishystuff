#!/usr/bin/env bash
set -euo pipefail

SCRIPT_PATH="$(readlink -f "${BASH_SOURCE[0]}")"
SCRIPT_DIR="$(cd "$(dirname "$SCRIPT_PATH")" && pwd)"
source "${SCRIPT_DIR}/lib/common.sh"

api_env_file="$(normalize_named_arg api_env_file "${1-/var/lib/fishystuff/gitops-beta/api/runtime.env}")"
dolt_env_file="$(normalize_named_arg dolt_env_file "${2-/var/lib/fishystuff/gitops-beta/dolt/beta.env}")"
api_bundle="$(normalize_named_arg api_bundle "${3-auto}")"
dolt_bundle="$(normalize_named_arg dolt_bundle "${4-auto}")"
summary_file="$(normalize_named_arg summary_file "${5-data/gitops/beta-current.handoff-summary.json}")"

cd "$RECIPE_REPO_ROOT"

kv_value() {
  local key="$1"
  local file="$2"
  awk -F= -v key="$key" '$1 == key { print substr($0, length(key) + 2); exit }' "$file"
}

require_kv_equals() {
  local key="$1"
  local file="$2"
  local expected="$3"
  local value=""

  value="$(kv_value "$key" "$file")"
  if [[ "$value" != "$expected" ]]; then
    echo "${key} expected ${expected}, got: ${value}" >&2
    exit 2
  fi
}

run_env_check() {
  local service="$1"
  local env_file="$2"
  local output="$3"
  local stderr="$4"

  if bash scripts/recipes/gitops-check-beta-runtime-env.sh "$service" "$env_file" >"$output" 2>"$stderr"; then
    printf 'ready'
    return
  fi

  if grep -F "runtime env file does not exist" "$stderr" >/dev/null; then
    printf 'missing'
    return
  fi

  cat "$stderr" >&2 || true
  exit 2
}

api_secretspec_status() {
  if ! command -v secretspec >/dev/null 2>&1; then
    printf 'unavailable'
    return
  fi

  if secretspec check --profile beta-runtime >"${tmp_dir}/secretspec.out" 2>"${tmp_dir}/secretspec.err"; then
    printf 'ready'
    return
  fi

  if grep -E 'DBus error|secure storage|Operation not permitted|permission denied' "${tmp_dir}/secretspec.err" >/dev/null; then
    printf 'unavailable'
    return
  fi

  printf 'missing'
}

tmp_dir="$(mktemp -d)"
cleanup() {
  rm -rf "$tmp_dir"
}
trap cleanup EXIT

api_output="${tmp_dir}/api.out"
api_stderr="${tmp_dir}/api.err"
dolt_output="${tmp_dir}/dolt.out"
dolt_stderr="${tmp_dir}/dolt.err"
host_preflight_output="${tmp_dir}/host-preflight.out"
: >"$host_preflight_output"

api_status="$(run_env_check api "$api_env_file" "$api_output" "$api_stderr")"
dolt_status="$(run_env_check dolt "$dolt_env_file" "$dolt_output" "$dolt_stderr")"
api_secret_status="$(api_secretspec_status)"

if [[ "$api_status" == "ready" ]]; then
  require_kv_equals remote_deploy_performed "$api_output" false
  require_kv_equals infrastructure_mutation_performed "$api_output" false
  require_kv_equals local_host_mutation_performed "$api_output" false
fi
if [[ "$dolt_status" == "ready" ]]; then
  require_kv_equals remote_deploy_performed "$dolt_output" false
  require_kv_equals infrastructure_mutation_performed "$dolt_output" false
  require_kv_equals local_host_mutation_performed "$dolt_output" false
fi

packet_status="pending_runtime_env"
if [[ "$api_status" == "ready" && "$dolt_status" == "ready" ]]; then
  packet_status="ready"
fi

runtime_env_host_preflight_command="just gitops-beta-runtime-env-host-preflight api_env_file=${api_env_file} dolt_env_file=${dolt_env_file}"
service_start_packet_command="just gitops-beta-service-start-packet api_bundle=${api_bundle} dolt_bundle=${dolt_bundle} api_env_file=${api_env_file} dolt_env_file=${dolt_env_file} summary_file=${summary_file}"

if [[ "$packet_status" != "ready" ]]; then
  bash scripts/recipes/gitops-beta-runtime-env-host-preflight.sh \
    "$api_env_file" \
    "$dolt_env_file" >"$host_preflight_output"
  require_kv_equals remote_deploy_performed "$host_preflight_output" false
  require_kv_equals infrastructure_mutation_performed "$host_preflight_output" false
  require_kv_equals local_host_mutation_performed "$host_preflight_output" false
fi

printf 'gitops_beta_runtime_env_packet_ok=true\n'
printf 'runtime_env_packet_status=%s\n' "$packet_status"
printf 'runtime_env_packet_api_env_file=%s\n' "$api_env_file"
printf 'runtime_env_packet_dolt_env_file=%s\n' "$dolt_env_file"
printf 'runtime_env_packet_api_status=%s\n' "$api_status"
printf 'runtime_env_packet_dolt_status=%s\n' "$dolt_status"
printf 'runtime_env_packet_api_secretspec_status=%s\n' "$api_secret_status"
if [[ "$packet_status" != "ready" ]]; then
  printf 'runtime_env_packet_before_write_command=%s\n' "$runtime_env_host_preflight_command"
  printf 'runtime_env_packet_host_preflight_status=%s\n' "$(kv_value runtime_env_host_preflight_status "$host_preflight_output")"
  printf 'runtime_env_packet_host_preflight_next_required_action=%s\n' "$(kv_value runtime_env_host_preflight_next_required_action "$host_preflight_output")"
  printf 'runtime_env_packet_host_preflight_current_hostname=%s\n' "$(kv_value runtime_env_host_preflight_current_hostname "$host_preflight_output")"
  printf 'runtime_env_packet_host_preflight_expected_hostname=%s\n' "$(kv_value runtime_env_host_preflight_expected_hostname "$host_preflight_output")"
  printf 'runtime_env_packet_host_preflight_path_ready=%s\n' "$(kv_value runtime_env_host_preflight_path_ready "$host_preflight_output")"
  printf 'runtime_env_packet_host_preflight_ready=%s\n' "$(kv_value runtime_env_host_preflight_ready "$host_preflight_output")"
  if [[ -n "$(kv_value runtime_env_host_preflight_next_command_01 "$host_preflight_output")" ]]; then
    printf 'runtime_env_packet_host_preflight_next_command_01=%s\n' "$(kv_value runtime_env_host_preflight_next_command_01 "$host_preflight_output")"
  fi
fi
if [[ "$api_status" == "ready" ]]; then
  printf 'runtime_env_packet_api_database=%s\n' "$(kv_value gitops_beta_runtime_env_database "$api_output")"
  printf 'runtime_env_packet_api_public_site_base_url=%s\n' "$(kv_value gitops_beta_runtime_env_public_site_base_url "$api_output")"
  printf 'runtime_env_packet_api_public_cdn_base_url=%s\n' "$(kv_value gitops_beta_runtime_env_public_cdn_base_url "$api_output")"
fi
if [[ "$dolt_status" == "missing" ]]; then
  printf 'runtime_env_packet_next_command_01=FISHYSTUFF_GITOPS_ENABLE_BETA_DOLT_RUNTIME_ENV_WRITE=1 just gitops-beta-write-runtime-env service=dolt output=%s\n' "$dolt_env_file"
  if [[ "$api_status" == "missing" ]]; then
    printf 'runtime_env_packet_next_command_02=FISHYSTUFF_GITOPS_ENABLE_BETA_API_RUNTIME_ENV_WRITE=1 just gitops-beta-write-runtime-env-secretspec service=api output=%s profile=beta-runtime\n' "$api_env_file"
    if [[ "$api_secret_status" == "missing" ]]; then
      printf 'runtime_env_packet_missing_secret_01=FISHYSTUFF_GITOPS_BETA_API_DATABASE_URL\n'
    elif [[ "$api_secret_status" == "unavailable" ]]; then
      printf 'runtime_env_packet_secret_check_unavailable=true\n'
    fi
  fi
elif [[ "$api_status" == "missing" ]]; then
  printf 'runtime_env_packet_next_command_01=FISHYSTUFF_GITOPS_ENABLE_BETA_API_RUNTIME_ENV_WRITE=1 just gitops-beta-write-runtime-env-secretspec service=api output=%s profile=beta-runtime\n' "$api_env_file"
  if [[ "$api_secret_status" == "missing" ]]; then
    printf 'runtime_env_packet_missing_secret_01=FISHYSTUFF_GITOPS_BETA_API_DATABASE_URL\n'
  elif [[ "$api_secret_status" == "unavailable" ]]; then
    printf 'runtime_env_packet_secret_check_unavailable=true\n'
  fi
fi
if [[ "$packet_status" == "ready" ]]; then
  printf 'runtime_env_packet_next_command_01=%s\n' "$service_start_packet_command"
else
  printf 'runtime_env_packet_after_success_command=%s\n' "$service_start_packet_command"
fi
printf 'remote_deploy_performed=false\n'
printf 'infrastructure_mutation_performed=false\n'
printf 'local_host_mutation_performed=false\n'
