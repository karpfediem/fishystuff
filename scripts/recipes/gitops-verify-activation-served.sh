#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "${SCRIPT_DIR}/lib/common.sh"

draft_file="$(normalize_named_arg draft_file "${1-data/gitops/production-activation.draft.desired.json}")"
summary_file="$(normalize_named_arg summary_file "${2-data/gitops/production-current.handoff-summary.json}")"
admission_file="$(normalize_named_arg admission_file "${3-}")"
deploy_bin="$(normalize_named_arg deploy_bin "${4-auto}")"
state_dir="$(normalize_named_arg state_dir "${5-/var/lib/fishystuff/gitops}")"
run_dir="$(normalize_named_arg run_dir "${6-/run/fishystuff/gitops}")"

cd "$RECIPE_REPO_ROOT"

require_command() {
  local command_name="$1"
  if ! command -v "$command_name" >/dev/null; then
    echo "missing required command: ${command_name}" >&2
    exit 2
  fi
}

require_command jq

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
  echo "gitops-verify-activation-served requires admission_file or FISHYSTUFF_GITOPS_ADMISSION_EVIDENCE_FILE" >&2
  exit 2
fi
if [[ "$admission_file" != /* ]]; then
  admission_file="${RECIPE_REPO_ROOT}/${admission_file}"
fi

bash scripts/recipes/gitops-check-activation-draft.sh "$draft_file" "$summary_file" "$admission_file" "$deploy_bin"

environment="production"
release_id="$(jq -er '.environments.production.active_release | select(type == "string" and length > 0)' "$draft_file")"
host="$(jq -er '.environments.production.host | select(type == "string" and length > 0)' "$draft_file")"
generation="$(jq -er '.generation | select(type == "number")' "$draft_file")"
api_upstream="$(jq -er '.environments.production.api_upstream | select(type == "string" and length > 0)' "$draft_file")"
admission_url="$(jq -er '.environments.production.admission_probe.url | select(type == "string" and length > 0)' "$draft_file")"

bash scripts/recipes/gitops-inspect-served.sh "$deploy_bin" "$environment" "$state_dir" "$run_dir" "$host" "$release_id"

status_path="${state_dir%/}/status/${environment}.json"
active_path="${state_dir%/}/active/${environment}.json"
admission_path="${run_dir%/}/admission/${environment}.json"
route_path="${run_dir%/}/routes/${environment}.json"

if ! jq -e \
  --argjson generation "$generation" \
  --arg environment "$environment" \
  --arg host "$host" \
  --arg release_id "$release_id" \
  '.desired_generation == $generation
    and .environment == $environment
    and .host == $host
    and .release_id == $release_id
    and .phase == "served"
    and .served == true
    and .admission_state == "passed_fixture"
    and .rollback_available == true
    and (.rollback_retained_count > 0)' \
  "$status_path" >/dev/null; then
  echo "served status does not match activation draft" >&2
  exit 2
fi

if ! jq -e \
  --argjson generation "$generation" \
  --arg environment "$environment" \
  --arg host "$host" \
  --arg release_id "$release_id" \
  --arg api_upstream "$api_upstream" \
  '.desired_generation == $generation
    and .environment == $environment
    and .host == $host
    and .release_id == $release_id
    and .api_upstream == $api_upstream
    and .served == true
    and .admission_state == "passed_fixture"' \
  "$active_path" >/dev/null; then
  echo "active selection does not match activation draft" >&2
  exit 2
fi

if ! jq -e \
  --argjson generation "$generation" \
  --arg environment "$environment" \
  --arg host "$host" \
  --arg release_id "$release_id" \
  --arg api_upstream "$api_upstream" \
  '.desired_generation == $generation
    and .environment == $environment
    and .host == $host
    and .release_id == $release_id
    and .api_upstream == $api_upstream
    and .served == true
    and .state == "selected_local_route"' \
  "$route_path" >/dev/null; then
  echo "route selection does not match activation draft" >&2
  exit 2
fi

if ! jq -e \
  --arg environment "$environment" \
  --arg host "$host" \
  --arg release_id "$release_id" \
  --arg admission_url "$admission_url" \
  '.environment == $environment
    and .host == $host
    and .release_id == $release_id
    and .admission_state == "passed_fixture"
    and .url == $admission_url' \
  "$admission_path" >/dev/null; then
  echo "admission status does not match activation draft" >&2
  exit 2
fi

printf 'gitops_activation_served_ok=%s\n' "$release_id"
printf 'gitops_activation_served_generation=%s\n' "$generation"
printf 'gitops_activation_served_state_dir=%s\n' "$state_dir"
printf 'gitops_activation_served_run_dir=%s\n' "$run_dir"
printf 'remote_deploy_performed=false\n'
printf 'infrastructure_mutation_performed=false\n'
