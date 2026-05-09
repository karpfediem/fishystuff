#!/usr/bin/env bash
set -euo pipefail

SCRIPT_PATH="$(readlink -f "${BASH_SOURCE[0]}")"
SCRIPT_DIR="$(cd "$(dirname "$SCRIPT_PATH")" && pwd)"
source "${SCRIPT_DIR}/lib/common.sh"

service="$(normalize_named_arg service "${1-api}")"
bundle="$(normalize_named_arg bundle "${2-auto}")"
install_bin="$(normalize_named_arg install_bin "${3-${FISHYSTUFF_GITOPS_INSTALL_BIN:-install}}")"
systemctl_bin="$(normalize_named_arg systemctl_bin "${4-${FISHYSTUFF_GITOPS_SYSTEMCTL_BIN:-systemctl}}")"

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
    echo "gitops-beta-install-service requires ${name}=${expected}" >&2
    exit 2
  fi
}

require_env_nonempty() {
  local name="$1"
  local value="${!name-}"

  if [[ -z "$value" ]]; then
    echo "gitops-beta-install-service requires ${name}" >&2
    exit 2
  fi
}

kv_value() {
  local key="$1"
  local file="$2"
  awk -F= -v key="$key" '$1 == key { print substr($0, length(key) + 2); exit }' "$file"
}

case "$service" in
  api)
    install_flag="FISHYSTUFF_GITOPS_ENABLE_BETA_API_INSTALL"
    restart_flag="FISHYSTUFF_GITOPS_ENABLE_BETA_API_RESTART"
    unit_hash_var="FISHYSTUFF_GITOPS_BETA_API_UNIT_SHA256"
    expected_unit_name="fishystuff-beta-api.service"
    ;;
  dolt)
    install_flag="FISHYSTUFF_GITOPS_ENABLE_BETA_DOLT_INSTALL"
    restart_flag="FISHYSTUFF_GITOPS_ENABLE_BETA_DOLT_RESTART"
    unit_hash_var="FISHYSTUFF_GITOPS_BETA_DOLT_UNIT_SHA256"
    expected_unit_name="fishystuff-beta-dolt.service"
    ;;
  *)
    echo "unsupported beta service install: ${service}" >&2
    exit 2
    ;;
esac

require_command awk
require_command mktemp
require_executable_or_command "$install_bin" install_bin
require_executable_or_command "$systemctl_bin" systemctl_bin

require_env_value "$install_flag" 1
require_env_value "$restart_flag" 1
require_env_nonempty "$unit_hash_var"

bundle_output="$(mktemp)"
cleanup() {
  rm -f "$bundle_output"
}
trap cleanup EXIT

if ! bash scripts/recipes/gitops-check-beta-service-bundle.sh "$service" "$bundle" >"$bundle_output"; then
  cat "$bundle_output" >&2
  exit 2
fi

bundle_path="$(kv_value gitops_beta_service_bundle_ok "$bundle_output")"
unit_name="$(kv_value gitops_beta_service_bundle_unit_name "$bundle_output")"
systemd_unit_source="$(kv_value gitops_beta_service_bundle_systemd_unit "$bundle_output")"
systemd_unit_sha256="$(kv_value gitops_beta_service_bundle_systemd_unit_sha256 "$bundle_output")"
systemd_unit_install_path="$(kv_value gitops_beta_service_bundle_unit_install_path "$bundle_output")"
runtime_env_target="$(kv_value gitops_beta_service_bundle_runtime_env_target "$bundle_output")"
expected_unit_sha256="${!unit_hash_var}"

require_value "$bundle_path" "beta service bundle check did not report a bundle path"
require_value "$unit_name" "beta service bundle check did not report a unit name"
require_value "$systemd_unit_source" "beta service bundle check did not report a systemd unit"
require_value "$systemd_unit_sha256" "beta service bundle check did not report a systemd unit hash"
require_value "$systemd_unit_install_path" "beta service bundle check did not report a unit install path"

if [[ "$unit_name" != "$expected_unit_name" ]]; then
  echo "beta ${service} service bundle unit is wrong: ${unit_name}" >&2
  exit 2
fi
if [[ "$systemd_unit_install_path" != "/etc/systemd/system/${expected_unit_name}" ]]; then
  echo "beta ${service} service unit install path is wrong: ${systemd_unit_install_path}" >&2
  exit 2
fi
if [[ "$systemd_unit_sha256" != "$expected_unit_sha256" ]]; then
  echo "${unit_hash_var} does not match beta ${service} systemd unit" >&2
  exit 2
fi

"$install_bin" -D -m 0644 "$systemd_unit_source" "$systemd_unit_install_path"
"$systemctl_bin" daemon-reload
"$systemctl_bin" restart "$unit_name"
"$systemctl_bin" is-active --quiet "$unit_name"

printf 'gitops_beta_service_install_ok=%s\n' "$unit_name"
printf 'gitops_beta_service_install_service=%s\n' "$service"
printf 'gitops_beta_service_install_bundle=%s\n' "$bundle_path"
printf 'gitops_beta_service_install_unit_source=%s\n' "$systemd_unit_source"
printf 'gitops_beta_service_install_unit_target=%s\n' "$systemd_unit_install_path"
printf 'gitops_beta_service_install_unit_sha256=%s\n' "$systemd_unit_sha256"
printf 'gitops_beta_service_install_runtime_env_target=%s\n' "$runtime_env_target"
printf 'gitops_beta_service_restart_ok=%s\n' "$unit_name"
printf 'gitops_beta_%s_service_install_ok=%s\n' "$service" "$unit_name"
printf 'local_host_mutation_performed=true\n'
printf 'remote_deploy_performed=false\n'
printf 'infrastructure_mutation_performed=false\n'
