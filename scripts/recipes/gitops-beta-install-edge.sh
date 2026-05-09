#!/usr/bin/env bash
set -euo pipefail

SCRIPT_PATH="$(readlink -f "${BASH_SOURCE[0]}")"
SCRIPT_DIR="$(cd "$(dirname "$SCRIPT_PATH")" && pwd)"
source "${SCRIPT_DIR}/lib/common.sh"

edge_bundle="$(normalize_named_arg edge_bundle "${1-auto}")"
proof_dir="$(normalize_named_arg proof_dir "${2-data/gitops}")"
max_age_seconds="$(normalize_named_arg max_age_seconds "${3-86400}")"
install_bin="$(normalize_named_arg install_bin "${4-${FISHYSTUFF_GITOPS_INSTALL_BIN:-install}}")"
systemctl_bin="$(normalize_named_arg systemctl_bin "${5-${FISHYSTUFF_GITOPS_SYSTEMCTL_BIN:-systemctl}}")"

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
    echo "gitops-beta-install-edge requires ${name}=${expected}" >&2
    exit 2
  fi
}

require_env_nonempty() {
  local name="$1"
  local value="${!name-}"

  if [[ -z "$value" ]]; then
    echo "gitops-beta-install-edge requires ${name}" >&2
    exit 2
  fi
}

kv_value() {
  local key="$1"
  local file="$2"
  awk -F= -v key="$key" '$1 == key { print substr($0, length(key) + 2); exit }' "$file"
}

require_command awk
require_command jq
require_command mktemp
require_command sha256sum
require_executable_or_command "$install_bin" install_bin
require_executable_or_command "$systemctl_bin" systemctl_bin

case "$max_age_seconds" in
  '' | *[!0-9]*)
    echo "max_age_seconds must be a non-negative integer, got: ${max_age_seconds}" >&2
    exit 2
    ;;
esac

require_env_value FISHYSTUFF_GITOPS_ENABLE_BETA_EDGE_INSTALL 1
require_env_value FISHYSTUFF_GITOPS_ENABLE_BETA_EDGE_RESTART 1
require_env_nonempty FISHYSTUFF_GITOPS_BETA_EDGE_SERVED_PROOF_SHA256
require_env_nonempty FISHYSTUFF_GITOPS_BETA_EDGE_UNIT_SHA256

proof_index_output="$(mktemp)"
edge_output="$(mktemp)"
cleanup() {
  rm -f "$proof_index_output" "$edge_output"
}
trap cleanup EXIT

if ! bash scripts/recipes/gitops-beta-proof-index.sh "$proof_dir" "$max_age_seconds" true >"$proof_index_output"; then
  cat "$proof_index_output" >&2
  exit 2
fi

proof_complete="$(kv_value gitops_beta_proof_index_complete "$proof_index_output")"
served_proof="$(kv_value gitops_beta_proof_index_served_proof "$proof_index_output")"
served_proof_sha256="$(kv_value gitops_beta_proof_index_served_proof_sha256 "$proof_index_output")"
served_release_id="$(kv_value gitops_beta_proof_index_served_release_id "$proof_index_output")"
served_generation="$(kv_value gitops_beta_proof_index_served_generation "$proof_index_output")"

if [[ "$proof_complete" != "true" ]]; then
  echo "beta proof index is not complete" >&2
  exit 2
fi
if [[ "$served_proof_sha256" != "${FISHYSTUFF_GITOPS_BETA_EDGE_SERVED_PROOF_SHA256}" ]]; then
  echo "FISHYSTUFF_GITOPS_BETA_EDGE_SERVED_PROOF_SHA256 does not match latest beta served proof" >&2
  exit 2
fi
if [[ -z "$served_proof" || ! -f "$served_proof" ]]; then
  echo "latest beta served proof does not exist: ${served_proof}" >&2
  exit 2
fi

if ! bash scripts/recipes/gitops-check-edge-handoff-bundle.sh "$edge_bundle" beta >"$edge_output"; then
  cat "$edge_output" >&2
  exit 2
fi

edge_bundle_path="$(kv_value gitops_edge_handoff_bundle_ok "$edge_output")"
edge_environment="$(kv_value gitops_edge_handoff_environment "$edge_output")"
unit_name="$(kv_value gitops_edge_handoff_unit_name "$edge_output")"
systemd_unit_source="$(kv_value gitops_edge_handoff_systemd_unit "$edge_output")"
edge_caddy_validate="$(kv_value gitops_edge_handoff_caddy_validate "$edge_output")"
edge_site_root="$(kv_value gitops_edge_handoff_site_root "$edge_output")"
edge_cdn_root="$(kv_value gitops_edge_handoff_cdn_root "$edge_output")"
edge_tls_dir="$(kv_value gitops_edge_handoff_tls_dir "$edge_output")"

