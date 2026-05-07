#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "${SCRIPT_DIR}/lib/common.sh"

summary_file="$(normalize_named_arg summary_file "${1-data/gitops/production-current.handoff-summary.json}")"
state_file="$(normalize_named_arg state_file "${2-}")"

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

if [[ "$summary_file" != /* ]]; then
  summary_file="${RECIPE_REPO_ROOT}/${summary_file}"
fi
if [[ ! -f "$summary_file" ]]; then
  echo "handoff summary does not exist: ${summary_file}" >&2
  exit 2
fi

summary_state_path="$(jq -er '.desired_state_path | select(type == "string" and length > 0)' "$summary_file")"
if [[ -z "$state_file" ]]; then
  state_file="$summary_state_path"
elif [[ "$state_file" != /* ]]; then
  state_file="${RECIPE_REPO_ROOT}/${state_file}"
fi

if [[ "$state_file" != "$summary_state_path" ]]; then
  echo "handoff summary desired_state_path does not match checked state file" >&2
  echo "summary: ${summary_state_path}" >&2
  echo "checked: ${state_file}" >&2
  exit 2
fi
if [[ ! -f "$state_file" ]]; then
  echo "desired-state file does not exist: ${state_file}" >&2
  exit 2
fi

expected_sha256="$(jq -er '.desired_state_sha256 | select(type == "string" and test("^[0-9a-f]{64}$"))' "$summary_file")"
read -r actual_sha256 _ < <(sha256sum "$state_file")
if [[ "$actual_sha256" != "$expected_sha256" ]]; then
  echo "handoff summary desired_state_sha256 does not match checked state file" >&2
  echo "summary: ${expected_sha256}" >&2
  echo "actual:  ${actual_sha256}" >&2
  exit 2
fi

if ! jq -e '
    .schema == "fishystuff.gitops.production-current-handoff.v1"
    and .checks.production_current_desired_generated == true
    and .checks.desired_serving_preflight_passed == true
    and .checks.closure_paths_verified == true
    and .checks.cdn_retained_roots_verified == true
    and .checks.gitops_unify_passed == true
    and .checks.remote_deploy_performed == false
    and .checks.infrastructure_mutation_performed == false
  ' "$summary_file" >/dev/null; then
  echo "handoff summary does not record the required completed local checks" >&2
  exit 2
fi

jq -r \
  '(.active_release.release_id // "active") as $active_release_id
  | (
      (.active_release.closures | to_entries[] | [$active_release_id, .key, .value]),
      (.retained_releases[]? as $release | $release.closures | to_entries[] | [$release.release_id, .key, .value])
    )
  | @tsv' \
  "$summary_file" |
  while IFS=$'\t' read -r release_id closure_name store_path; do
    if [[ -z "$store_path" || ! -e "$store_path" ]]; then
      echo "handoff summary closure path does not exist for ${release_id} ${closure_name}: ${store_path}" >&2
      exit 2
    fi
  done

active_manifest="$(jq -er '.cdn_retention.active_manifest | select(type == "string" and length > 0)' "$summary_file")"
if [[ ! -f "$active_manifest" ]]; then
  echo "active CDN manifest recorded by handoff summary does not exist: ${active_manifest}" >&2
  exit 2
fi

active_current_root="$(jq -er '.current_root | select(type == "string" and length > 0)' "$active_manifest")"
active_retained_roots="$(jq -ce '.retained_roots | if type == "array" then . else error("retained_roots must be an array") end' "$active_manifest")"
active_retained_count="$(jq -er '.retained_roots | length' "$active_manifest")"
declared_retained_count="$(jq -er '.retained_root_count | select(type == "number")' "$active_manifest")"
if [[ "$active_retained_count" != "$declared_retained_count" ]]; then
  echo "active CDN manifest retained_root_count does not match retained_roots length: ${active_manifest}" >&2
  exit 2
fi

if ! jq -e \
    --arg active_current_root "$active_current_root" \
    --argjson active_retained_roots "$active_retained_roots" \
    '.cdn_retention.active_current_root == $active_current_root
    and .cdn_retention.active_retained_roots == $active_retained_roots
    and ([.cdn_retention.retained_releases[]? | .retained_by_active_cdn_serving_root] | all(. == true))' \
    "$summary_file" >/dev/null; then
  echo "handoff summary CDN retention data does not match the active CDN manifest" >&2
  exit 2
fi

jq -r '.cdn_retention.retained_releases[]? | [.release_id, .expected_retained_cdn_root] | @tsv' "$summary_file" |
  while IFS=$'\t' read -r release_id expected_retained_cdn_root; do
    if ! jq -e --arg root "$expected_retained_cdn_root" '.retained_roots | index($root) != null' "$active_manifest" >/dev/null; then
      echo "active CDN manifest no longer retains ${release_id} root ${expected_retained_cdn_root}" >&2
      exit 2
    fi
  done

printf 'gitops_handoff_summary_ok=%s\n' "$summary_file" >&2
