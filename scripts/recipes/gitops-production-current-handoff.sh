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

require_command() {
  local command_name="$1"
  if ! command -v "$command_name" >/dev/null; then
    echo "missing required command: ${command_name}" >&2
    exit 2
  fi
}

require_command jq
require_command sha256sum

environment="${FISHYSTUFF_GITOPS_ENVIRONMENT:-production}"
current_desired_script="${FISHYSTUFF_GITOPS_CURRENT_DESIRED_SCRIPT:-scripts/recipes/gitops-production-current-desired.sh}"
handoff_schema="${FISHYSTUFF_GITOPS_HANDOFF_SCHEMA:-}"
if [[ -z "$handoff_schema" ]]; then
  if [[ "$environment" == "production" ]]; then
    handoff_schema="fishystuff.gitops.production-current-handoff.v1"
  else
    handoff_schema="fishystuff.gitops.current-handoff.v1"
  fi
fi
require_retained_releases="${FISHYSTUFF_GITOPS_HANDOFF_REQUIRE_RETAINED:-}"
if [[ -z "$require_retained_releases" ]]; then
  if [[ "$environment" == "production" ]]; then
    require_retained_releases="true"
  else
    require_retained_releases="false"
  fi
fi
check_desired_serving="${FISHYSTUFF_GITOPS_HANDOFF_CHECK_DESIRED_SERVING:-$require_retained_releases}"

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

require_safe_name() {
  local name="$1"
  local value="$2"
  require_value "$value" "$name must not be empty"
  if [[ ! "$value" =~ ^[A-Za-z0-9._-]+$ ]]; then
    echo "$name contains unsupported characters: $value" >&2
    exit 2
  fi
}

require_safe_name FISHYSTUFF_GITOPS_ENVIRONMENT "$environment"
require_retained_releases="$(bool_arg FISHYSTUFF_GITOPS_HANDOFF_REQUIRE_RETAINED "$require_retained_releases")"
check_desired_serving="$(bool_arg FISHYSTUFF_GITOPS_HANDOFF_CHECK_DESIRED_SERVING "$check_desired_serving")"

if [[ "$output" == "-" ]]; then
  echo "gitops-production-current-handoff requires a file output, not '-'" >&2
  exit 2
fi

