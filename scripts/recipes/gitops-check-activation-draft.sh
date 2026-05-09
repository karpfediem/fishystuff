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
  echo "gitops-check-activation-draft requires admission_file or FISHYSTUFF_GITOPS_ADMISSION_EVIDENCE_FILE" >&2
  exit 2
fi
if [[ "$admission_file" != /* ]]; then
  admission_file="${RECIPE_REPO_ROOT}/${admission_file}"
fi
if [[ ! -f "$draft_file" ]]; then
  echo "activation draft does not exist: ${draft_file}" >&2
  exit 2
fi
if [[ ! -f "$admission_file" ]]; then
  echo "admission evidence does not exist: ${admission_file}" >&2
  exit 2
fi

bash scripts/recipes/gitops-check-handoff-summary.sh "$summary_file"

state_file="$(jq -er '.desired_state_path | select(type == "string" and length > 0)' "$summary_file")"
desired_state_sha256="$(jq -er '.desired_state_sha256 | select(type == "string" and test("^[0-9a-f]{64}$"))' "$summary_file")"
read -r handoff_summary_sha256 _ < <(sha256sum "$summary_file")
environment="$(jq -er '.environment.name | select(type == "string" and length > 0)' "$summary_file")"
active_release_id="$(jq -er '.environment.active_release | select(type == "string" and length > 0)' "$summary_file")"
dolt_commit="$(jq -er '.active_release.dolt_commit | select(type == "string" and length > 0)' "$summary_file")"
release_identity="$(
  jq -er \
    --arg release_id "$active_release_id" \
    '(.releases[$release_id] // error("active release is missing")) as $release
    | "release=\($release_id);generation=\($release.generation);git_rev=\($release.git_rev);dolt_commit=\($release.dolt_commit);dolt_repository=\($release.dolt.repository);dolt_branch_context=\($release.dolt.branch_context);dolt_mode=\($release.dolt.mode);api=\($release.closures.api.store_path);site=\($release.closures.site.store_path);cdn_runtime=\($release.closures.cdn_runtime.store_path);dolt_service=\($release.closures.dolt_service.store_path)"' \
    "$state_file"
)"

if ! jq -e \
  --arg environment "$environment" \
  --arg handoff_summary_sha256 "$handoff_summary_sha256" \
  --arg desired_state_sha256 "$desired_state_sha256" \
  --arg release_id "$active_release_id" \
  --arg release_identity "$release_identity" \
  --arg dolt_commit "$dolt_commit" \
  '
    .schema == "fishystuff.gitops.activation-admission.v1"
    and .environment == $environment
    and .handoff_summary_sha256 == $handoff_summary_sha256
    and .desired_state_sha256 == $desired_state_sha256
    and .release_id == $release_id
    and .release_identity == $release_identity
    and .dolt_commit == $dolt_commit
    and (.api_upstream | type == "string" and length > 0)
    and (.api_upstream | test("^[A-Za-z][A-Za-z0-9+.-]*://"))
    and (.api_upstream | test("^[A-Za-z][A-Za-z0-9+.-]*://[^/?#]*@") | not)
    and (.api_meta.url | type == "string" and length > 0)
    and .api_meta.observed_status == 200
    and .api_meta.release_id == $release_id
    and .api_meta.release_identity == $release_identity
    and .api_meta.dolt_commit == $dolt_commit
    and (.db_backed_probe.name | type == "string" and length > 0)
    and .db_backed_probe.passed == true
    and (.site_cdn_probe.name | type == "string" and length > 0)
    and .site_cdn_probe.passed == true
  ' "$admission_file" >/dev/null; then
  echo "activation admission evidence does not match verified ${environment} handoff" >&2
  exit 2
fi

api_upstream="$(jq -er '.api_upstream' "$admission_file")"
admission_url="$(jq -er '.api_meta.url' "$admission_file")"
timeout_ms="$(jq -er '.api_meta.timeout_ms // 2000' "$admission_file")"
require_loopback_http_url "activation admission api_upstream" "$api_upstream"

if ! jq -e \
  --slurpfile handoff_state "$state_file" \
  --arg environment "$environment" \
  --arg active_release_id "$active_release_id" \
  --arg api_upstream "$api_upstream" \
  --arg admission_url "$admission_url" \
  --argjson timeout_ms "$timeout_ms" \
  '
    $handoff_state[0] as $handoff
    | .cluster == $handoff.cluster
    and .mode == "local-apply"
    and (.generation > $handoff.generation)
    and .hosts == $handoff.hosts
    and .releases == $handoff.releases
    and .environments[$environment].enabled == true
    and .environments[$environment].strategy == $handoff.environments[$environment].strategy
    and .environments[$environment].host == $handoff.environments[$environment].host
    and .environments[$environment].active_release == $active_release_id
    and .environments[$environment].retained_releases == $handoff.environments[$environment].retained_releases
    and .environments[$environment].serve == true
    and .environments[$environment].api_upstream == $api_upstream
    and .environments[$environment].admission_probe.kind == "api_meta"
    and .environments[$environment].admission_probe.probe_name == "api-meta"
    and .environments[$environment].admission_probe.url == $admission_url
    and .environments[$environment].admission_probe.expected_status == 200
    and .environments[$environment].admission_probe.timeout_ms == $timeout_ms
    and .environments[$environment].transition.kind == "activate"
    and .environments[$environment].transition.from_release == ""
  ' "$draft_file" >/dev/null; then
  echo "activation draft does not match verified handoff and admission evidence" >&2
  exit 2
fi

bash scripts/recipes/gitops-check-desired-serving.sh "$deploy_bin" "$draft_file" "$environment"

printf 'gitops_activation_draft_ok=%s\n' "$draft_file" >&2
