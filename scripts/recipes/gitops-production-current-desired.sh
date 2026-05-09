#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "${SCRIPT_DIR}/lib/common.sh"

output="$(normalize_named_arg output "${1-data/gitops/production-current.desired.json}")"
dolt_ref="$(normalize_named_arg dolt_ref "${2-main}")"

cd "$RECIPE_REPO_ROOT"

require_command() {
  local name="$1"
  if ! command -v "$name" >/dev/null 2>&1; then
    echo "$name is required" >&2
    exit 127
  fi
}

require_positive_int() {
  local name="$1"
  local value="$2"
  if [[ ! "$value" =~ ^[1-9][0-9]*$ ]]; then
    echo "$name must be a positive integer, got: ${value:-<empty>}" >&2
    exit 2
  fi
}

require_store_path() {
  local name="$1"
  local value="$2"
  require_value "$value" "$name must not be empty"
  case "$value" in
    /nix/store/*) ;;
    *)
      echo "$name must be a /nix/store path, got: $value" >&2
      exit 2
      ;;
  esac
}

reject_credential_url() {
  local name="$1"
  local value="$2"
  if [[ "$value" != file://* && "$value" =~ ^[A-Za-z][A-Za-z0-9+.-]*://[^/?#]*@ ]]; then
    echo "$name must not contain embedded credentials" >&2
    exit 2
  fi
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

require_safe_ref_name() {
  local name="$1"
  local value="$2"
  require_value "$value" "$name must not be empty"
  if [[ ! "$value" =~ ^[A-Za-z0-9._/-]+$ ]]; then
    echo "$name contains unsupported characters: $value" >&2
    exit 2
  fi
}

read_retained_releases_source() {
  if [[ -n "${FISHYSTUFF_GITOPS_RETAINED_RELEASES_FILE:-}" ]]; then
    if [[ ! -f "$FISHYSTUFF_GITOPS_RETAINED_RELEASES_FILE" ]]; then
      echo "FISHYSTUFF_GITOPS_RETAINED_RELEASES_FILE does not exist: $FISHYSTUFF_GITOPS_RETAINED_RELEASES_FILE" >&2
      exit 2
    fi
    cat "$FISHYSTUFF_GITOPS_RETAINED_RELEASES_FILE"
    return
  fi
  if [[ -n "${FISHYSTUFF_GITOPS_RETAINED_RELEASES_JSON:-}" ]]; then
    printf '%s\n' "$FISHYSTUFF_GITOPS_RETAINED_RELEASES_JSON"
    return
  fi
  printf '[]\n'
}

build_or_use_path() {
  local env_name="$1"
  local attr="$2"
  local value="${!env_name:-}"
  local nix_args=(build --no-link --print-out-paths)
  if [[ -n "$value" ]]; then
    printf '%s\n' "$value"
    return
  fi
  if [[ "$attr" == "cdn-serving-root" && -n "${FISHYSTUFF_OPERATOR_ROOT:-}" ]]; then
    nix_args+=(--impure)
  fi
  nix "${nix_args[@]}" ".#${attr}" | tail -n 1
}

current_git_rev() {
  local rev=""
  rev="$(git rev-parse HEAD)"
  if ! git diff-index --quiet HEAD --; then
    rev="${rev}-dirty"
  fi
  printf '%s\n' "$rev"
}

current_dolt_commit() {
  local ref="$1"
  local error_file=""
  local output=""
  if [[ ! "$ref" =~ ^[A-Za-z0-9_./-]+$ ]]; then
    echo "dolt_ref contains unsupported characters: $ref" >&2
    exit 2
  fi
  error_file="$(mktemp /tmp/fishystuff-gitops-dolt-log.XXXXXX)"
  if ! output="$(dolt log -n 1 "$ref" --oneline 2>"$error_file")"; then
    echo "could not read local Dolt ref $ref; set FISHYSTUFF_GITOPS_DOLT_COMMIT to an exact commit to bypass local Dolt discovery" >&2
    cat "$error_file" >&2
    rm -f "$error_file"
    exit 2
  fi
  rm -f "$error_file"
  awk '{ print $1 }' <<< "$output"
}

current_dolt_remote_url() {
  local error_file=""
  local remote_output=""
  local url=""
  error_file="$(mktemp /tmp/fishystuff-gitops-dolt-remote.XXXXXX)"
  if ! remote_output="$(dolt remote -v 2>"$error_file")"; then
    echo "could not read local Dolt remotes; set FISHYSTUFF_GITOPS_DOLT_REMOTE_URL to bypass local Dolt discovery" >&2
    cat "$error_file" >&2
    rm -f "$error_file"
    exit 2
  fi
  rm -f "$error_file"
  url="$(awk '$1 == "origin" { print $2; exit }' <<< "$remote_output")"
  if [[ -z "$url" ]]; then
    url="https://doltremoteapi.dolthub.com/fishystuff/fishystuff"
  fi
  printf '%s\n' "$url"
}

require_command jq
require_command sha256sum
require_command git

cluster="${FISHYSTUFF_GITOPS_CLUSTER:-production}"
environment="${FISHYSTUFF_GITOPS_ENVIRONMENT:-production}"
host_key="${FISHYSTUFF_GITOPS_HOST_KEY:-${environment}-single-host}"
hostname="${FISHYSTUFF_GITOPS_HOSTNAME:-$host_key}"
dolt_branch_context="${FISHYSTUFF_GITOPS_DOLT_BRANCH_CONTEXT:-}"
if [[ -z "$dolt_branch_context" ]]; then
  if [[ "$environment" == "production" ]]; then
    dolt_branch_context="main"
  else
    dolt_branch_context="$environment"
  fi
fi
dolt_cache_dir="${FISHYSTUFF_GITOPS_DOLT_CACHE_DIR:-}"
if [[ -z "$dolt_cache_dir" ]]; then
  case "$environment" in
    production)
      dolt_cache_dir="/var/lib/fishystuff/gitops/dolt-cache/fishystuff"
      ;;
    beta)
      dolt_cache_dir="/var/lib/fishystuff/gitops-beta/dolt-cache/fishystuff"
      ;;
    *)
      dolt_cache_dir="/var/lib/fishystuff/gitops-${environment}/dolt-cache/fishystuff"
      ;;
  esac
fi
gcroot_base="${FISHYSTUFF_GITOPS_GCROOT_BASE:-}"
if [[ -z "$gcroot_base" ]]; then
  case "$environment" in
    production)
      gcroot_base="/nix/var/nix/gcroots/fishystuff/gitops"
      ;;
    beta)
      gcroot_base="/nix/var/nix/gcroots/fishystuff/gitops-beta"
      ;;
    *)
      gcroot_base="/nix/var/nix/gcroots/fishystuff/gitops-${environment}"
      ;;
  esac
fi
dolt_release_ref_prefix="${FISHYSTUFF_GITOPS_DOLT_RELEASE_REF_PREFIX:-fishystuff/gitops}"

api_attr="${FISHYSTUFF_GITOPS_API_ATTR:-api-service-bundle-production}"
site_attr="${FISHYSTUFF_GITOPS_SITE_ATTR:-site-content}"
cdn_runtime_attr="${FISHYSTUFF_GITOPS_CDN_RUNTIME_ATTR:-cdn-serving-root}"
dolt_service_attr="${FISHYSTUFF_GITOPS_DOLT_SERVICE_ATTR:-dolt-service-bundle-production}"

require_safe_name FISHYSTUFF_GITOPS_CLUSTER "$cluster"
require_safe_name FISHYSTUFF_GITOPS_ENVIRONMENT "$environment"
require_safe_name FISHYSTUFF_GITOPS_HOST_KEY "$host_key"
require_safe_name FISHYSTUFF_GITOPS_HOSTNAME "$hostname"
require_safe_ref_name FISHYSTUFF_GITOPS_DOLT_BRANCH_CONTEXT "$dolt_branch_context"

desired_generation="${FISHYSTUFF_GITOPS_GENERATION:-1}"
release_generation="${FISHYSTUFF_GITOPS_RELEASE_GENERATION:-$desired_generation}"
require_positive_int FISHYSTUFF_GITOPS_GENERATION "$desired_generation"
require_positive_int FISHYSTUFF_GITOPS_RELEASE_GENERATION "$release_generation"

git_rev="${FISHYSTUFF_GITOPS_GIT_REV:-$(current_git_rev)}"
dolt_commit="${FISHYSTUFF_GITOPS_DOLT_COMMIT:-}"
if [[ -z "$dolt_commit" ]]; then
  require_command dolt
  dolt_commit="$(current_dolt_commit "$dolt_ref")"
fi
require_value "$git_rev" "git revision must not be empty"
require_value "$dolt_commit" "Dolt commit must not be empty"

dolt_remote_url="${FISHYSTUFF_GITOPS_DOLT_REMOTE_URL:-}"
if [[ -z "$dolt_remote_url" ]]; then
  require_command dolt
  dolt_remote_url="$(current_dolt_remote_url)"
fi
reject_credential_url FISHYSTUFF_GITOPS_DOLT_REMOTE_URL "$dolt_remote_url"

api_closure="$(build_or_use_path FISHYSTUFF_GITOPS_API_CLOSURE "$api_attr")"
site_closure="$(build_or_use_path FISHYSTUFF_GITOPS_SITE_CLOSURE "$site_attr")"
cdn_runtime_closure="$(build_or_use_path FISHYSTUFF_GITOPS_CDN_RUNTIME_CLOSURE "$cdn_runtime_attr")"
dolt_service_closure="$(build_or_use_path FISHYSTUFF_GITOPS_DOLT_SERVICE_CLOSURE "$dolt_service_attr")"

require_store_path FISHYSTUFF_GITOPS_API_CLOSURE "$api_closure"
require_store_path FISHYSTUFF_GITOPS_SITE_CLOSURE "$site_closure"
require_store_path FISHYSTUFF_GITOPS_CDN_RUNTIME_CLOSURE "$cdn_runtime_closure"
require_store_path FISHYSTUFF_GITOPS_DOLT_SERVICE_CLOSURE "$dolt_service_closure"

release_material="$(
  jq -cnS \
    --argjson generation "$release_generation" \
    --arg git_rev "$git_rev" \
    --arg dolt_commit "$dolt_commit" \
    --arg dolt_repository "fishystuff/fishystuff" \
    --arg dolt_branch_context "$dolt_branch_context" \
    --arg dolt_mode "read_only" \
    --arg api "$api_closure" \
    --arg site "$site_closure" \
    --arg cdn_runtime "$cdn_runtime_closure" \
    --arg dolt_service "$dolt_service_closure" \
    '{
      generation: $generation,
      git_rev: $git_rev,
      dolt_commit: $dolt_commit,
      dolt_repository: $dolt_repository,
      dolt_branch_context: $dolt_branch_context,
      dolt_mode: $dolt_mode,
      api: $api,
      site: $site,
      cdn_runtime: $cdn_runtime,
      dolt_service: $dolt_service
    }'
)"
release_hash="$(printf '%s' "$release_material" | sha256sum | awk '{ print $1 }')"
release_id="release-${release_hash:0:16}"
release_ref="${dolt_release_ref_prefix}/${release_id}"
retained_releases_source="$(read_retained_releases_source)"
retained_releases="$(
  jq -c \
    --arg active_release_id "$release_id" \
    --arg default_remote_url "$dolt_remote_url" \
    --arg default_cache_dir "$dolt_cache_dir" \
    --arg default_gcroot_base "$gcroot_base" \
    --arg default_release_ref_prefix "$dolt_release_ref_prefix" \
    --arg dolt_branch_context "$dolt_branch_context" \
    '
      def string_field($name):
        if (.[$name] | type) == "string" and .[$name] != "" then
          .[$name]
        else
          error("retained release requires non-empty string field " + $name)
        end;
      def optional_string_field($name; $default):
        if has($name) then
          if (.[$name] | type) == "string" then
            .[$name]
          else
            error("retained release field " + $name + " must be a string")
          end
        else
          $default
        end;
      def positive_int_field($name):
        .[$name] as $value
        | if ($value | type) == "number" and $value > 0 and $value == ($value | floor) then
          $value
        else
          error("retained release requires positive integer field " + $name)
        end;
      def store_path_field($name):
        string_field($name) as $path
        | if $path | startswith("/nix/store/") then
            $path
          else
            error("retained release field " + $name + " must be a /nix/store path")
          end;
      def release_id_ok($id):
        if $id | test("^[A-Za-z0-9._-]+$") then
          $id
        else
          error("retained release_id contains unsupported characters")
        end;
      def no_userinfo($name; $url):
        if ($url | startswith("file://") | not) and ($url | test("^[A-Za-z][A-Za-z0-9+.-]*://[^/?#]*@")) then
          error($name + " must not contain embedded credentials")
        else
          $url
        end;
      def closure($release_id; $name; $path; $gcroot_name):
        {
          enabled: true,
          store_path: $path,
          gcroot_path: ($default_gcroot_base + "/" + $release_id + "/" + $gcroot_name)
        };

      if type != "array" then
        error("FISHYSTUFF_GITOPS_RETAINED_RELEASES_JSON must be an array")
      else
        [
          .[] as $release
          | ($release | string_field("release_id") | release_id_ok(.)) as $release_id
          | ($release | positive_int_field("generation")) as $generation
          | ($release | string_field("git_rev")) as $git_rev
          | ($release | string_field("dolt_commit")) as $dolt_commit
          | ($release | store_path_field("api_closure")) as $api
          | ($release | store_path_field("site_closure")) as $site
          | ($release | store_path_field("cdn_runtime_closure")) as $cdn_runtime
          | ($release | store_path_field("dolt_service_closure")) as $dolt_service
          | ($release | optional_string_field("dolt_materialization"; "fetch_pin")) as $dolt_materialization
          | ($release | optional_string_field("dolt_remote_url"; $default_remote_url) | no_userinfo("retained release dolt_remote_url"; .)) as $dolt_remote_url
          | ($release | optional_string_field("dolt_cache_dir"; $default_cache_dir)) as $dolt_cache_dir
          | ($release | optional_string_field("dolt_release_ref"; $default_release_ref_prefix + "/" + $release_id)) as $dolt_release_ref
          | if $release_id == $active_release_id then
              error("retained release must not equal active release_id")
            elif $dolt_materialization != "fetch_pin" and $dolt_materialization != "metadata_only" then
              error("retained release dolt_materialization must be fetch_pin or metadata_only")
            else
              {
                release_id: $release_id,
                document: {
                  generation: $generation,
                  git_rev: $git_rev,
                  dolt_commit: $dolt_commit,
                  closures: {
                    api: closure($release_id; "api"; $api; "api"),
                    site: closure($release_id; "site"; $site; "site"),
                    cdn_runtime: closure($release_id; "cdn_runtime"; $cdn_runtime; "cdn-runtime"),
                    dolt_service: closure($release_id; "dolt_service"; $dolt_service; "dolt-service")
                  },
                  dolt: {
                    repository: "fishystuff/fishystuff",
                    commit: $dolt_commit,
                    branch_context: $dolt_branch_context,
                    mode: "read_only",
                    materialization: $dolt_materialization,
                    remote_url: $dolt_remote_url,
                    cache_dir: $dolt_cache_dir,
                    release_ref: $dolt_release_ref
                  }
                }
              }
            end
        ] as $releases
        | if ($releases | map(.release_id) | length) != ($releases | map(.release_id) | unique | length) then
            error("retained release IDs must be unique")
          else
            $releases
          end
      end
    ' <<< "$retained_releases_source"
)"
retained_release_ids="$(jq -c '[.[].release_id]' <<< "$retained_releases")"
retained_release_documents="$(jq -c 'reduce .[] as $release ({}; .[$release.release_id] = $release.document)' <<< "$retained_releases")"

json="$(
  jq -n \
    --argjson generation "$desired_generation" \
    --argjson release_generation "$release_generation" \
    --arg cluster "$cluster" \
    --arg environment "$environment" \
    --arg host_key "$host_key" \
    --arg hostname "$hostname" \
    --arg release_id "$release_id" \
    --arg git_rev "$git_rev" \
    --arg dolt_commit "$dolt_commit" \
    --arg dolt_branch_context "$dolt_branch_context" \
    --arg dolt_remote_url "$dolt_remote_url" \
    --arg dolt_release_ref "$release_ref" \
    --arg dolt_cache_dir "$dolt_cache_dir" \
    --arg gcroot_base "$gcroot_base" \
    --arg api "$api_closure" \
    --arg site "$site_closure" \
    --arg cdn_runtime "$cdn_runtime_closure" \
    --arg dolt_service "$dolt_service_closure" \
    --argjson retained_release_ids "$retained_release_ids" \
    --argjson retained_release_documents "$retained_release_documents" \
    '{
      cluster: $cluster,
      generation: $generation,
      mode: "validate",
      hosts: {
        ($host_key): {
          enabled: true,
          role: "single-site",
          hostname: $hostname
        }
      },
      releases: ({
        ($release_id): {
          generation: $release_generation,
          git_rev: $git_rev,
          dolt_commit: $dolt_commit,
          closures: {
            api: {
              enabled: true,
              store_path: $api,
              gcroot_path: ($gcroot_base + "/" + $release_id + "/api")
            },
            site: {
              enabled: true,
              store_path: $site,
              gcroot_path: ($gcroot_base + "/" + $release_id + "/site")
            },
            cdn_runtime: {
              enabled: true,
              store_path: $cdn_runtime,
              gcroot_path: ($gcroot_base + "/" + $release_id + "/cdn-runtime")
            },
            dolt_service: {
              enabled: true,
              store_path: $dolt_service,
              gcroot_path: ($gcroot_base + "/" + $release_id + "/dolt-service")
            }
          },
          dolt: {
            repository: "fishystuff/fishystuff",
            commit: $dolt_commit,
            branch_context: $dolt_branch_context,
            mode: "read_only",
            materialization: "fetch_pin",
            remote_url: $dolt_remote_url,
            cache_dir: $dolt_cache_dir,
            release_ref: $dolt_release_ref
          }
        }
      } + $retained_release_documents),
      environments: {
        ($environment): {
          enabled: true,
          strategy: "single_active",
          host: $host_key,
          active_release: $release_id,
          retained_releases: $retained_release_ids,
          serve: false
        }
      }
    }'
)"

if [[ "$output" == "-" ]]; then
  printf '%s\n' "$json"
else
  if [[ "$output" != /* ]]; then
    output="${RECIPE_REPO_ROOT}/${output}"
  fi
  mkdir -p "$(dirname "$output")"
  tmp="$(mktemp "$(dirname "$output")/.${output##*/}.XXXXXX")"
  printf '%s\n' "$json" > "$tmp"
  mv "$tmp" "$output"
  printf 'wrote %s\n' "$output" >&2
fi

retained_release_count="$(jq 'length' <<< "$retained_release_ids")"
printf 'gitops_current_environment=%s\n' "$environment" >&2
printf 'gitops_current_release_id=%s\n' "$release_id" >&2
printf 'gitops_current_dolt_commit=%s\n' "$dolt_commit" >&2
printf 'gitops_retained_release_count=%s\n' "$retained_release_count" >&2
if [[ "$environment" == "production" ]]; then
  printf 'production_current_release_id=%s\n' "$release_id" >&2
  printf 'production_current_dolt_commit=%s\n' "$dolt_commit" >&2
  printf 'production_retained_release_count=%s\n' "$retained_release_count" >&2
elif [[ "$environment" == "beta" ]]; then
  printf 'beta_current_release_id=%s\n' "$release_id" >&2
  printf 'beta_current_dolt_commit=%s\n' "$dolt_commit" >&2
  printf 'beta_retained_release_count=%s\n' "$retained_release_count" >&2
fi
