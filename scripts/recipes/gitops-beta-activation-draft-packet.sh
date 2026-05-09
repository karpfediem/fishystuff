#!/usr/bin/env bash
set -euo pipefail

SCRIPT_PATH="$(readlink -f "${BASH_SOURCE[0]}")"
SCRIPT_DIR="$(cd "$(dirname "$SCRIPT_PATH")" && pwd)"
source "${SCRIPT_DIR}/lib/common.sh"

draft_file="$(normalize_named_arg draft_file "${1-data/gitops/beta-activation.draft.desired.json}")"
summary_file="$(normalize_named_arg summary_file "${2-data/gitops/beta-current.handoff-summary.json}")"
admission_file="$(normalize_named_arg admission_file "${3-data/gitops/beta-admission.evidence.json}")"
proof_dir="$(normalize_named_arg proof_dir "${4-data/gitops}")"
edge_bundle="$(normalize_named_arg edge_bundle "${5-auto}")"
deploy_bin="$(normalize_named_arg deploy_bin "${6-auto}")"
api_upstream="$(normalize_named_arg api_upstream "${7-http://127.0.0.1:18192}")"
observation_dir="$(normalize_named_arg observation_dir "${8-data/gitops/beta-admission-observations}")"

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

absolute_path_or_auto() {
  local path="$1"
  if [[ "$path" == "auto" ]]; then
    printf '%s' "$path"
    return
  fi
  absolute_path "$path"
}

require_admission_ready() {
  local packet_output="$1"

  if ! bash scripts/recipes/gitops-beta-admission-packet.sh \
    "$admission_file" \
    "$summary_file" \
    "$api_upstream" \
    "$observation_dir" \
    "$draft_file" >"$packet_output"; then
    cat "$packet_output" >&2 || true
    exit 2
  fi
  if ! grep -F "admission_packet_status=ready" "$packet_output" >/dev/null; then
    echo "beta admission packet did not report ready evidence" >&2
    cat "$packet_output" >&2 || true
    exit 2
  fi
}

require_command grep
require_command jq
require_command mktemp
require_command sha256sum

if [[ "$api_upstream" == */ ]]; then
  echo "api_upstream must not end with /" >&2
  exit 2
fi
if [[ "$api_upstream" =~ ^[A-Za-z][A-Za-z0-9+.-]*://[^/?#]*@ ]]; then
  echo "api_upstream must not contain embedded credentials" >&2
  exit 2
fi
require_loopback_http_url api_upstream "$api_upstream"

draft_file="$(absolute_path "$draft_file")"
summary_file="$(absolute_path "$summary_file")"
admission_file="$(absolute_path "$admission_file")"
proof_dir="$(absolute_path "$proof_dir")"
edge_bundle="$(absolute_path_or_auto "$edge_bundle")"
observation_dir="$(absolute_path "$observation_dir")"

bash scripts/recipes/gitops-check-handoff-summary.sh "$summary_file" >/dev/null 2>&1
environment="$(jq -er '.environment.name | select(type == "string" and length > 0)' "$summary_file")"
if [[ "$environment" != "beta" ]]; then
  echo "gitops-beta-activation-draft-packet requires a beta handoff summary, got: ${environment}" >&2
  exit 2
fi

read -r handoff_summary_sha256 _ < <(sha256sum "$summary_file")
active_release_id="$(jq -er '.environment.active_release | select(type == "string" and length > 0)' "$summary_file")"

tmp_dir="$(mktemp -d)"
cleanup() {
  rm -rf "$tmp_dir"
}
trap cleanup EXIT

admission_packet_command="just gitops-beta-admission-packet admission_file=${admission_file} summary_file=${summary_file} api_upstream=${api_upstream} observation_dir=${observation_dir} draft_file=${draft_file}"
activation_draft_command="just gitops-beta-activation-draft output=${draft_file} summary_file=${summary_file} admission_file=${admission_file} deploy_bin=${deploy_bin}"
operator_proof_command="just gitops-beta-operator-proof output_dir=${proof_dir} draft_file=${draft_file} summary_file=${summary_file} admission_file=${admission_file} edge_bundle=${edge_bundle} deploy_bin=${deploy_bin}"

status="missing_admission"
if [[ -f "$admission_file" ]]; then
  require_admission_ready "${tmp_dir}/admission-packet.out"
  status="missing_draft"
fi

if [[ "$status" == "missing_draft" && -f "$draft_file" ]]; then
  if ! bash scripts/recipes/gitops-check-activation-draft.sh \
    "$draft_file" \
    "$summary_file" \
    "$admission_file" \
    "$deploy_bin" >"${tmp_dir}/check-activation.stdout" 2>"${tmp_dir}/check-activation.stderr"; then
    cat "${tmp_dir}/check-activation.stdout" >&2 || true
    cat "${tmp_dir}/check-activation.stderr" >&2 || true
    exit 2
  fi
  status="ready"
fi

printf 'gitops_beta_activation_draft_packet_ok=true\n'
printf 'activation_draft_packet_status=%s\n' "$status"
printf 'activation_draft_packet_summary_file=%s\n' "$summary_file"
printf 'activation_draft_packet_summary_sha256=%s\n' "$handoff_summary_sha256"
printf 'activation_draft_packet_admission_file=%s\n' "$admission_file"
printf 'activation_draft_packet_draft_file=%s\n' "$draft_file"
printf 'activation_draft_packet_proof_dir=%s\n' "$proof_dir"
printf 'activation_draft_packet_edge_bundle=%s\n' "$edge_bundle"
printf 'activation_draft_packet_deploy_bin=%s\n' "$deploy_bin"
printf 'activation_draft_packet_api_upstream=%s\n' "$api_upstream"
printf 'activation_draft_packet_release_id=%s\n' "$active_release_id"
case "$status" in
  missing_admission)
    printf 'activation_draft_packet_next_command_01=%s\n' "$admission_packet_command"
    printf 'activation_draft_packet_after_success_command=%s\n' "$activation_draft_command"
    ;;
  missing_draft)
    printf 'activation_draft_packet_next_command_01=%s\n' "$activation_draft_command"
    printf 'activation_draft_packet_after_success_command=%s\n' "$operator_proof_command"
    ;;
  ready)
    printf 'activation_draft_packet_next_command_01=%s\n' "$operator_proof_command"
    ;;
esac
printf 'remote_deploy_performed=false\n'
printf 'infrastructure_mutation_performed=false\n'
printf 'local_host_mutation_performed=false\n'
