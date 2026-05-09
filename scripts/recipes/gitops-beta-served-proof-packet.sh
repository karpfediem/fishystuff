#!/usr/bin/env bash
set -euo pipefail

SCRIPT_PATH="$(readlink -f "${BASH_SOURCE[0]}")"
SCRIPT_DIR="$(cd "$(dirname "$SCRIPT_PATH")" && pwd)"
source "${SCRIPT_DIR}/lib/common.sh"

proof_dir="$(normalize_named_arg proof_dir "${1-data/gitops}")"
max_age_seconds="$(normalize_named_arg max_age_seconds "${2-86400}")"
draft_file="$(normalize_named_arg draft_file "${3-data/gitops/beta-activation.draft.desired.json}")"
summary_file="$(normalize_named_arg summary_file "${4-data/gitops/beta-current.handoff-summary.json}")"
admission_file="$(normalize_named_arg admission_file "${5-data/gitops/beta-admission.evidence.json}")"
operator_proof_file="$(normalize_named_arg proof_file "${6-}")"
deploy_bin="$(normalize_named_arg deploy_bin "${7-auto}")"
state_dir="$(normalize_named_arg state_dir "${8-/var/lib/fishystuff/gitops-beta}")"
run_dir="$(normalize_named_arg run_dir "${9-/run/fishystuff/gitops-beta}")"
edge_bundle="$(normalize_named_arg edge_bundle "${10-auto}")"
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

require_command awk
require_command grep
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

proof_dir="$(absolute_path "$proof_dir")"
draft_file="$(absolute_path "$draft_file")"
summary_file="$(absolute_path "$summary_file")"
admission_file="$(absolute_path "$admission_file")"
operator_proof_file="$(absolute_path_or_empty "$operator_proof_file")"
state_dir="$(absolute_path "$state_dir")"
run_dir="$(absolute_path "$run_dir")"
edge_bundle="$(absolute_path_or_auto "$edge_bundle")"
observation_dir="$(absolute_path "$observation_dir")"

bash scripts/recipes/gitops-check-handoff-summary.sh "$summary_file" >/dev/null 2>&1
environment="$(jq -er '.environment.name | select(type == "string" and length > 0)' "$summary_file")"
if [[ "$environment" != "beta" ]]; then
  echo "gitops-beta-served-proof-packet requires a beta handoff summary, got: ${environment}" >&2
  exit 2
fi

read -r handoff_summary_sha256 _ < <(sha256sum "$summary_file")
active_release_id="$(jq -er '.environment.active_release | select(type == "string" and length > 0)' "$summary_file")"

tmp_dir="$(mktemp -d)"
cleanup() {
  rm -rf "$tmp_dir"
}
trap cleanup EXIT

operator_packet_output="${tmp_dir}/operator-packet.out"
if ! bash scripts/recipes/gitops-beta-operator-proof-packet.sh \
  "$operator_proof_file" \
  "$proof_dir" \
  "$max_age_seconds" \
  "$draft_file" \
  "$summary_file" \
  "$admission_file" \
  "$edge_bundle" \
  "$deploy_bin" \
  "$api_upstream" \
  "$observation_dir" >"$operator_packet_output"; then
  cat "$operator_packet_output" >&2 || true
  exit 2
fi

operator_status="$(kv_value operator_proof_packet_status "$operator_packet_output")"
operator_next_command="$(kv_value operator_proof_packet_next_command_01 "$operator_packet_output")"
operator_proof_file="$(kv_value operator_proof_packet_proof_file "$operator_packet_output")"
operator_proof_sha256="$(kv_value operator_proof_packet_proof_sha256 "$operator_packet_output")"

status="$operator_status"
served_proof=""
served_proof_sha256=""
served_generation=""
served_state_status="not_checked"
if [[ "$operator_status" == "ready" ]]; then
  if bash scripts/recipes/gitops-beta-verify-activation-served.sh \
    "$draft_file" \
    "$summary_file" \
    "$admission_file" \
    "$deploy_bin" \
    "$state_dir" \
    "$run_dir" >"${tmp_dir}/served-verify.out" 2>"${tmp_dir}/served-verify.err"; then
    served_state_status="verified"
    status="missing_served_proof"
  else
    if [[ -f "${state_dir}/status/beta.json" ]]; then
      cat "${tmp_dir}/served-verify.out" >&2 || true
      cat "${tmp_dir}/served-verify.err" >&2 || true
      exit 2
    fi
    served_state_status="missing"
    status="missing_served_state"
  fi
fi

