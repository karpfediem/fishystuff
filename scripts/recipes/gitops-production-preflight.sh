#!/usr/bin/env bash
set -euo pipefail

SCRIPT_PATH="$(readlink -f "${BASH_SOURCE[0]}")"
SCRIPT_DIR="$(cd "$(dirname "$SCRIPT_PATH")" && pwd)"
source "${SCRIPT_DIR}/lib/common.sh"

draft_file="$(normalize_named_arg draft_file "${1-data/gitops/production-activation.draft.desired.json}")"
summary_file="$(normalize_named_arg summary_file "${2-data/gitops/production-current.handoff-summary.json}")"
admission_file="$(normalize_named_arg admission_file "${3-}")"
edge_bundle="$(normalize_named_arg edge_bundle "${4-auto}")"
deploy_bin="$(normalize_named_arg deploy_bin "${5-auto}")"
run_helper_tests="$(normalize_named_arg run_helper_tests "${6-true}")"
served_state_dir="$(normalize_named_arg served_state_dir "${7-}")"
rollback_set_path="$(normalize_named_arg rollback_set_path "${8-}")"

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

bool_arg() {
  local name="$1"
  local value="$2"
  case "$value" in
    true | yes | 1)
      printf 'true'
      ;;
    false | no | 0)
      printf 'false'
      ;;
    *)
      echo "${name} must be true or false, got: ${value}" >&2
      exit 2
      ;;
  esac
}

run_step() {
  local step="$1"
  shift
  local stdout="${tmp_dir}/${step}.stdout"
  local stderr="${tmp_dir}/${step}.stderr"

  printf 'gitops_production_preflight_step_start=%s\n' "$step" >&2
  if "$@" >"$stdout" 2>"$stderr"; then
    if [[ -s "$stderr" ]]; then
      sed "s/^/[${step}] /" "$stderr" >&2
    fi
    printf 'gitops_production_preflight_step_pass=%s\n' "$step" >&2
    return
  fi

  printf 'gitops_production_preflight_step_fail=%s\n' "$step" >&2
  if [[ -s "$stdout" ]]; then
    sed "s/^/[${step}:stdout] /" "$stdout" >&2
  fi
  if [[ -s "$stderr" ]]; then
    sed "s/^/[${step}:stderr] /" "$stderr" >&2
  fi
  exit 1
}

