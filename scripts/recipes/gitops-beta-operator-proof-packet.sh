#!/usr/bin/env bash
set -euo pipefail

SCRIPT_PATH="$(readlink -f "${BASH_SOURCE[0]}")"
SCRIPT_DIR="$(cd "$(dirname "$SCRIPT_PATH")" && pwd)"
source "${SCRIPT_DIR}/lib/common.sh"

proof_file="$(normalize_named_arg proof_file "${1-}")"
proof_dir="$(normalize_named_arg proof_dir "${2-data/gitops}")"
max_age_seconds="$(normalize_named_arg max_age_seconds "${3-86400}")"
draft_file="$(normalize_named_arg draft_file "${4-data/gitops/beta-activation.draft.desired.json}")"
summary_file="$(normalize_named_arg summary_file "${5-data/gitops/beta-current.handoff-summary.json}")"
admission_file="$(normalize_named_arg admission_file "${6-data/gitops/beta-admission.evidence.json}")"
edge_bundle="$(normalize_named_arg edge_bundle "${7-auto}")"
deploy_bin="$(normalize_named_arg deploy_bin "${8-auto}")"
api_upstream="$(normalize_named_arg api_upstream "${9-http://127.0.0.1:18192}")"
observation_dir="$(normalize_named_arg observation_dir "${10-data/gitops/beta-admission-observations}")"

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

latest_beta_operator_proof() {
  local dir="$1"
  if [[ ! -d "$dir" ]]; then
    return
  fi
  find "$dir" -maxdepth 1 -type f -name 'beta-operator-proof.*.json' -printf '%T@ %p\n' \
    | sort -nr \
    | awk 'NR == 1 { $1 = ""; sub(/^ /, ""); print }'
}

require_command awk
require_command find
require_command grep
require_command jq
require_command mktemp
require_command sha256sum
require_command sort

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

proof_file="$(absolute_path_or_empty "$proof_file")"
proof_dir="$(absolute_path "$proof_dir")"
draft_file="$(absolute_path "$draft_file")"
summary_file="$(absolute_path "$summary_file")"
admission_file="$(absolute_path "$admission_file")"
edge_bundle="$(absolute_path_or_auto "$edge_bundle")"
observation_dir="$(absolute_path "$observation_dir")"

bash scripts/recipes/gitops-check-handoff-summary.sh "$summary_file" >/dev/null 2>&1
environment="$(jq -er '.environment.name | select(type == "string" and length > 0)' "$summary_file")"
if [[ "$environment" != "beta" ]]; then
  echo "gitops-beta-operator-proof-packet requires a beta handoff summary, got: ${environment}" >&2
  exit 2
fi

read -r handoff_summary_sha256 _ < <(sha256sum "$summary_file")
active_release_id="$(jq -er '.environment.active_release | select(type == "string" and length > 0)' "$summary_file")"

tmp_dir="$(mktemp -d)"
cleanup() {
  rm -rf "$tmp_dir"
}
trap cleanup EXIT

activation_packet_command="just gitops-beta-activation-draft-packet draft_file=${draft_file} summary_file=${summary_file} admission_file=${admission_file} proof_dir=${proof_dir} edge_bundle=${edge_bundle} deploy_bin=${deploy_bin} api_upstream=${api_upstream} observation_dir=${observation_dir}"
operator_proof_command="just gitops-beta-operator-proof output_dir=${proof_dir} draft_file=${draft_file} summary_file=${summary_file} admission_file=${admission_file} edge_bundle=${edge_bundle} deploy_bin=${deploy_bin}"
operator_proof_packet_command="just gitops-beta-operator-proof-packet proof_file=${proof_file} proof_dir=${proof_dir} max_age_seconds=${max_age_seconds} draft_file=${draft_file} summary_file=${summary_file} admission_file=${admission_file} edge_bundle=${edge_bundle} deploy_bin=${deploy_bin} api_upstream=${api_upstream} observation_dir=${observation_dir}"

status="missing_activation_draft"
if bash scripts/recipes/gitops-beta-activation-draft-packet.sh \
  "$draft_file" \
  "$summary_file" \
  "$admission_file" \
  "$proof_dir" \
  "$edge_bundle" \
  "$deploy_bin" \
  "$api_upstream" \
  "$observation_dir" >"${tmp_dir}/activation-packet.out"; then
  if grep -F "activation_draft_packet_status=ready" "${tmp_dir}/activation-packet.out" >/dev/null; then
    status="missing_operator_proof"
  fi
else
  cat "${tmp_dir}/activation-packet.out" >&2 || true
  exit 2
fi

if [[ "$status" == "missing_operator_proof" ]]; then
  selected_proof="$proof_file"
  if [[ -z "$selected_proof" ]]; then
    selected_proof="$(latest_beta_operator_proof "$proof_dir")"
  fi
  if [[ -n "$selected_proof" ]]; then
    if ! bash scripts/recipes/gitops-check-beta-operator-proof.sh \
      "$selected_proof" \
      "$max_age_seconds" \
      "$proof_dir" >"${tmp_dir}/check-proof.out" 2>"${tmp_dir}/check-proof.err"; then
      cat "${tmp_dir}/check-proof.out" >&2 || true
      cat "${tmp_dir}/check-proof.err" >&2 || true
      exit 2
    fi
    if ! jq -e \
      --arg draft_file "$draft_file" \
      --arg summary_file "$summary_file" \
      --arg admission_file "$admission_file" \
      '.inputs.draft_file == $draft_file
      and .inputs.summary_file == $summary_file
      and .inputs.admission_file == $admission_file' \
      "$selected_proof" >/dev/null; then
      echo "beta operator proof does not match selected activation tuple" >&2
      exit 2
    fi
    proof_file="$selected_proof"
    proof_sha256="$(awk -F= '$1 == "gitops_beta_operator_proof_sha256" { print $2; exit }' "${tmp_dir}/check-proof.out")"
    status="ready"
  fi
fi

printf 'gitops_beta_operator_proof_packet_ok=true\n'
printf 'operator_proof_packet_status=%s\n' "$status"
printf 'operator_proof_packet_summary_file=%s\n' "$summary_file"
printf 'operator_proof_packet_summary_sha256=%s\n' "$handoff_summary_sha256"
printf 'operator_proof_packet_admission_file=%s\n' "$admission_file"
printf 'operator_proof_packet_draft_file=%s\n' "$draft_file"
printf 'operator_proof_packet_proof_dir=%s\n' "$proof_dir"
printf 'operator_proof_packet_proof_file=%s\n' "$proof_file"
printf 'operator_proof_packet_edge_bundle=%s\n' "$edge_bundle"
printf 'operator_proof_packet_deploy_bin=%s\n' "$deploy_bin"
printf 'operator_proof_packet_api_upstream=%s\n' "$api_upstream"
printf 'operator_proof_packet_release_id=%s\n' "$active_release_id"
case "$status" in
  missing_activation_draft)
    printf 'operator_proof_packet_next_command_01=%s\n' "$activation_packet_command"
    printf 'operator_proof_packet_after_success_command=%s\n' "$operator_proof_packet_command"
    ;;
  missing_operator_proof)
    printf 'operator_proof_packet_next_command_01=%s\n' "$operator_proof_command"
    ;;
  ready)
    printf 'operator_proof_packet_proof_sha256=%s\n' "$proof_sha256"
    printf 'operator_proof_packet_next_command_01=FISHYSTUFF_GITOPS_ENABLE_BETA_APPLY=1 FISHYSTUFF_GITOPS_ENABLE_LOCAL_APPLY=1 FISHYSTUFF_GITOPS_BETA_APPLY_OPERATOR_PROOF_SHA256=%s just gitops-beta-apply-activation-draft draft_file=%s summary_file=%s admission_file=%s deploy_bin=%s proof_file=%s proof_max_age_seconds=%s\n' \
      "$proof_sha256" \
      "$draft_file" \
      "$summary_file" \
      "$admission_file" \
      "$deploy_bin" \
      "$proof_file" \
      "$max_age_seconds"
    ;;
esac
printf 'remote_deploy_performed=false\n'
printf 'infrastructure_mutation_performed=false\n'
printf 'local_host_mutation_performed=false\n'