require_value "$edge_bundle_path" "beta edge handoff bundle check did not report a bundle path"
require_value "$edge_environment" "beta edge handoff bundle check did not report an environment"
require_value "$unit_name" "beta edge handoff bundle check did not report a unit name"
require_value "$systemd_unit_source" "beta edge handoff bundle check did not report a systemd unit"
require_value "$edge_caddy_validate" "beta edge handoff bundle check did not report Caddy validation"

if [[ "$edge_environment" != "beta" ]]; then
  echo "edge handoff bundle environment is not beta: ${edge_environment}" >&2
  exit 2
fi
if [[ "$unit_name" != "fishystuff-beta-edge.service" ]]; then
  echo "edge handoff bundle unit is not beta: ${unit_name}" >&2
  exit 2
fi
if [[ "$edge_caddy_validate" != "true" ]]; then
  echo "edge handoff bundle Caddyfile was not validated" >&2
  exit 2
fi
if [[ "$edge_site_root" != "/var/lib/fishystuff/gitops-beta/served/beta/site" ]]; then
  echo "edge handoff bundle site root is not beta GitOps state: ${edge_site_root}" >&2
  exit 2
fi
if [[ "$edge_cdn_root" != "/var/lib/fishystuff/gitops-beta/served/beta/cdn" ]]; then
  echo "edge handoff bundle CDN root is not beta GitOps state: ${edge_cdn_root}" >&2
  exit 2
fi
if [[ "$edge_tls_dir" != "/run/fishystuff/beta-edge/tls" ]]; then
  echo "edge handoff bundle TLS dir is not beta-only: ${edge_tls_dir}" >&2
  exit 2
fi

bundle_json="${edge_bundle_path}/bundle.json"
if [[ ! -f "$bundle_json" ]]; then
  echo "beta edge handoff bundle does not contain bundle.json: ${bundle_json}" >&2
  exit 2
fi
if [[ ! -f "$systemd_unit_source" ]]; then
  echo "beta edge systemd unit artifact is missing: ${systemd_unit_source}" >&2
  exit 2
fi

systemd_unit_install_path="$(jq -er --arg unit_name "$unit_name" '.backends.systemd.units[] | select(.name == $unit_name) | .install_path' "$bundle_json")"
if [[ "$systemd_unit_install_path" != "/etc/systemd/system/fishystuff-beta-edge.service" ]]; then
  echo "beta edge systemd unit install path is not beta-only: ${systemd_unit_install_path}" >&2
  exit 2
fi

read -r systemd_unit_sha256 _ < <(sha256sum "$systemd_unit_source")
if [[ "$systemd_unit_sha256" != "${FISHYSTUFF_GITOPS_BETA_EDGE_UNIT_SHA256}" ]]; then
  echo "FISHYSTUFF_GITOPS_BETA_EDGE_UNIT_SHA256 does not match beta edge systemd unit" >&2
  exit 2
fi

"$install_bin" -D -m 0644 "$systemd_unit_source" "$systemd_unit_install_path"
"$systemctl_bin" daemon-reload
"$systemctl_bin" restart "$unit_name"
"$systemctl_bin" is-active --quiet "$unit_name"

printf 'gitops_beta_edge_install_ok=%s\n' "$unit_name"
printf 'gitops_beta_edge_install_environment=beta\n'
printf 'gitops_beta_edge_install_bundle=%s\n' "$edge_bundle_path"
printf 'gitops_beta_edge_install_unit_source=%s\n' "$systemd_unit_source"
printf 'gitops_beta_edge_install_unit_target=%s\n' "$systemd_unit_install_path"
printf 'gitops_beta_edge_install_unit_sha256=%s\n' "$systemd_unit_sha256"
printf 'gitops_beta_edge_install_served_proof=%s\n' "$served_proof"
printf 'gitops_beta_edge_install_served_proof_sha256=%s\n' "$served_proof_sha256"
printf 'gitops_beta_edge_install_served_release_id=%s\n' "$served_release_id"
printf 'gitops_beta_edge_install_served_generation=%s\n' "$served_generation"
printf 'gitops_beta_edge_restart_ok=%s\n' "$unit_name"
printf 'local_host_mutation_performed=true\n'
printf 'remote_deploy_performed=false\n'
printf 'infrastructure_mutation_performed=false\n'
