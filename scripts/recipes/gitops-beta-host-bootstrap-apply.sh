#!/usr/bin/env bash
set -euo pipefail

SCRIPT_PATH="$(readlink -f "${BASH_SOURCE[0]}")"
SCRIPT_DIR="$(cd "$(dirname "$SCRIPT_PATH")" && pwd)"
source "${SCRIPT_DIR}/lib/common.sh"

install_bin="$(normalize_named_arg install_bin "${1-${FISHYSTUFF_GITOPS_INSTALL_BIN:-install}}")"
groupadd_bin="$(normalize_named_arg groupadd_bin "${2-${FISHYSTUFF_GITOPS_GROUPADD_BIN:-groupadd}}")"
useradd_bin="$(normalize_named_arg useradd_bin "${3-${FISHYSTUFF_GITOPS_USERADD_BIN:-useradd}}")"
getent_bin="$(normalize_named_arg getent_bin "${4-${FISHYSTUFF_GITOPS_GETENT_BIN:-getent}}")"

cd "$RECIPE_REPO_ROOT"

require_command() {
  local command_name="$1"
  if ! command -v "$command_name" >/dev/null 2>&1; then
    echo "missing required command: ${command_name}" >&2
    exit 127
  fi
}

require_executable_or_command() {
  local command_name="$1"
  local label="$2"

  if [[ "$command_name" == */* ]]; then
    if [[ ! -x "$command_name" ]]; then
      echo "${label} is not executable: ${command_name}" >&2
      exit 127
    fi
    return
  fi
  require_command "$command_name"
}

require_env_value() {
  local name="$1"
  local expected="$2"
  local value="${!name-}"

  if [[ "$value" != "$expected" ]]; then
    echo "gitops-beta-host-bootstrap-apply requires ${name}=${expected}" >&2
    exit 2
  fi
}

kv_value() {
  local key="$1"
  local file="$2"
  awk -F= -v key="$key" '$1 == key { print substr($0, length(key) + 2); exit }' "$file"
}

require_plan_value() {
  local key="$1"
  local file="$2"
  local value=""

  value="$(kv_value "$key" "$file")"
  if [[ -z "$value" ]]; then
    echo "beta host bootstrap plan did not report ${key}" >&2
    exit 2
  fi
  printf '%s' "$value"
}

require_plan_equals() {
  local key="$1"
  local expected="$2"
  local file="$3"
  local value=""

  value="$(require_plan_value "$key" "$file")"
  if [[ "$value" != "$expected" ]]; then
    echo "beta host bootstrap plan ${key} must be ${expected}, got: ${value}" >&2
    exit 2
  fi
}

require_beta_directory_path() {
  local path="$1"

  case "$path" in
    /var/lib/fishystuff/gitops-beta | \
    /var/lib/fishystuff/gitops-beta/* | \
    /var/lib/fishystuff/beta-dolt | \
    /run/fishystuff/gitops-beta | \
    /run/fishystuff/gitops-beta/* | \
    /var/lib/fishystuff/gitops-beta/tls/live)
      ;;
    *)
      echo "refusing to bootstrap non-beta directory: ${path}" >&2
      exit 2
      ;;
  esac
}

require_bootstrap_plan_safety() {
  local plan_output="$1"

  require_plan_equals gitops_beta_host_bootstrap_plan_ok true "$plan_output"
  require_plan_equals deployment beta "$plan_output"
  require_plan_equals deployment_environment beta "$plan_output"
  require_plan_equals resident_expected_hostname_match true "$plan_output"
  require_plan_equals api_runtime_env_path /var/lib/fishystuff/gitops-beta/api/runtime.env "$plan_output"
  require_plan_equals api_release_env_path /var/lib/fishystuff/gitops-beta/api/beta.env "$plan_output"
  require_plan_equals dolt_runtime_env_path /var/lib/fishystuff/gitops-beta/dolt/beta.env "$plan_output"
  require_plan_equals service_unit_01 fishystuff-beta-dolt.service "$plan_output"
  require_plan_equals service_unit_02 fishystuff-beta-api.service "$plan_output"
  require_plan_equals service_unit_03 fishystuff-beta-edge.service "$plan_output"
  require_plan_equals remote_deploy_performed false "$plan_output"
  require_plan_equals infrastructure_mutation_performed false "$plan_output"
  require_plan_equals local_host_mutation_performed false "$plan_output"
}

ensure_group() {
  local group="$1"

  if "$getent_bin" group "$group" >/dev/null 2>&1; then
    printf 'existing'
    return
  fi

  "$groupadd_bin" --system "$group"
  printf 'created'
}

ensure_user() {
  local user="$1"
  local group="$2"
  local home="$3"

  if "$getent_bin" passwd "$user" >/dev/null 2>&1; then
    printf 'existing'
    return
  fi

  "$useradd_bin" --system --gid "$group" --home-dir "$home" --no-create-home "$user"
  printf 'created'
}

require_env_value FISHYSTUFF_GITOPS_ENABLE_BETA_HOST_BOOTSTRAP 1
require_env_value FISHYSTUFF_GITOPS_ENABLE_BETA_HOST_DIRECTORIES 1
require_env_value FISHYSTUFF_GITOPS_ENABLE_BETA_HOST_USER_GROUPS 1
require_command awk
require_command mktemp
require_executable_or_command "$install_bin" install_bin
require_executable_or_command "$groupadd_bin" groupadd_bin
require_executable_or_command "$useradd_bin" useradd_bin
require_executable_or_command "$getent_bin" getent_bin

plan_output="$(mktemp)"
cleanup() {
  rm -f "$plan_output"
}
trap cleanup EXIT

if ! bash scripts/recipes/gitops-beta-host-bootstrap-plan.sh >"$plan_output"; then
  cat "$plan_output" >&2
  exit 2
fi

require_bootstrap_plan_safety "$plan_output"

group="$(require_plan_value required_system_group_01 "$plan_output")"
user_pair="$(require_plan_value required_system_user_01 "$plan_output")"
user="${user_pair%%:*}"
user_group="${user_pair#*:}"

if [[ "$group" != "fishystuff-beta-dolt" || "$user" != "fishystuff-beta-dolt" || "$user_group" != "$group" ]]; then
  echo "beta host bootstrap plan reported unexpected user/group: group=${group} user_pair=${user_pair}" >&2
  exit 2
fi

group_action="$(ensure_group "$group")"
user_action="$(ensure_user "$user" "$group" /var/lib/fishystuff/beta-dolt/home)"

printf 'gitops_beta_host_bootstrap_apply_ok=true\n'
printf 'gitops_beta_host_bootstrap_current_hostname=%s\n' "$(require_plan_value current_hostname "$plan_output")"
printf 'gitops_beta_host_bootstrap_expected_hostname=%s\n' "$(require_plan_value resident_expected_hostname "$plan_output")"
printf 'gitops_beta_host_bootstrap_expected_hostname_match=%s\n' "$(require_plan_value resident_expected_hostname_match "$plan_output")"
printf 'gitops_beta_host_bootstrap_group=%s\n' "$group"
printf 'gitops_beta_host_bootstrap_group_action=%s\n' "$group_action"
printf 'gitops_beta_host_bootstrap_user=%s\n' "$user"
printf 'gitops_beta_host_bootstrap_user_action=%s\n' "$user_action"

for number in 01 02 03 04 05 06 07 08; do
  path="$(require_plan_value "required_directory_${number}_path" "$plan_output")"
  mode="$(require_plan_value "required_directory_${number}_mode" "$plan_output")"

  require_beta_directory_path "$path"
  "$install_bin" -d -m "$mode" "$path"
  printf 'gitops_beta_host_bootstrap_directory_%s=%s:%s\n' "$number" "$mode" "$path"
done

printf 'gitops_beta_host_bootstrap_env_file_01=%s\n' "$(require_plan_value api_runtime_env_path "$plan_output")"
printf 'gitops_beta_host_bootstrap_env_file_02=%s\n' "$(require_plan_value api_release_env_path "$plan_output")"
printf 'gitops_beta_host_bootstrap_env_file_03=%s\n' "$(require_plan_value dolt_runtime_env_path "$plan_output")"
printf 'local_host_mutation_performed=true\n'
printf 'remote_deploy_performed=false\n'
printf 'infrastructure_mutation_performed=false\n'
