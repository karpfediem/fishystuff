#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "${SCRIPT_DIR}/lib/common.sh"

output="$(normalize_named_arg output "${1-data/gitops/production-current.desired.json}")"
dolt_ref="$(normalize_named_arg dolt_ref "${2-main}")"
mgmt_bin="$(normalize_named_arg mgmt_bin "${3-auto}")"
deploy_bin="$(normalize_named_arg deploy_bin "${4-auto}")"
summary_output="$(normalize_named_arg summary_output "${5-}")"

cd "$RECIPE_REPO_ROOT"

if [[ "$output" == "-" ]]; then
  echo "gitops-production-current-handoff requires a file output, not '-'" >&2
  exit 2
fi

if [[ -z "${FISHYSTUFF_GITOPS_RETAINED_RELEASES_FILE:-}" && -z "${FISHYSTUFF_GITOPS_RETAINED_RELEASES_JSON:-}" ]]; then
  echo "gitops-production-current-handoff requires FISHYSTUFF_GITOPS_RETAINED_RELEASES_FILE or FISHYSTUFF_GITOPS_RETAINED_RELEASES_JSON" >&2
  echo "derive it with: just gitops-retained-releases-json > /tmp/fishystuff-retained-releases.json" >&2
  exit 2
fi

state_file="$output"
if [[ "$state_file" != /* ]]; then
  state_file="${RECIPE_REPO_ROOT}/${state_file}"
fi

summary_file="$summary_output"
if [[ -z "$summary_file" ]]; then
  summary_file="${state_file%.desired.json}.handoff-summary.json"
  if [[ "$summary_file" == "$state_file" ]]; then
    summary_file="${state_file}.handoff-summary.json"
  fi
elif [[ "$summary_file" != /* ]]; then
  summary_file="${RECIPE_REPO_ROOT}/${summary_file}"
fi

write_handoff_summary() {
  local desired_state="$1"
  local summary="$2"
  local tmp=""

  mkdir -p "$(dirname "$summary")"
  tmp="$(mktemp "$(dirname "$summary")/.${summary##*/}.XXXXXX")"
  jq -S \
    --arg desired_state_path "$desired_state" \
    '(.environments.production // error("production environment is missing")) as $env
    | (.releases[$env.active_release] // error("active release is missing")) as $active
    | {
        schema: "fishystuff.gitops.production-current-handoff.v1",
        desired_state_path: $desired_state_path,
        cluster,
        mode,
        desired_generation: .generation,
        environment: {
          name: "production",
          host: $env.host,
          serve_requested: $env.serve,
          active_release: $env.active_release,
          retained_releases: $env.retained_releases
        },
        active_release: {
          release_id: $env.active_release,
          generation: $active.generation,
          git_rev: $active.git_rev,
          dolt_commit: $active.dolt_commit,
          closures: {
            api: $active.closures.api.store_path,
            site: $active.closures.site.store_path,
            cdn_runtime: $active.closures.cdn_runtime.store_path,
            dolt_service: $active.closures.dolt_service.store_path
          },
          dolt: {
            materialization: $active.dolt.materialization,
            branch_context: $active.dolt.branch_context,
            cache_dir: $active.dolt.cache_dir,
            release_ref: $active.dolt.release_ref
          }
        },
        retained_release_count: ($env.retained_releases | length),
        retained_releases: [
          $env.retained_releases[] as $release_id
          | (.releases[$release_id] // error("retained release " + $release_id + " is missing")) as $release
          | {
              release_id: $release_id,
              generation: $release.generation,
              git_rev: $release.git_rev,
              dolt_commit: $release.dolt_commit,
              closures: {
                api: $release.closures.api.store_path,
                site: $release.closures.site.store_path,
                cdn_runtime: $release.closures.cdn_runtime.store_path,
                dolt_service: $release.closures.dolt_service.store_path
              },
              dolt: {
                materialization: $release.dolt.materialization,
                branch_context: $release.dolt.branch_context,
                cache_dir: $release.dolt.cache_dir,
                release_ref: $release.dolt.release_ref
              }
            }
        ],
        checks: {
          production_current_desired_generated: true,
          desired_serving_preflight_passed: true,
          gitops_unify_passed: true,
          remote_deploy_performed: false,
          infrastructure_mutation_performed: false
        }
      }' "$desired_state" >"$tmp"
  mv "$tmp" "$summary"
}

bash scripts/recipes/gitops-production-current-desired.sh "$output" "$dolt_ref"
bash scripts/recipes/gitops-check-desired-serving.sh "$deploy_bin" "$state_file" production
bash scripts/recipes/gitops-unify.sh "$mgmt_bin" "$state_file"
write_handoff_summary "$state_file" "$summary_file"

printf 'production_current_handoff_ready=%s\n' "$state_file" >&2
printf 'production_current_handoff_summary=%s\n' "$summary_file" >&2