if [[ "$require_retained_releases" == "true" && -z "${FISHYSTUFF_GITOPS_RETAINED_RELEASES_FILE:-}" && -z "${FISHYSTUFF_GITOPS_RETAINED_RELEASES_JSON:-}" ]]; then
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
  local cdn_retention="$3"
  local desired_state_sha256=""
  local tmp=""

  read -r desired_state_sha256 _ < <(sha256sum "$desired_state")
  mkdir -p "$(dirname "$summary")"
  tmp="$(mktemp "$(dirname "$summary")/.${summary##*/}.XXXXXX")"
  jq -S \
    --arg desired_state_path "$desired_state" \
    --arg desired_state_sha256 "$desired_state_sha256" \
    --arg environment "$environment" \
    --arg handoff_schema "$handoff_schema" \
    --argjson desired_serving_preflight_passed "$check_desired_serving" \
    --slurpfile cdn_retention "$cdn_retention" \
    '(.environments[$environment] // error($environment + " environment is missing")) as $env
    | (.releases[$env.active_release] // error("active release is missing")) as $active
    | {
        schema: $handoff_schema,
        desired_state_path: $desired_state_path,
        desired_state_sha256: $desired_state_sha256,
        cluster,
        mode,
        desired_generation: .generation,
        environment: {
          name: $environment,
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
        cdn_retention: $cdn_retention[0],
        checks: {
          production_current_desired_generated: true,
          current_desired_generated: true,
          desired_serving_preflight_passed: $desired_serving_preflight_passed,
          desired_serving_preflight_skipped: ($desired_serving_preflight_passed | not),
          closure_paths_verified: true,
          cdn_manifest_verified: true,
          cdn_retained_roots_verified: true,
          gitops_unify_passed: true,
          remote_deploy_performed: false,
          infrastructure_mutation_performed: false
        }
      }' "$desired_state" >"$tmp"
  mv "$tmp" "$summary"
}

verify_desired_closure_paths() {
  local desired_state="$1"

  jq -r \
    --arg environment "$environment" \
    '(.environments[$environment] // error($environment + " environment is missing")) as $env
    | (
        [$env.active_release] + $env.retained_releases
      )[] as $release_id
    | (.releases[$release_id] // error("release " + $release_id + " is missing")) as $release
    | $release.closures
    | to_entries[]
    | [$release_id, .key, .value.store_path]
    | @tsv' \
    "$desired_state" |
    while IFS=$'\t' read -r release_id closure_name store_path; do
      if [[ -z "$store_path" || ! -e "$store_path" ]]; then
        echo "handoff closure path does not exist for ${release_id} ${closure_name}: ${store_path}" >&2
        exit 2
      fi
    done
}

write_cdn_retention_summary() {
  local desired_state="$1"
  local output="$2"
  local active_cdn_runtime=""
  local active_manifest=""
  local active_current_root=""
  local active_retained_roots=""
  local declared_retained_count=""
  local actual_retained_count=""
  local retained_entries=""
  local retained_checks=""

  active_cdn_runtime="$(
    jq -er \
      --arg environment "$environment" \
      '(.environments[$environment] // error($environment + " environment is missing")) as $env
      | (.releases[$env.active_release] // error("active release is missing")) as $active
      | $active.closures.cdn_runtime.store_path
      | select(type == "string" and length > 0)' \
      "$desired_state"
  )"
  active_manifest="${active_cdn_runtime}/cdn-serving-manifest.json"
  if [[ ! -f "$active_manifest" ]]; then
    echo "active ${environment} cdn_runtime does not contain cdn-serving-manifest.json: ${active_cdn_runtime}" >&2
    exit 2
  fi

  active_current_root="$(jq -er '.current_root | select(type == "string" and length > 0)' "$active_manifest")"
  active_retained_roots="$(jq -ce '.retained_roots | if type == "array" then . else error("retained_roots must be an array") end' "$active_manifest")"
  declared_retained_count="$(jq -er '.retained_root_count | select(type == "number")' "$active_manifest")"
  actual_retained_count="$(jq -er '.retained_roots | length' "$active_manifest")"
  if [[ "$declared_retained_count" != "$actual_retained_count" ]]; then
    echo "active CDN serving manifest retained_root_count=${declared_retained_count} but retained_roots has ${actual_retained_count} entries: ${active_manifest}" >&2
    exit 2
  fi

  retained_entries="$(mktemp)"
  jq -r \
    --arg environment "$environment" \
    '(.environments[$environment] // error($environment + " environment is missing")) as $env
    | $env.retained_releases[] as $release_id
    | (.releases[$release_id] // error("retained release " + $release_id + " is missing")) as $release
    | [$release_id, $release.closures.cdn_runtime.store_path]
    | @tsv' \
    "$desired_state" |
    while IFS=$'\t' read -r release_id retained_cdn_runtime; do
      local retained_manifest=""
      local expected_retained_cdn_root=""
      local retained_is_serving_root="false"

      retained_manifest="${retained_cdn_runtime}/cdn-serving-manifest.json"
      if [[ -f "$retained_manifest" ]]; then
        retained_is_serving_root="true"
        expected_retained_cdn_root="$(jq -er '.current_root | select(type == "string" and length > 0)' "$retained_manifest")"
      else
        expected_retained_cdn_root="$retained_cdn_runtime"
      fi

      if ! jq -e --arg root "$expected_retained_cdn_root" '.retained_roots | index($root) != null' "$active_manifest" >/dev/null; then
        echo "active CDN serving root does not retain ${release_id} CDN root ${expected_retained_cdn_root}" >&2
        echo "active manifest: ${active_manifest}" >&2
        exit 2
      fi

      jq -n \
        --arg release_id "$release_id" \
        --arg cdn_runtime "$retained_cdn_runtime" \
        --arg expected_retained_cdn_root "$expected_retained_cdn_root" \
        --argjson retained_is_serving_root "$retained_is_serving_root" \
        '{
          release_id: $release_id,
          cdn_runtime: $cdn_runtime,
          retained_cdn_runtime_is_serving_root: $retained_is_serving_root,
          expected_retained_cdn_root: $expected_retained_cdn_root,
          retained_by_active_cdn_serving_root: true
        }' >>"$retained_entries"
    done

  retained_checks="$(jq -s '.' "$retained_entries")"
  rm -f "$retained_entries"

  jq -n \
    --arg active_cdn_runtime "$active_cdn_runtime" \
    --arg active_manifest "$active_manifest" \
    --arg active_current_root "$active_current_root" \
    --argjson active_retained_roots "$active_retained_roots" \
    --argjson retained_releases "$retained_checks" \
    '{
      active_cdn_runtime: $active_cdn_runtime,
      active_manifest: $active_manifest,
      active_current_root: $active_current_root,
      active_retained_roots: $active_retained_roots,
      retained_releases: $retained_releases
    }' >"$output"
}

cdn_retention_summary="$(mktemp)"
bash "$current_desired_script" "$output" "$dolt_ref"
if [[ "$check_desired_serving" == "true" ]]; then
  bash scripts/recipes/gitops-check-desired-serving.sh "$deploy_bin" "$state_file" "$environment"
fi
verify_desired_closure_paths "$state_file"
write_cdn_retention_summary "$state_file" "$cdn_retention_summary"
bash scripts/recipes/gitops-unify.sh "$mgmt_bin" "$state_file"
write_handoff_summary "$state_file" "$summary_file" "$cdn_retention_summary"
rm -f "$cdn_retention_summary"
bash scripts/recipes/gitops-check-handoff-summary.sh "$summary_file" "$state_file"

printf 'gitops_current_handoff_environment=%s\n' "$environment" >&2
printf 'gitops_current_handoff_ready=%s\n' "$state_file" >&2
printf 'gitops_current_handoff_summary=%s\n' "$summary_file" >&2
if [[ "$environment" == "production" ]]; then
  printf 'production_current_handoff_ready=%s\n' "$state_file" >&2
  printf 'production_current_handoff_summary=%s\n' "$summary_file" >&2
elif [[ "$environment" == "beta" ]]; then
  printf 'beta_current_handoff_ready=%s\n' "$state_file" >&2
  printf 'beta_current_handoff_summary=%s\n' "$summary_file" >&2
fi
