#!/usr/bin/env bash
set -euo pipefail

SCRIPT_PATH="$(readlink -f "${BASH_SOURCE[0]}")"
SCRIPT_DIR="$(cd "$(dirname "$SCRIPT_PATH")" && pwd)"
source "${SCRIPT_DIR}/lib/common.sh"

service="$(normalize_named_arg service "${1-api}")"
bundle="$(normalize_named_arg bundle "${2-auto}")"

cd "$RECIPE_REPO_ROOT"

require_command() {
  local command_name="$1"
  if ! command -v "$command_name" >/dev/null 2>&1; then
    echo "missing required command: ${command_name}" >&2
    exit 127
  fi
}

require_same_path() {
  local label="$1"
  local actual="$2"
  local expected="$3"

  if [[ "$actual" != "$expected" ]]; then
    echo "beta ${service} service bundle ${label} path mismatch" >&2
    echo "actual:   ${actual}" >&2
    echo "expected: ${expected}" >&2
    exit 2
  fi
}

require_bundle_metadata() {
  local label="$1"
  local filter="$2"

  if ! jq -e \
    --arg service_id "$service_id" \
    --arg unit_name "$unit_name" \
    --arg config_destination "$config_destination" \
    --arg runtime_env_target "$runtime_env_target" \
    --arg release_env_target "$release_env_target" \
    --arg systemd_unit_store "$systemd_unit_store" \
    "$filter" \
    "$bundle_json" >/dev/null; then
    echo "beta ${service} service bundle metadata is missing ${label}" >&2
    exit 2
  fi
}

require_unit_line() {
  local label="$1"
  local needle="$2"

  if ! grep -Fx -- "$needle" "$systemd_unit" >/dev/null; then
    echo "beta ${service} service unit is missing ${label}: ${needle}" >&2
    exit 2
  fi
}

require_unit_fragment() {
  local label="$1"
  local needle="$2"

  if ! grep -F -- "$needle" "$systemd_unit" >/dev/null; then
    echo "beta ${service} service unit is missing ${label}: ${needle}" >&2
    exit 2
  fi
}

require_config_line() {
  local label="$1"
  local needle="$2"

  if ! grep -Fx -- "$needle" "$config_file" >/dev/null; then
    echo "beta ${service} service config is missing ${label}: ${needle}" >&2
    exit 2
  fi
}

require_config_fragment() {
  local label="$1"
  local needle="$2"

  if ! grep -F -- "$needle" "$config_file" >/dev/null; then
    echo "beta ${service} service config is missing ${label}: ${needle}" >&2
    exit 2
  fi
}

require_unit_environment_file() {
  local target="$1"

  if grep -Fx -- "EnvironmentFile=${target}" "$systemd_unit" >/dev/null; then
    return
  fi
  if grep -Fx -- "EnvironmentFile=-${target}" "$systemd_unit" >/dev/null; then
    return
  fi
  echo "beta ${service} service unit is missing runtime env file: ${target}" >&2
  exit 2
}

reject_unit_fragment() {
  local label="$1"
  local needle="$2"

  if grep -F -- "$needle" "$systemd_unit" >/dev/null; then
    echo "beta ${service} service unit must not contain ${label}: ${needle}" >&2
    exit 2
  fi
}

reject_config_fragment() {
  local label="$1"
  local needle="$2"

  if grep -F -- "$needle" "$config_file" >/dev/null; then
    echo "beta ${service} service config must not contain ${label}: ${needle}" >&2
    exit 2
  fi
}

require_command jq
require_command sha256sum

