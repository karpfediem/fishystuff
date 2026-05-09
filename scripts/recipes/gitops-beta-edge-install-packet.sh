#!/usr/bin/env bash
set -euo pipefail

SCRIPT_PATH="$(readlink -f "${BASH_SOURCE[0]}")"
SCRIPT_DIR="$(cd "$(dirname "$SCRIPT_PATH")" && pwd)"
source "${SCRIPT_DIR}/lib/common.sh"

edge_bundle="$(normalize_named_arg edge_bundle "${1-auto}")"
proof_dir="$(normalize_named_arg proof_dir "${2-data/gitops}")"
max_age_seconds="$(normalize_named_arg max_age_seconds "${3-86400}")"
draft_file="$(normalize_named_arg draft_file "${4-data/gitops/beta-activation.draft.desired.json}")"
summary_file="$(normalize_named_arg summary_file "${5-data/gitops/beta-current.handoff-summary.json}")"
admission_file="$(normalize_named_arg admission_file "${6-data/gitops/beta-admission.evidence.json}")"
operator_proof_file="$(normalize_named_arg proof_file "${7-}")"
deploy_bin="$(normalize_named_arg deploy_bin "${8-auto}")"
state_dir="$(normalize_named_arg state_dir "${9-/var/lib/fishystuff/gitops-beta}")"
run_dir="$(normalize_named_arg run_dir "${10-/run/fishystuff/gitops-beta}")"
api_upstream="$(normalize_named_arg api_upstream "${11-http://127.0.0.1:18192}")"
observation_dir="$(normalize_named_arg observation_dir "${12-data/gitops/beta-admission-observations}")"

cd "$RECIPE_REPO_ROOT"

require_command() {
  local command_name="$1"
  if ! command -v "$command_name" >/dev/null 2>&1; then
    echo "missing required command: ${command_name}" >&2
    exit 127
  fi
}

absolute_path() {
  local path="$1"
  if [[ "$path" == /* ]]; then
    printf '%s' "$path"
    return
  fi
  printf '%s/%s' "$RECIPE_REPO_ROOT" "$path"
}

absolute_path_or_empty() {
  local path="$1"
  if [[ -z "$path" ]]; then
    printf ''
    return
  fi
  absolute_path "$path"
}

absolute_path_or_auto() {
  local path="$1"
  if [[ "$path" == "auto" ]]; then
    printf '%s' "$path"
    return
  fi
  absolute_path "$path"
}

kv_value() {
  local key="$1"
  local file="$2"
  awk -F= -v key="$key" '$1 == key { print substr($0, length(key) + 2); exit }' "$file"
}

require_edge_kv() {
  local key="$1"
  local value="$2"
  if [[ -z "$value" ]]; then
    echo "beta edge install packet missing ${key} from edge handoff bundle check" >&2
    exit 2
  fi
}

require_command awk
require_command jq
require_command mktemp
require_command sha256sum

case "$max_age_seconds" in
  '' | *[!0-9]*)
    echo "max_age_seconds must be a non-negative integer, got: ${max_age_seconds}" >&2
    exit 2
    ;;
esac
if [[ "$api_upstream" == */ ]]; then
  echo "api_upstream must not end with /" >&2
  exit 2
fi
if [[ "$api_upstream" =~ ^[A-Za-z][A-Za-z0-9+.-]*://[^/?#]*@ ]]; then
  echo "api_upstream must not contain embedded credentials" >&2
  exit 2
fi
require_loopback_http_url api_upstream "$api_upstream"

current_hostname="$(deployment_current_hostname)"
expected_hostname="$(deployment_resident_hostname beta)"
expected_hostname_match="$(deployment_hostname_match_status "$current_hostname" "$expected_hostname")"
resident_target="$(deployment_resident_target beta)"

edge_bundle="$(absolute_path_or_auto "$edge_bundle")"
proof_dir="$(absolute_path "$proof_dir")"
draft_file="$(absolute_path "$draft_file")"
summary_file="$(absolute_path "$summary_file")"
admission_file="$(absolute_path "$admission_file")"
operator_proof_file="$(absolute_path_or_empty "$operator_proof_file")"
state_dir="$(absolute_path "$state_dir")"
run_dir="$(absolute_path "$run_dir")"
observation_dir="$(absolute_path "$observation_dir")"

tmp_dir="$(mktemp -d)"
cleanup() {
  rm -rf "$tmp_dir"
}
trap cleanup EXIT

proof_index_output="${tmp_dir}/proof-index.out"
bash scripts/recipes/gitops-beta-proof-index.sh "$proof_dir" "$max_age_seconds" false >"$proof_index_output"

proof_index_status="$(kv_value gitops_beta_proof_index_status "$proof_index_output")"
proof_index_complete="$(kv_value gitops_beta_proof_index_complete "$proof_index_output")"
proof_index_operator_proof="$(kv_value gitops_beta_proof_index_operator_proof "$proof_index_output")"
proof_index_served_proof="$(kv_value gitops_beta_proof_index_served_proof "$proof_index_output")"
proof_index_served_proof_sha256="$(kv_value gitops_beta_proof_index_served_proof_sha256 "$proof_index_output")"
proof_index_served_release_id="$(kv_value gitops_beta_proof_index_served_release_id "$proof_index_output")"
proof_index_served_generation="$(kv_value gitops_beta_proof_index_served_generation "$proof_index_output")"

status="missing_complete_proof_chain"
edge_bundle_path=""
unit_name=""
systemd_unit_source=""
systemd_unit_target=""
systemd_unit_sha256=""
edge_caddy_validate=""
edge_site_root=""
edge_cdn_root=""
edge_tls_dir=""

case "$proof_index_status" in
  complete)
    status="ready"
    ;;
  missing_proof_dir | missing_operator_proof | missing_served_proof)
    status="missing_complete_proof_chain"
    ;;
  *)
    cat "$proof_index_output" >&2
    exit 2
    ;;
esac

if [[ "$proof_index_complete" == "true" && "$status" == "ready" ]]; then
  edge_output="${tmp_dir}/edge.out"
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

  require_edge_kv gitops_edge_handoff_bundle_ok "$edge_bundle_path"
  require_edge_kv gitops_edge_handoff_environment "$edge_environment"
  require_edge_kv gitops_edge_handoff_unit_name "$unit_name"
  require_edge_kv gitops_edge_handoff_systemd_unit "$systemd_unit_source"
  require_edge_kv gitops_edge_handoff_caddy_validate "$edge_caddy_validate"

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
  if [[ "$edge_tls_dir" != "/var/lib/fishystuff/gitops-beta/tls/live" ]]; then
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
  systemd_unit_target="$(jq -er --arg unit_name "$unit_name" '.backends.systemd.units[] | select(.name == $unit_name) | .install_path' "$bundle_json")"
  if [[ "$systemd_unit_target" != "/etc/systemd/system/fishystuff-beta-edge.service" ]]; then
    echo "beta edge systemd unit install path is not beta-only: ${systemd_unit_target}" >&2
    exit 2
  fi
  read -r systemd_unit_sha256 _ < <(sha256sum "$systemd_unit_source")
fi

served_packet_command="just gitops-beta-served-proof-packet proof_dir=${proof_dir} max_age_seconds=${max_age_seconds} draft_file=${draft_file} summary_file=${summary_file} admission_file=${admission_file} proof_file=${operator_proof_file} deploy_bin=${deploy_bin} state_dir=${state_dir} run_dir=${run_dir} edge_bundle=${edge_bundle} api_upstream=${api_upstream} observation_dir=${observation_dir}"
proof_index_command="just gitops-beta-proof-index proof_dir=${proof_dir} max_age_seconds=${max_age_seconds} require_complete=true"

printf 'gitops_beta_edge_install_packet_ok=true\n'
printf 'edge_install_packet_status=%s\n' "$status"
printf 'edge_install_packet_current_hostname=%s\n' "$current_hostname"
printf 'edge_install_packet_expected_hostname=%s\n' "$expected_hostname"
printf 'edge_install_packet_expected_hostname_match=%s\n' "$expected_hostname_match"
printf 'edge_install_packet_resident_target=%s\n' "$resident_target"
printf 'edge_install_packet_proof_dir=%s\n' "$proof_dir"
printf 'edge_install_packet_proof_index_status=%s\n' "$proof_index_status"
printf 'edge_install_packet_proof_index_complete=%s\n' "$proof_index_complete"
printf 'edge_install_packet_operator_proof=%s\n' "$proof_index_operator_proof"
printf 'edge_install_packet_served_proof=%s\n' "$proof_index_served_proof"
printf 'edge_install_packet_served_proof_sha256=%s\n' "$proof_index_served_proof_sha256"
printf 'edge_install_packet_served_release_id=%s\n' "$proof_index_served_release_id"
printf 'edge_install_packet_served_generation=%s\n' "$proof_index_served_generation"
printf 'edge_install_packet_edge_bundle=%s\n' "$edge_bundle_path"
printf 'edge_install_packet_unit_name=%s\n' "$unit_name"
printf 'edge_install_packet_unit_source=%s\n' "$systemd_unit_source"
printf 'edge_install_packet_unit_target=%s\n' "$systemd_unit_target"
printf 'edge_install_packet_unit_sha256=%s\n' "$systemd_unit_sha256"
printf 'edge_install_packet_caddy_validate=%s\n' "$edge_caddy_validate"
printf 'edge_install_packet_site_root=%s\n' "$edge_site_root"
printf 'edge_install_packet_cdn_root=%s\n' "$edge_cdn_root"
printf 'edge_install_packet_tls_dir=%s\n' "$edge_tls_dir"
case "$status" in
  missing_complete_proof_chain)
    printf 'edge_install_packet_next_command_01=%s\n' "$served_packet_command"
    ;;
  ready)
    printf 'edge_install_packet_review_command=%s\n' "$proof_index_command"
    printf 'edge_install_packet_next_command_01=FISHYSTUFF_GITOPS_ENABLE_BETA_EDGE_INSTALL=1 FISHYSTUFF_GITOPS_ENABLE_BETA_EDGE_RESTART=1 FISHYSTUFF_GITOPS_BETA_EDGE_SERVED_PROOF_SHA256=%s FISHYSTUFF_GITOPS_BETA_EDGE_UNIT_SHA256=%s just gitops-beta-install-edge edge_bundle=%s proof_dir=%s max_age_seconds=%s\n' \
      "$proof_index_served_proof_sha256" \
      "$systemd_unit_sha256" \
      "$edge_bundle_path" \
      "$proof_dir" \
      "$max_age_seconds"
    ;;
esac
printf 'remote_deploy_performed=false\n'
printf 'infrastructure_mutation_performed=false\n'
printf 'local_host_mutation_performed=false\n'
