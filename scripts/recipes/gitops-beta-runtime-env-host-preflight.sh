#!/usr/bin/env bash
set -euo pipefail

SCRIPT_PATH="$(readlink -f "${BASH_SOURCE[0]}")"
SCRIPT_DIR="$(cd "$(dirname "$SCRIPT_PATH")" && pwd)"
source "${SCRIPT_DIR}/lib/common.sh"

api_env_file="$(normalize_named_arg api_env_file "${1-/var/lib/fishystuff/gitops-beta/api/runtime.env}")"
dolt_env_file="$(normalize_named_arg dolt_env_file "${2-/var/lib/fishystuff/gitops-beta/dolt/beta.env}")"

cd "$RECIPE_REPO_ROOT"

fail() {
  echo "$1" >&2
  exit 2
}

require_safe_runtime_env_path() {
  local service="$1"
  local path="$2"

  case "$service:$path" in
    api:/var/lib/fishystuff/gitops-beta/api/runtime.env | \
    dolt:/var/lib/fishystuff/gitops-beta/dolt/beta.env | \
    api:/tmp/* | \
    dolt:/tmp/*)
      ;;
    *)
      fail "refusing beta ${service} runtime env preflight outside the beta runtime path or /tmp: ${path}"
      ;;
  esac
}

bool_file_exists() {
  if [[ -f "$1" ]]; then
    printf 'true'
  else
    printf 'false'
  fi
}

bool_dir_exists() {
  if [[ -d "$1" ]]; then
    printf 'true'
  else
    printf 'false'
  fi
}

bool_path_writable() {
  if [[ -w "$1" ]]; then
    printf 'true'
  else
    printf 'false'
  fi
}

print_env_path_preflight() {
  local label="$1"
  local path="$2"
  local parent=""
  local parent_exists=""
  local parent_writable=""
  local file_exists=""
  local file_writable="false"

  parent="$(dirname "$path")"
  parent_exists="$(bool_dir_exists "$parent")"
  if [[ "$parent_exists" == "true" ]]; then
    parent_writable="$(bool_path_writable "$parent")"
  else
    parent_writable="false"
  fi

  file_exists="$(bool_file_exists "$path")"
  if [[ "$file_exists" == "true" ]]; then
    file_writable="$(bool_path_writable "$path")"
  fi

  printf 'runtime_env_host_preflight_%s_env_file=%s\n' "$label" "$path"
  printf 'runtime_env_host_preflight_%s_parent=%s\n' "$label" "$parent"
  printf 'runtime_env_host_preflight_%s_parent_exists=%s\n' "$label" "$parent_exists"
  printf 'runtime_env_host_preflight_%s_parent_writable=%s\n' "$label" "$parent_writable"
  printf 'runtime_env_host_preflight_%s_file_exists=%s\n' "$label" "$file_exists"
  printf 'runtime_env_host_preflight_%s_file_writable=%s\n' "$label" "$file_writable"
}

require_safe_runtime_env_path api "$api_env_file"
require_safe_runtime_env_path dolt "$dolt_env_file"
assert_deployment_configuration_safe beta

current_hostname="$(deployment_current_hostname)"
expected_hostname="$(deployment_resident_hostname beta)"
expected_match="$(deployment_hostname_match_status "$current_hostname" "$expected_hostname")"
api_parent="$(dirname "$api_env_file")"
dolt_parent="$(dirname "$dolt_env_file")"
path_ready="false"
ready="false"
if [[ -d "$api_parent" && -w "$api_parent" && -d "$dolt_parent" && -w "$dolt_parent" ]]; then
  path_ready="true"
fi
if [[ "$path_ready" == "true" && "$expected_match" == "true" ]]; then
  ready="true"
fi

next_required_action="write_beta_runtime_env"
if [[ "$expected_match" != "true" ]]; then
  next_required_action="run_on_expected_beta_host"
elif [[ "$path_ready" != "true" ]]; then
  next_required_action="bootstrap_beta_host"
fi

bootstrap_command="FISHYSTUFF_GITOPS_ENABLE_BETA_HOST_BOOTSTRAP=1 FISHYSTUFF_GITOPS_ENABLE_BETA_HOST_DIRECTORIES=1 FISHYSTUFF_GITOPS_ENABLE_BETA_HOST_USER_GROUPS=1 just gitops-beta-host-bootstrap-apply"
host_preflight_command="just gitops-beta-runtime-env-host-preflight api_env_file=${api_env_file} dolt_env_file=${dolt_env_file}"
runtime_env_packet_command="just gitops-beta-runtime-env-packet api_env_file=${api_env_file} dolt_env_file=${dolt_env_file}"

printf 'gitops_beta_runtime_env_host_preflight_ok=true\n'
printf 'runtime_env_host_preflight_status=%s\n' "$([[ "$ready" == "true" ]] && printf 'ready' || printf 'blocked')"
printf 'runtime_env_host_preflight_next_required_action=%s\n' "$next_required_action"
printf 'runtime_env_host_preflight_current_hostname=%s\n' "$current_hostname"
printf 'runtime_env_host_preflight_expected_hostname=%s\n' "$expected_hostname"
printf 'runtime_env_host_preflight_expected_hostname_match=%s\n' "$expected_match"
printf 'runtime_env_host_preflight_resident_target=%s\n' "$(deployment_resident_target beta)"
printf 'runtime_env_host_preflight_operator_secretspec_profile=%s\n' "${FISHYSTUFF_OPERATOR_SECRETSPEC_PROFILE:-unset}"
print_env_path_preflight api "$api_env_file"
print_env_path_preflight dolt "$dolt_env_file"
printf 'runtime_env_host_preflight_path_ready=%s\n' "$path_ready"
printf 'runtime_env_host_preflight_ready=%s\n' "$ready"
case "$next_required_action" in
  run_on_expected_beta_host)
    printf 'runtime_env_host_preflight_next_command_01=%s\n' "$host_preflight_command"
    printf 'runtime_env_host_preflight_next_note_01=run this preflight on %s before writing beta runtime env\n' "$expected_hostname"
    ;;
  bootstrap_beta_host)
    printf 'runtime_env_host_preflight_next_command_01=%s\n' "$bootstrap_command"
    ;;
  write_beta_runtime_env)
    printf 'runtime_env_host_preflight_next_command_01=%s\n' "$runtime_env_packet_command"
    ;;
esac
printf 'remote_deploy_performed=false\n'
printf 'infrastructure_mutation_performed=false\n'
printf 'local_host_mutation_performed=false\n'
