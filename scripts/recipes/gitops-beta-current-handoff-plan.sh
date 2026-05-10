#!/usr/bin/env bash
set -euo pipefail

SCRIPT_PATH="$(readlink -f "${BASH_SOURCE[0]}")"
SCRIPT_DIR="$(cd "$(dirname "$SCRIPT_PATH")" && pwd)"
source "${SCRIPT_DIR}/lib/common.sh"

output="$(normalize_named_arg output "${1-data/gitops/beta-current.desired.json}")"
dolt_ref="$(normalize_named_arg dolt_ref "${2-beta}")"
mgmt_bin="$(normalize_named_arg mgmt_bin "${3-auto}")"
deploy_bin="$(normalize_named_arg deploy_bin "${4-auto}")"
summary_output="$(normalize_named_arg summary_output "${5-}")"

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

require_safe_attr() {
  local name="$1"
  local value="$2"
  require_value "$value" "$name must not be empty"
  if [[ ! "$value" =~ ^[A-Za-z0-9._-]+$ ]]; then
    echo "$name contains unsupported characters: $value" >&2
    exit 2
  fi
}

reject_credential_url() {
  local name="$1"
  local value="$2"
  if [[ "$value" != file://* && "$value" =~ ^[A-Za-z][A-Za-z0-9+.-]*://[^/?#]*@ ]]; then
    echo "$name must not contain embedded credentials" >&2
    exit 2
  fi
}

print_closure_status() {
  local label="$1"
  local env_name="$2"
  local attr="$3"
  local value="${!env_name:-}"
  local status=""

  if [[ -n "$value" ]]; then
    if [[ "$value" != /nix/store/* ]]; then
      echo "${env_name} must be a /nix/store path, got: ${value}" >&2
      exit 2
    fi
    if [[ -e "$value" ]]; then
      status="provided_existing"
    else
      status="provided_missing"
      blocked="true"
    fi
    printf '%s_status=%s\n' "$label" "$status"
    printf '%s_path=%s\n' "$label" "$value"
    return
  fi

  status="will_build"
  closure_build_required="true"
  printf '%s_status=%s\n' "$label" "$status"
  printf '%s_attr=%s\n' "$label" "$attr"
}

print_cdn_runtime_status() {
  local attr="$1"
  local value="${FISHYSTUFF_GITOPS_CDN_RUNTIME_CLOSURE:-}"
  local operator_root="${FISHYSTUFF_OPERATOR_ROOT:-}"
  local status=""

  if [[ -n "$value" ]]; then
    print_closure_status cdn_runtime_closure FISHYSTUFF_GITOPS_CDN_RUNTIME_CLOSURE "$attr"
    return
  fi

  if [[ "$attr" == "cdn-serving-root" ]]; then
    if [[ -z "$operator_root" ]]; then
      status="blocked_missing_operator_root"
      blocked="true"
      printf 'cdn_runtime_closure_status=%s\n' "$status"
      printf 'cdn_runtime_closure_attr=%s\n' "$attr"
      printf 'cdn_runtime_operator_root_status=missing\n'
      return
    fi
    if [[ ! -d "$operator_root" ]]; then
      status="blocked_operator_root_missing"
      blocked="true"
      printf 'cdn_runtime_closure_status=%s\n' "$status"
      printf 'cdn_runtime_closure_attr=%s\n' "$attr"
      printf 'cdn_runtime_operator_root_status=missing_path\n'
      printf 'cdn_runtime_operator_root=%s\n' "$operator_root"
      return
    fi
    printf 'cdn_runtime_operator_root_status=present\n'
    printf 'cdn_runtime_operator_root=%s\n' "$operator_root"
    printf 'cdn_runtime_closure_build_mode=impure_operator_root\n'
  fi

  closure_build_required="true"
  printf 'cdn_runtime_closure_status=will_build\n'
  printf 'cdn_runtime_closure_attr=%s\n' "$attr"
}

print_git_status() {
  local git_rev="${FISHYSTUFF_GITOPS_GIT_REV:-}"

  if [[ -n "$git_rev" ]]; then
    printf 'git_rev_status=provided\n'
    printf 'git_rev=%s\n' "$git_rev"
    return
  fi
  if git rev-parse HEAD >/dev/null 2>&1; then
    local rev=""
    rev="$(git rev-parse HEAD)"
    if ! git diff-index --quiet HEAD --; then
      rev="${rev}-dirty"
    fi
    printf 'git_rev_status=discoverable\n'
    printf 'git_rev=%s\n' "$rev"
    return
  fi
  blocked="true"
  printf 'git_rev_status=blocked_not_discoverable\n'
}

print_dolt_commit_status() {
  local dolt_commit="${FISHYSTUFF_GITOPS_DOLT_COMMIT:-}"
  local output=""

  if [[ -n "$dolt_commit" ]]; then
    printf 'dolt_commit_status=provided\n'
    printf 'dolt_commit=%s\n' "$dolt_commit"
    return
  fi
  if ! command -v dolt >/dev/null 2>&1; then
    blocked="true"
    printf 'dolt_commit_status=blocked_missing_dolt_command\n'
    return
  fi
  if output="$(dolt log -n 1 "$dolt_ref" --oneline 2>/dev/null)" && [[ -n "$output" ]]; then
    printf 'dolt_commit_status=discoverable\n'
    printf 'dolt_commit=%s\n' "$(awk '{ print $1; exit }' <<< "$output")"
    return
  fi
  blocked="true"
  printf 'dolt_commit_status=blocked_ref_not_discoverable\n'
}

print_dolt_remote_status() {
  local dolt_remote_url="${FISHYSTUFF_GITOPS_DOLT_REMOTE_URL:-}"
  local remote_output=""
  local origin_url=""

  if [[ -n "$dolt_remote_url" ]]; then
    reject_credential_url FISHYSTUFF_GITOPS_DOLT_REMOTE_URL "$dolt_remote_url"
    printf 'dolt_remote_status=provided\n'
    printf 'dolt_remote_url=%s\n' "$dolt_remote_url"
    return
  fi
  if ! command -v dolt >/dev/null 2>&1; then
    blocked="true"
    printf 'dolt_remote_status=blocked_missing_dolt_command\n'
    return
  fi
  if ! remote_output="$(dolt remote -v 2>/dev/null)"; then
    blocked="true"
    printf 'dolt_remote_status=blocked_not_discoverable\n'
    return
  fi
  origin_url="$(awk '$1 == "origin" { print $2; exit }' <<< "$remote_output")"
  if [[ -n "$origin_url" ]]; then
    reject_credential_url discovered_dolt_remote "$origin_url"
    printf 'dolt_remote_status=discoverable_origin\n'
    printf 'dolt_remote_url=%s\n' "$origin_url"
    return
  fi
  printf 'dolt_remote_status=discoverable_default\n'
  printf 'dolt_remote_url=https://doltremoteapi.dolthub.com/fishystuff/fishystuff\n'
}

print_mgmt_status() {
  if [[ "$mgmt_bin" == "auto" ]]; then
    mgmt_build_required="true"
    printf 'mgmt_bin_status=auto_will_build\n'
    printf 'mgmt_flake=%s\n' "${FISHYSTUFF_GITOPS_MGMT_FLAKE:-${RECIPE_REPO_ROOT}#mgmt-gitops}"
    return
  fi
  if [[ "$mgmt_bin" == */* && -x "$mgmt_bin" ]]; then
    printf 'mgmt_bin_status=provided_executable\n'
    printf 'mgmt_bin=%s\n' "$mgmt_bin"
    return
  fi
  blocked="true"
  printf 'mgmt_bin_status=blocked_missing_executable\n'
  printf 'mgmt_bin=%s\n' "$mgmt_bin"
}

load_closure_from_existing_state() {
  local env_name="$1"
  local release_id="$2"
  local closure_name="$3"
  local value="${!env_name:-}"
  local state_value=""

  if [[ -n "$value" ]]; then
    return
  fi

  state_value="$(
    jq -er \
      --arg release_id "$release_id" \
      --arg closure_name "$closure_name" \
      '.releases[$release_id].closures[$closure_name].store_path | select(type == "string" and length > 0)' \
      "$state_file" 2>/dev/null || true
  )"
  if [[ -n "$state_value" ]]; then
    if [[ "$state_value" == /nix/store/* ]]; then
      printf -v "$env_name" '%s' "$state_value"
      existing_desired_state_closure_source="active_release"
    elif [[ "$existing_desired_state_closure_source" == "none" ]]; then
      existing_desired_state_closure_source="ignored_non_store_path"
    fi
  fi
}

load_existing_desired_state_closures() {
  local active_release=""

  existing_desired_state_status="missing"
  existing_desired_state_closure_source="none"
  if [[ ! -f "$state_file" ]]; then
    return
  fi

  existing_desired_state_status="present"
  active_release="$(
    jq -er '.environments.beta.active_release | select(type == "string" and length > 0)' \
      "$state_file" 2>/dev/null || true
  )"
  if [[ -z "$active_release" ]]; then
    existing_desired_state_closure_source="missing_active_release"
    return
  fi

  existing_desired_state_active_release="$active_release"
  load_closure_from_existing_state FISHYSTUFF_GITOPS_API_CLOSURE "$active_release" api
  load_closure_from_existing_state FISHYSTUFF_GITOPS_SITE_CLOSURE "$active_release" site
  load_closure_from_existing_state FISHYSTUFF_GITOPS_CDN_RUNTIME_CLOSURE "$active_release" cdn_runtime
  load_closure_from_existing_state FISHYSTUFF_GITOPS_DOLT_SERVICE_CLOSURE "$active_release" dolt_service
}

require_command awk
require_command git
require_command jq

environment="${FISHYSTUFF_GITOPS_ENVIRONMENT:-beta}"
cluster="${FISHYSTUFF_GITOPS_CLUSTER:-beta}"
dolt_branch_context="${FISHYSTUFF_GITOPS_DOLT_BRANCH_CONTEXT:-beta}"
api_attr="${FISHYSTUFF_GITOPS_API_ATTR:-api-service-bundle-beta-gitops-handoff}"
site_attr="${FISHYSTUFF_GITOPS_SITE_ATTR:-site-content-beta}"
cdn_runtime_attr="${FISHYSTUFF_GITOPS_CDN_RUNTIME_ATTR:-cdn-serving-root}"
dolt_service_attr="${FISHYSTUFF_GITOPS_DOLT_SERVICE_ATTR:-dolt-service-bundle-beta-gitops-handoff}"

if [[ "$environment" != "beta" || "$cluster" != "beta" ]]; then
  echo "gitops-beta-current-handoff-plan only describes beta handoff input readiness" >&2
  exit 2
fi
require_safe_name FISHYSTUFF_GITOPS_ENVIRONMENT "$environment"
require_safe_name FISHYSTUFF_GITOPS_CLUSTER "$cluster"
require_safe_ref_name FISHYSTUFF_GITOPS_DOLT_BRANCH_CONTEXT "$dolt_branch_context"
require_safe_ref_name dolt_ref "$dolt_ref"
require_safe_attr FISHYSTUFF_GITOPS_API_ATTR "$api_attr"
require_safe_attr FISHYSTUFF_GITOPS_SITE_ATTR "$site_attr"
require_safe_attr FISHYSTUFF_GITOPS_CDN_RUNTIME_ATTR "$cdn_runtime_attr"
require_safe_attr FISHYSTUFF_GITOPS_DOLT_SERVICE_ATTR "$dolt_service_attr"

state_file="$(absolute_path "$output")"
summary_file="$summary_output"
if [[ -z "$summary_file" ]]; then
  summary_file="${state_file%.desired.json}.handoff-summary.json"
  if [[ "$summary_file" == "$state_file" ]]; then
    summary_file="${state_file}.handoff-summary.json"
  fi
else
  summary_file="$(absolute_path "$summary_file")"
fi

blocked="false"
closure_build_required="false"
mgmt_build_required="false"
existing_desired_state_status="missing"
existing_desired_state_active_release=""
existing_desired_state_closure_source="none"
load_existing_desired_state_closures

printf 'gitops_beta_current_handoff_plan_ok=true\n'
printf 'environment=beta\n'
printf 'cluster=beta\n'
printf 'desired_state_path=%s\n' "$state_file"
printf 'existing_desired_state_status=%s\n' "$existing_desired_state_status"
if [[ -n "$existing_desired_state_active_release" ]]; then
  printf 'existing_desired_state_active_release=%s\n' "$existing_desired_state_active_release"
fi
printf 'existing_desired_state_closure_source=%s\n' "$existing_desired_state_closure_source"
printf 'handoff_summary_path=%s\n' "$summary_file"
printf 'dolt_ref=%s\n' "$dolt_ref"
printf 'deploy_bin=%s\n' "$deploy_bin"

print_git_status
print_dolt_commit_status
print_dolt_remote_status
print_closure_status api_closure FISHYSTUFF_GITOPS_API_CLOSURE "$api_attr"
print_closure_status site_closure FISHYSTUFF_GITOPS_SITE_CLOSURE "$site_attr"
print_cdn_runtime_status "$cdn_runtime_attr"
print_closure_status dolt_service_closure FISHYSTUFF_GITOPS_DOLT_SERVICE_CLOSURE "$dolt_service_attr"
print_mgmt_status

if [[ "$blocked" == "true" ]]; then
  printf 'handoff_plan_status=blocked\n'
  printf 'handoff_can_run=false\n'
else
  if [[ "$closure_build_required" == "true" || "$mgmt_build_required" == "true" ]]; then
    printf 'handoff_plan_status=ready_to_build\n'
  else
    printf 'handoff_plan_status=ready\n'
  fi
  printf 'handoff_can_run=true\n'
fi
printf 'closure_build_required=%s\n' "$closure_build_required"
printf 'mgmt_build_required=%s\n' "$mgmt_build_required"
printf 'read_only_handoff_command=just gitops-beta-current-handoff output=%s dolt_ref=%s mgmt_bin=%s deploy_bin=%s summary_output=%s\n' "$state_file" "$dolt_ref" "$mgmt_bin" "$deploy_bin" "$summary_file"
printf 'remote_deploy_performed=false\n'
printf 'infrastructure_mutation_performed=false\n'
printf 'local_host_mutation_performed=false\n'