case "$service" in
  api)
    auto_package="api-service-bundle-beta-gitops-handoff"
    service_id="fishystuff-beta-api"
    unit_name="fishystuff-beta-api.service"
    config_destination="config.toml"
    runtime_env_target="/var/lib/fishystuff/gitops-beta/api/runtime.env"
    release_env_target="/var/lib/fishystuff/gitops-beta/api/beta.env"
    ;;
  dolt)
    auto_package="dolt-service-bundle-beta-gitops-handoff"
    service_id="fishystuff-beta-dolt"
    unit_name="fishystuff-beta-dolt.service"
    config_destination="sql-server.yaml"
    runtime_env_target="/var/lib/fishystuff/gitops-beta/dolt/beta.env"
    release_env_target=""
    ;;
  *)
    echo "unsupported beta service bundle: ${service}" >&2
    exit 2
    ;;
esac

if [[ "$bundle" == "auto" ]]; then
  require_command nix
  bundle="$(nix build --no-link --print-out-paths ".#${auto_package}" | tail -n 1)"
elif [[ "$bundle" != /* ]]; then
  bundle="${RECIPE_REPO_ROOT}/${bundle}"
fi

if [[ ! -d "$bundle" ]]; then
  echo "beta ${service} service bundle does not exist: ${bundle}" >&2
  exit 2
fi

exe_file="${bundle}/artifacts/exe/main"
config_file="${bundle}/artifacts/config/base"
systemd_unit="${bundle}/artifacts/systemd/unit"
bundle_json="${bundle}/bundle.json"
store_paths="${bundle}/store-paths"

if [[ ! -x "$exe_file" ]]; then
  echo "beta ${service} service executable is missing or not executable: ${exe_file}" >&2
  exit 2
fi
if [[ ! -f "$config_file" ]]; then
  echo "beta ${service} service config is missing: ${config_file}" >&2
  exit 2
fi
if [[ ! -f "$systemd_unit" ]]; then
  echo "beta ${service} service systemd unit is missing: ${systemd_unit}" >&2
  exit 2
fi
if [[ ! -f "$bundle_json" ]]; then
  echo "beta ${service} service bundle metadata is missing: ${bundle_json}" >&2
  exit 2
fi
if [[ ! -f "$store_paths" ]]; then
  echo "beta ${service} service bundle store-paths is missing: ${store_paths}" >&2
  exit 2
fi

exe_store="$(jq -er '.artifacts."exe/main".storePath | select(type == "string" and length > 0)' "$bundle_json")"
config_store="$(jq -er '.artifacts."config/base".storePath | select(type == "string" and length > 0)' "$bundle_json")"
systemd_unit_store="$(jq -er '.artifacts."systemd/unit".storePath | select(type == "string" and length > 0)' "$bundle_json")"
exe_real="$(readlink -f "$exe_file")"
config_real="$(readlink -f "$config_file")"
systemd_unit_real="$(readlink -f "$systemd_unit")"

require_same_path "executable artifact" "$exe_real" "$exe_store"
require_same_path "config artifact" "$config_real" "$config_store"
require_same_path "systemd unit artifact" "$systemd_unit_real" "$systemd_unit_store"

require_bundle_metadata "service ID" '.id == $service_id'
require_bundle_metadata "config artifact" '.artifacts."config/base".destination == $config_destination'
require_bundle_metadata "systemd unit artifact" '.artifacts."systemd/unit".destination == $unit_name and .artifacts."systemd/unit".storePath == $systemd_unit_store'
require_bundle_metadata "runtime env overlay" '.runtimeOverlays[]? | select(.targetPath == $runtime_env_target and .secret == true and .onChange == "restart")'
require_bundle_metadata "systemd unit install" '.backends.systemd.daemon_reload == true and (.backends.systemd.units[]? | select(.name == $unit_name and .install_path == ("/etc/systemd/system/" + $unit_name) and .state == "running" and .startup == "enabled"))'
if [[ -n "$release_env_target" ]]; then
  require_bundle_metadata "GitOps release environment file" '.supervision.environmentFiles[]? | select(. == $release_env_target or . == ("-" + $release_env_target))'
fi

grep -Fx "$config_store" "$store_paths" >/dev/null
grep -Fx "$systemd_unit_store" "$store_paths" >/dev/null

require_unit_fragment "ExecStart" "ExecStart="
require_unit_line "restart policy" "Restart=on-failure"
require_unit_line "install target" "WantedBy=multi-user.target"
require_unit_environment_file "$runtime_env_target"
if [[ -n "$release_env_target" ]]; then
  require_unit_environment_file "$release_env_target"
fi
require_unit_line "beta deployment environment" 'Environment="FISHYSTUFF_DEPLOYMENT_ENVIRONMENT=beta"'
reject_unit_fragment "shared API runtime env" "/run/fishystuff/api/env"
reject_unit_fragment "production service name" "fishystuff-api.service"
reject_unit_fragment "production service name" "fishystuff-dolt.service"

case "$service" in
  api)
    require_unit_line "dynamic user" "DynamicUser=true"
    require_unit_line "private tmp" "PrivateTmp=true"
    require_unit_line "strict system protection" "ProtectSystem=strict"
    require_unit_line "no new privileges" "NoNewPrivileges=true"
    require_unit_line "beta OTEL deployment environment" 'Environment="FISHYSTUFF_OTEL_DEPLOYMENT_ENVIRONMENT=beta"'
    require_unit_fragment "beta loopback bind" "--bind 127.0.0.1:18192"
    reject_unit_fragment "production API user" "User=fishystuff-api"
    reject_unit_fragment "production API group" "Group=fishystuff-api"
    ;;
  dolt)
    require_unit_line "beta Dolt user" "User=fishystuff-beta-dolt"
    require_unit_line "beta Dolt group" "Group=fishystuff-beta-dolt"
    require_unit_line "beta Dolt state directory" "StateDirectory=fishystuff/beta-dolt"
    require_unit_line "state directory mode" "StateDirectoryMode=0750"
    require_unit_line "beta Dolt working directory" "WorkingDirectory=/var/lib/fishystuff/beta-dolt"
    require_unit_line "beta Dolt home" 'Environment="HOME=/var/lib/fishystuff/beta-dolt/home"'
    require_config_fragment "beta SQL port" "port: 3316"
    reject_config_fragment "production Dolt data directory" "/var/lib/fishystuff/dolt"
    reject_unit_fragment "production Dolt user" "User=fishystuff-dolt"
    reject_unit_fragment "production Dolt group" "Group=fishystuff-dolt"
    reject_unit_fragment "production Dolt state directory" "StateDirectory=fishystuff/dolt"
    reject_unit_fragment "dynamic Dolt user" "DynamicUser=true"
    ;;
esac

read -r systemd_unit_sha256 _ < <(sha256sum "$systemd_unit")

printf 'gitops_beta_service_bundle_ok=%s\n' "$bundle"
printf 'gitops_beta_service_bundle_service=%s\n' "$service"
printf 'gitops_beta_service_bundle_service_id=%s\n' "$service_id"
printf 'gitops_beta_service_bundle_unit_name=%s\n' "$unit_name"
printf 'gitops_beta_service_bundle_systemd_unit=%s\n' "$systemd_unit"
printf 'gitops_beta_service_bundle_systemd_unit_store=%s\n' "$systemd_unit_store"
printf 'gitops_beta_service_bundle_systemd_unit_sha256=%s\n' "$systemd_unit_sha256"
printf 'gitops_beta_service_bundle_unit_install_path=/etc/systemd/system/%s\n' "$unit_name"
printf 'gitops_beta_service_bundle_runtime_env_target=%s\n' "$runtime_env_target"
if [[ -n "$release_env_target" ]]; then
  printf 'gitops_beta_service_bundle_release_env_target=%s\n' "$release_env_target"
fi
printf 'gitops_beta_%s_service_bundle_ok=%s\n' "$service" "$bundle"
printf 'gitops_beta_%s_service_bundle_unit_sha256=%s\n' "$service" "$systemd_unit_sha256"
printf 'remote_deploy_performed=false\n'
printf 'infrastructure_mutation_performed=false\n'
