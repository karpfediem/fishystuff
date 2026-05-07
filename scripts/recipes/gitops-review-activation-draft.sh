#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "${SCRIPT_DIR}/lib/common.sh"

draft_file="$(normalize_named_arg draft_file "${1-data/gitops/production-activation.draft.desired.json}")"
summary_file="$(normalize_named_arg summary_file "${2-data/gitops/production-current.handoff-summary.json}")"
admission_file="$(normalize_named_arg admission_file "${3-}")"
deploy_bin="$(normalize_named_arg deploy_bin "${4-auto}")"

cd "$RECIPE_REPO_ROOT"

require_command() {
  local command_name="$1"
  if ! command -v "$command_name" >/dev/null; then
    echo "missing required command: ${command_name}" >&2
    exit 2
  fi
}

require_command jq
require_command sha256sum

if [[ "$draft_file" != /* ]]; then
  draft_file="${RECIPE_REPO_ROOT}/${draft_file}"
fi
if [[ "$summary_file" != /* ]]; then
  summary_file="${RECIPE_REPO_ROOT}/${summary_file}"
fi
if [[ -z "$admission_file" ]]; then
  admission_file="${FISHYSTUFF_GITOPS_ADMISSION_EVIDENCE_FILE:-}"
fi
if [[ -z "$admission_file" ]]; then
  echo "gitops-review-activation-draft requires admission_file or FISHYSTUFF_GITOPS_ADMISSION_EVIDENCE_FILE" >&2
  exit 2
fi
if [[ "$admission_file" != /* ]]; then
  admission_file="${RECIPE_REPO_ROOT}/${admission_file}"
fi

bash scripts/recipes/gitops-check-activation-draft.sh "$draft_file" "$summary_file" "$admission_file" "$deploy_bin"

state_file="$(jq -er '.desired_state_path | select(type == "string" and length > 0)' "$summary_file")"
read -r summary_sha256 _ < <(sha256sum "$summary_file")
read -r draft_sha256 _ < <(sha256sum "$draft_file")
desired_state_sha256="$(jq -er '.desired_state_sha256' "$summary_file")"
release_id="$(jq -er '.environment.active_release' "$summary_file")"
retained_release_ids="$(jq -cer '.environment.retained_releases' "$summary_file")"
retained_release_count="$(jq -er '.retained_release_count' "$summary_file")"

jq -r \
  --arg draft_file "$draft_file" \
  --arg draft_sha256 "$draft_sha256" \
  --arg summary_file "$summary_file" \
  --arg summary_sha256 "$summary_sha256" \
  --arg state_file "$state_file" \
  --arg desired_state_sha256 "$desired_state_sha256" \
  --arg release_id "$release_id" \
  --arg retained_release_ids "$retained_release_ids" \
  --arg retained_release_count "$retained_release_count" \
  --slurpfile admission "$admission_file" \
  '
    (.releases[$release_id] // error("active release is missing")) as $release
    | ($admission[0]) as $evidence
    | [
        "gitops_activation_review_ok=" + $draft_file,
        "activation_draft_sha256=" + $draft_sha256,
        "handoff_summary=" + $summary_file,
        "handoff_summary_sha256=" + $summary_sha256,
        "handoff_desired_state=" + $state_file,
        "handoff_desired_state_sha256=" + $desired_state_sha256,
        "environment=production",
        "mode=" + .mode,
        "serve=" + (.environments.production.serve | tostring),
        "transition_kind=" + .environments.production.transition.kind,
        "release_id=" + $release_id,
        "release_identity=" + $evidence.release_identity,
        "git_rev=" + $release.git_rev,
        "dolt_commit=" + $release.dolt_commit,
        "api_closure=" + $release.closures.api.store_path,
        "site_closure=" + $release.closures.site.store_path,
        "cdn_runtime_closure=" + $release.closures.cdn_runtime.store_path,
        "dolt_service_closure=" + $release.closures.dolt_service.store_path,
        "api_upstream=" + .environments.production.api_upstream,
        "api_meta_url=" + .environments.production.admission_probe.url,
        "db_backed_probe=" + $evidence.db_backed_probe.name,
        "site_cdn_probe=" + $evidence.site_cdn_probe.name,
        "retained_release_count=" + $retained_release_count,
        "retained_release_ids=" + $retained_release_ids,
        "remote_deploy_performed=false",
        "infrastructure_mutation_performed=false"
      ]
    | .[]' \
  "$draft_file"
