#!/usr/bin/env bash
set -euo pipefail

SCRIPT_PATH="$(readlink -f "${BASH_SOURCE[0]}")"
SCRIPT_DIR="$(cd "$(dirname "$SCRIPT_PATH")" && pwd)"
source "${SCRIPT_DIR}/lib/common.sh"

admission_file="$(normalize_named_arg admission_file "${1-data/gitops/beta-admission.evidence.json}")"
summary_file="$(normalize_named_arg summary_file "${2-data/gitops/beta-current.handoff-summary.json}")"
api_upstream="$(normalize_named_arg api_upstream "${3-http://127.0.0.1:18192}")"
observation_dir="$(normalize_named_arg observation_dir "${4-data/gitops/beta-admission-observations}")"
draft_file="$(normalize_named_arg draft_file "${5-data/gitops/beta-activation.draft.desired.json}")"

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

require_command jq
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

admission_file="$(absolute_path "$admission_file")"
summary_file="$(absolute_path "$summary_file")"
observation_dir="$(absolute_path "$observation_dir")"
draft_file="$(absolute_path "$draft_file")"

bash scripts/recipes/gitops-check-handoff-summary.sh "$summary_file" >/dev/null 2>&1
environment="$(jq -er '.environment.name | select(type == "string" and length > 0)' "$summary_file")"
if [[ "$environment" != "beta" ]]; then
  echo "gitops-beta-admission-packet requires a beta handoff summary, got: ${environment}" >&2
  exit 2
fi

read -r handoff_summary_sha256 _ < <(sha256sum "$summary_file")
active_release_id="$(jq -er '.environment.active_release | select(type == "string" and length > 0)' "$summary_file")"
dolt_commit="$(jq -er '.active_release.dolt_commit | select(type == "string" and length > 0)' "$summary_file")"

status="missing"
if [[ -f "$admission_file" ]]; then
  if ! jq -e \
    --arg handoff_summary_sha256 "$handoff_summary_sha256" \
    --arg release_id "$active_release_id" \
    --arg dolt_commit "$dolt_commit" \
    --arg api_upstream "$api_upstream" \
    '.schema == "fishystuff.gitops.activation-admission.v1"
    and .environment == "beta"
    and .handoff_summary_sha256 == $handoff_summary_sha256
    and .release_id == $release_id
    and .dolt_commit == $dolt_commit
    and .api_upstream == $api_upstream
    and (.api_meta.url == ($api_upstream + "/api/v1/meta"))
    and (.db_backed_probe.name | type == "string" and length > 0)
    and .db_backed_probe.passed == true
    and (.site_cdn_probe.name | type == "string" and length > 0)
    and .site_cdn_probe.passed == true' \
    "$admission_file" >/dev/null; then
    echo "beta admission evidence does not match the checked beta handoff summary or API upstream" >&2
    exit 2
  fi
  status="ready"
fi

observe_command="just gitops-beta-observe-admission output=${admission_file} summary_file=${summary_file} api_upstream=${api_upstream} observation_dir=${observation_dir}"
activation_draft_command="just gitops-beta-activation-draft output=${draft_file} summary_file=${summary_file} admission_file=${admission_file}"

printf 'gitops_beta_admission_packet_ok=true\n'
printf 'admission_packet_status=%s\n' "$status"
printf 'admission_packet_summary_file=%s\n' "$summary_file"
printf 'admission_packet_summary_sha256=%s\n' "$handoff_summary_sha256"
printf 'admission_packet_admission_file=%s\n' "$admission_file"
printf 'admission_packet_observation_dir=%s\n' "$observation_dir"
printf 'admission_packet_api_upstream=%s\n' "$api_upstream"
printf 'admission_packet_release_id=%s\n' "$active_release_id"
printf 'admission_packet_dolt_commit=%s\n' "$dolt_commit"
if [[ "$status" == "ready" ]]; then
  printf 'admission_packet_db_probe=%s\n' "$(jq -er '.db_backed_probe.name' "$admission_file")"
  printf 'admission_packet_site_cdn_probe=%s\n' "$(jq -er '.site_cdn_probe.name' "$admission_file")"
  printf 'admission_packet_next_command_01=%s\n' "$activation_draft_command"
else
  printf 'admission_packet_next_command_01=%s\n' "$observe_command"
  printf 'admission_packet_after_success_command=%s\n' "$activation_draft_command"
fi
printf 'remote_deploy_performed=false\n'
printf 'infrastructure_mutation_performed=false\n'
printf 'local_host_mutation_performed=false\n'