if [[ "$status" == "missing_served_proof" ]]; then
  if bash scripts/recipes/gitops-beta-proof-index.sh "$proof_dir" "$max_age_seconds" false >"${tmp_dir}/proof-index.out"; then
    index_status="$(kv_value gitops_beta_proof_index_status "${tmp_dir}/proof-index.out")"
    index_complete="$(kv_value gitops_beta_proof_index_complete "${tmp_dir}/proof-index.out")"
    served_proof="$(kv_value gitops_beta_proof_index_served_proof "${tmp_dir}/proof-index.out")"
    served_proof_sha256="$(kv_value gitops_beta_proof_index_served_proof_sha256 "${tmp_dir}/proof-index.out")"
    served_generation="$(kv_value gitops_beta_proof_index_served_generation "${tmp_dir}/proof-index.out")"
    if [[ "$index_complete" == "true" ]]; then
      status="ready"
    elif [[ "$index_status" != "missing_served_proof" ]]; then
      cat "${tmp_dir}/proof-index.out" >&2
      exit 2
    fi
  else
    cat "${tmp_dir}/proof-index.out" >&2 || true
    exit 2
  fi
fi

operator_packet_command="just gitops-beta-operator-proof-packet proof_file=${operator_proof_file} proof_dir=${proof_dir} max_age_seconds=${max_age_seconds} draft_file=${draft_file} summary_file=${summary_file} admission_file=${admission_file} edge_bundle=${edge_bundle} deploy_bin=${deploy_bin} api_upstream=${api_upstream} observation_dir=${observation_dir}"
served_proof_command="just gitops-beta-served-proof output_dir=${proof_dir} draft_file=${draft_file} summary_file=${summary_file} admission_file=${admission_file} proof_file=${operator_proof_file} deploy_bin=${deploy_bin} state_dir=${state_dir} run_dir=${run_dir} proof_max_age_seconds=${max_age_seconds}"
proof_index_command="just gitops-beta-proof-index proof_dir=${proof_dir} max_age_seconds=${max_age_seconds} require_complete=true"
edge_install_command="just gitops-beta-install-edge edge_bundle=${edge_bundle} proof_dir=${proof_dir} max_age_seconds=${max_age_seconds}"

printf 'gitops_beta_served_proof_packet_ok=true\n'
printf 'served_proof_packet_status=%s\n' "$status"
printf 'served_proof_packet_summary_file=%s\n' "$summary_file"
printf 'served_proof_packet_summary_sha256=%s\n' "$handoff_summary_sha256"
printf 'served_proof_packet_admission_file=%s\n' "$admission_file"
printf 'served_proof_packet_draft_file=%s\n' "$draft_file"
printf 'served_proof_packet_proof_dir=%s\n' "$proof_dir"
printf 'served_proof_packet_operator_proof_file=%s\n' "$operator_proof_file"
printf 'served_proof_packet_operator_proof_sha256=%s\n' "$operator_proof_sha256"
printf 'served_proof_packet_served_proof_file=%s\n' "$served_proof"
printf 'served_proof_packet_served_proof_sha256=%s\n' "$served_proof_sha256"
printf 'served_proof_packet_served_generation=%s\n' "$served_generation"
printf 'served_proof_packet_served_state_status=%s\n' "$served_state_status"
printf 'served_proof_packet_edge_bundle=%s\n' "$edge_bundle"
printf 'served_proof_packet_deploy_bin=%s\n' "$deploy_bin"
printf 'served_proof_packet_state_dir=%s\n' "$state_dir"
printf 'served_proof_packet_run_dir=%s\n' "$run_dir"
printf 'served_proof_packet_release_id=%s\n' "$active_release_id"
case "$status" in
  missing_activation_draft)
    if [[ -n "$operator_next_command" ]]; then
      printf 'served_proof_packet_next_command_01=%s\n' "$operator_next_command"
      printf 'served_proof_packet_after_success_command=%s\n' "$operator_packet_command"
    else
      printf 'served_proof_packet_next_command_01=%s\n' "$operator_packet_command"
    fi
    ;;
  missing_operator_proof)
    printf 'served_proof_packet_next_command_01=%s\n' "$operator_packet_command"
    if [[ -n "$operator_next_command" ]]; then
      printf 'served_proof_packet_operator_next_command=%s\n' "$operator_next_command"
    fi
    ;;
  missing_served_state)
    printf 'served_proof_packet_next_command_01=%s\n' "$operator_next_command"
    printf 'served_proof_packet_after_success_command=%s\n' "$served_proof_command"
    ;;
  missing_served_proof)
    printf 'served_proof_packet_next_command_01=%s\n' "$served_proof_command"
    printf 'served_proof_packet_after_success_command=%s\n' "$proof_index_command"
    ;;
  ready)
    printf 'served_proof_packet_next_command_01=%s\n' "$proof_index_command"
    printf 'served_proof_packet_after_success_command=%s\n' "$edge_install_command"
    ;;
esac
printf 'remote_deploy_performed=false\n'
printf 'infrastructure_mutation_performed=false\n'
printf 'local_host_mutation_performed=false\n'
