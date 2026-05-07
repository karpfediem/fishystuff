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

build_or_use_path() {
  local env_name="$1"
  local attr="$2"
  local value="${!env_name:-}"
  if [[ -n "$value" ]]; then
    printf '%s\n' "$value"
    return
  fi
  nix build --no-link --print-out-paths ".#${attr}" | tail -n 1
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

api_closure="$(build_or_use_path FISHYSTUFF_GITOPS_API_CLOSURE api-service-bundle-production)"
site_closure="$(build_or_use_path FISHYSTUFF_GITOPS_SITE_CLOSURE site-content)"
cdn_runtime_closure="$(build_or_use_path FISHYSTUFF_GITOPS_CDN_RUNTIME_CLOSURE cdn-serving-root)"
dolt_service_closure="$(build_or_use_path FISHYSTUFF_GITOPS_DOLT_SERVICE_CLOSURE dolt-service-bundle-production)"

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
    --arg dolt_branch_context "main" \
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
release_ref="fishystuff/gitops/${release_id}"

json="$(
  jq -n \
    --argjson generation "$desired_generation" \
    --argjson release_generation "$release_generation" \
    --arg release_id "$release_id" \
    --arg git_rev "$git_rev" \
    --arg dolt_commit "$dolt_commit" \
    --arg dolt_remote_url "$dolt_remote_url" \
    --arg dolt_release_ref "$release_ref" \
    --arg api "$api_closure" \
    --arg site "$site_closure" \
    --arg cdn_runtime "$cdn_runtime_closure" \
    --arg dolt_service "$dolt_service_closure" \
    '{
      cluster: "production",
      generation: $generation,
      mode: "validate",
      hosts: {
        "production-single-host": {
          enabled: true,
          role: "single-site",
          hostname: "production-single-host"
        }
      },
      releases: {
        ($release_id): {
          generation: $release_generation,
          git_rev: $git_rev,
          dolt_commit: $dolt_commit,
          closures: {
            api: {
              enabled: true,
              store_path: $api,
              gcroot_path: ("/nix/var/nix/gcroots/fishystuff/gitops/" + $release_id + "/api")
            },
            site: {
              enabled: true,
              store_path: $site,
              gcroot_path: ("/nix/var/nix/gcroots/fishystuff/gitops/" + $release_id + "/site")
            },
            cdn_runtime: {
              enabled: true,
              store_path: $cdn_runtime,
              gcroot_path: ("/nix/var/nix/gcroots/fishystuff/gitops/" + $release_id + "/cdn-runtime")
            },
            dolt_service: {
              enabled: true,
              store_path: $dolt_service,
              gcroot_path: ("/nix/var/nix/gcroots/fishystuff/gitops/" + $release_id + "/dolt-service")
            }
          },
          dolt: {
            repository: "fishystuff/fishystuff",
            commit: $dolt_commit,
            branch_context: "main",
            mode: "read_only",
            materialization: "fetch_pin",
            remote_url: $dolt_remote_url,
            cache_dir: "/var/lib/fishystuff/gitops/dolt-cache/fishystuff",
            release_ref: $dolt_release_ref
          }
        }
      },
      environments: {
        production: {
          enabled: true,
          strategy: "single_active",
          host: "production-single-host",
          active_release: $release_id,
          retained_releases: [],
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

printf 'production_current_release_id=%s\n' "$release_id" >&2
printf 'production_current_dolt_commit=%s\n' "$dolt_commit" >&2