compare_served_retained_releases() {
  local served_retained_json="$1"

  if ! jq -e \
    --slurpfile served_retained "$served_retained_json" \
    '
      def normalize_handoff:
        (.retained_releases // [])
        | map({
            release_id,
            generation,
            git_rev,
            dolt_commit,
            api_closure: .closures.api,
            site_closure: .closures.site,
            cdn_runtime_closure: .closures.cdn_runtime,
            dolt_service_closure: .closures.dolt_service,
            dolt_materialization: .dolt.materialization,
            dolt_cache_dir: (.dolt.cache_dir // ""),
            dolt_release_ref: (.dolt.release_ref // "")
          });
      def normalize_served:
        .
        | map({
            release_id,
            generation,
            git_rev,
            dolt_commit,
            api_closure,
            site_closure,
            cdn_runtime_closure,
            dolt_service_closure,
            dolt_materialization,
            dolt_cache_dir: (.dolt_cache_dir // ""),
            dolt_release_ref: (.dolt_release_ref // "")
          });
      normalize_handoff == ($served_retained[0] | normalize_served)
    ' "$summary_file" >/dev/null; then
    echo "served rollback-set retained releases do not match the handoff summary" >&2
    echo "served retained JSON: ${served_retained_json}" >&2
    echo "handoff summary:      ${summary_file}" >&2
    exit 2
  fi
}

require_command jq
require_command sha256sum
require_command sed

draft_file="$(absolute_path "$draft_file")"
summary_file="$(absolute_path "$summary_file")"
if [[ -z "$admission_file" ]]; then
  admission_file="${FISHYSTUFF_GITOPS_ADMISSION_EVIDENCE_FILE:-}"
fi
if [[ -z "$admission_file" ]]; then
  echo "gitops-production-preflight requires admission_file or FISHYSTUFF_GITOPS_ADMISSION_EVIDENCE_FILE" >&2
  exit 2
fi
admission_file="$(absolute_path "$admission_file")"
run_helper_tests="$(bool_arg run_helper_tests "$run_helper_tests")"
if [[ -n "$served_state_dir" ]]; then
  served_state_dir="$(absolute_path "$served_state_dir")"
fi
if [[ -n "$rollback_set_path" ]]; then
  rollback_set_path="$(absolute_path "$rollback_set_path")"
fi
if [[ -z "$served_state_dir" && -n "$rollback_set_path" ]]; then
  served_state_dir="/var/lib/fishystuff/gitops"
fi

tmp_dir="$(mktemp -d)"
cleanup() {
  rm -rf "$tmp_dir"
}
trap cleanup EXIT

run_step handoff_summary \
  bash scripts/recipes/gitops-check-handoff-summary.sh "$summary_file"
run_step activation_draft \
  bash scripts/recipes/gitops-check-activation-draft.sh "$draft_file" "$summary_file" "$admission_file" "$deploy_bin"
run_step activation_review \
  bash scripts/recipes/gitops-review-activation-draft.sh "$draft_file" "$summary_file" "$admission_file" "$deploy_bin"
run_step edge_handoff_bundle \
  bash scripts/recipes/gitops-check-edge-handoff-bundle.sh "$edge_bundle"
run_step host_handoff_plan \
  bash scripts/recipes/gitops-production-host-handoff-plan.sh "$draft_file" "$summary_file" "$admission_file" "$edge_bundle" "$deploy_bin"

served_rollback_checked="false"
served_rollback_set_path=""
if [[ -n "$served_state_dir" ]]; then
  if [[ -n "$rollback_set_path" ]]; then
    served_rollback_set_path="$rollback_set_path"
  else
    served_rollback_set_path="${served_state_dir%/}/rollback-set/production.json"
  fi
  run_step served_rollback_set \
    bash scripts/recipes/gitops-retained-releases-json.sh "$deploy_bin" production "$served_state_dir" "$served_rollback_set_path"
  compare_served_retained_releases "${tmp_dir}/served_rollback_set.stdout"
  served_rollback_checked="true"
fi

if [[ "$run_helper_tests" == "true" ]]; then
  require_command cargo
  run_step helper_cargo_test cargo test -p fishystuff_deploy
  run_step helper_current_handoff bash scripts/recipes/gitops-production-current-handoff-test.sh
  run_step helper_edge_bundle bash scripts/recipes/gitops-production-edge-handoff-bundle-test.sh
  run_step helper_host_handoff_plan bash scripts/recipes/gitops-production-host-handoff-plan-test.sh
fi

state_file="$(jq -er '.desired_state_path | select(type == "string" and length > 0)' "$summary_file")"
read -r draft_sha256 _ < <(sha256sum "$draft_file")
read -r summary_sha256 _ < <(sha256sum "$summary_file")
read -r admission_sha256 _ < <(sha256sum "$admission_file")
edge_bundle_path="$(awk -F= '$1 == "gitops_edge_handoff_bundle_ok" { print $2 }' "${tmp_dir}/edge_handoff_bundle.stdout")"
require_value "$edge_bundle_path" "edge handoff bundle check did not report a bundle path"
release_id="$(jq -er '.environments.production.active_release | select(type == "string" and length > 0)' "$draft_file")"
release_identity="$(jq -er '.release_identity | select(type == "string" and length > 0)' "$admission_file")"
api_upstream="$(jq -er '.environments.production.api_upstream | select(type == "string" and length > 0)' "$draft_file")"

printf 'gitops_production_preflight_ok=%s\n' "$draft_file"
printf 'activation_draft_sha256=%s\n' "$draft_sha256"
printf 'handoff_summary=%s\n' "$summary_file"
printf 'handoff_summary_sha256=%s\n' "$summary_sha256"
printf 'handoff_desired_state=%s\n' "$state_file"
printf 'admission_evidence=%s\n' "$admission_file"
printf 'admission_evidence_sha256=%s\n' "$admission_sha256"
printf 'edge_bundle=%s\n' "$edge_bundle_path"
printf 'environment=production\n'
printf 'release_id=%s\n' "$release_id"
printf 'release_identity=%s\n' "$release_identity"
printf 'api_upstream=%s\n' "$api_upstream"
printf 'helper_regressions_run=%s\n' "$run_helper_tests"
printf 'served_rollback_set_checked=%s\n' "$served_rollback_checked"
if [[ "$served_rollback_checked" == "true" ]]; then
  printf 'served_state_dir=%s\n' "$served_state_dir"
  printf 'served_rollback_set=%s\n' "$served_rollback_set_path"
fi
printf 'remote_deploy_performed=false\n'
printf 'infrastructure_mutation_performed=false\n'
printf 'host_handoff_plan_begin\n'
cat "${tmp_dir}/host_handoff_plan.stdout"
printf 'host_handoff_plan_end\n'
