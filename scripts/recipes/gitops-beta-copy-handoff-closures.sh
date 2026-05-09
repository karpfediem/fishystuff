#!/usr/bin/env bash
set -euo pipefail

SCRIPT_PATH="$(readlink -f "${BASH_SOURCE[0]}")"
SCRIPT_DIR="$(cd "$(dirname "$SCRIPT_PATH")" && pwd)"
source "${SCRIPT_DIR}/lib/common.sh"

target="$(normalize_named_arg target "${1:-${FISHYSTUFF_BETA_RESIDENT_TARGET:-}}")"
summary_file="$(normalize_named_arg summary_file "${2:-data/gitops/beta-current.handoff-summary.json}")"
push_bin="$(normalize_named_arg push_bin "${3:-scripts/recipes/push-closure.sh}")"

cd "$RECIPE_REPO_ROOT"

fail() {
  echo "$1" >&2
  exit 2
}

require_env_value() {
  local name="$1"
  local expected="$2"
  local value="${!name-}"

  if [[ "$value" != "$expected" ]]; then
    fail "gitops-beta-copy-handoff-closures requires ${name}=${expected}"
  fi
}

require_command_or_executable() {
  local command_name="$1"
  local label="$2"

  if [[ "$command_name" == */* ]]; then
    if [[ ! -x "$command_name" ]]; then
      fail "${label} is not executable: ${command_name}"
    fi
    return
  fi
  if ! command -v "$command_name" >/dev/null 2>&1; then
    echo "missing required command: ${command_name}" >&2
    exit 127
  fi
}

require_beta_deploy_profile() {
  local active_profile="${FISHYSTUFF_OPERATOR_SECRETSPEC_PROFILE:-}"

  case "$active_profile" in
    beta-deploy)
      ;;
    production-deploy | prod-deploy | production)
      fail "gitops-beta-copy-handoff-closures must not run with production SecretSpec profile active: ${active_profile}"
      ;;
    *)
      fail "gitops-beta-copy-handoff-closures requires FISHYSTUFF_OPERATOR_SECRETSPEC_PROFILE=beta-deploy"
      ;;
  esac
}

require_safe_target() {
  local value="$1"
  local user=""
  local host=""

  if [[ -z "$value" ]]; then
    fail "target is required; use target=root@<fresh-beta-ip>"
  fi
  if [[ "$value" != *@* ]]; then
    fail "target must be user@IPv4, got: ${value}"
  fi
  user="${value%@*}"
  host="${value#*@}"
  if [[ "$user" != "root" ]]; then
    fail "fresh beta closure copy currently expects root SSH, got user: ${user}"
  fi
  if [[ ! "$host" =~ ^([0-9]{1,3}\.){3}[0-9]{1,3}$ ]]; then
    fail "target host must be an IPv4 address, got: ${host}"
  fi
  if [[ "$host" == "178.104.230.121" ]]; then
    fail "target points at the previous beta host; use the fresh replacement IP"
  fi
}

summary_value() {
  local query="$1"
  jq -er "$query" "$summary_file"
}

require_summary_equals() {
  local label="$1"
  local query="$2"
  local expected="$3"
  local value=""

  value="$(summary_value "$query")"
  if [[ "$value" != "$expected" ]]; then
    fail "handoff summary ${label} must be ${expected}, got: ${value}"
  fi
}

require_store_path() {
  local label="$1"
  local value="$2"

  if [[ "$value" != /nix/store/* ]]; then
    fail "${label} must be a /nix/store path, got: ${value}"
  fi
  if [[ ! -e "$value" ]]; then
    fail "${label} does not exist locally: ${value}"
  fi
}

require_env_value FISHYSTUFF_GITOPS_ENABLE_BETA_REMOTE_CLOSURE_COPY 1
require_env_value FISHYSTUFF_GITOPS_BETA_REMOTE_CLOSURE_TARGET "$target"
require_beta_deploy_profile
assert_deployment_configuration_safe beta
assert_beta_infra_cluster_dns_scope_safe
require_safe_target "$target"
require_command_or_executable jq jq
require_command_or_executable "$push_bin" push_bin

if [[ ! -f "$summary_file" ]]; then
  fail "handoff summary does not exist: ${summary_file}"
fi

require_summary_equals schema '.schema' fishystuff.gitops.current-handoff.v1
require_summary_equals cluster '.cluster' beta
require_summary_equals environment '.environment.name' beta
require_summary_equals mode '.mode' validate
require_summary_equals closure_paths_verified '.checks.closure_paths_verified | tostring' true
require_summary_equals gitops_unify_passed '.checks.gitops_unify_passed | tostring' true
require_summary_equals summary_remote_deploy_performed '.checks.remote_deploy_performed | tostring' false
require_summary_equals summary_infrastructure_mutation_performed '.checks.infrastructure_mutation_performed | tostring' false

release_id="$(summary_value '.active_release.release_id')"
git_rev="$(summary_value '.active_release.git_rev')"
dolt_commit="$(summary_value '.active_release.dolt_commit')"
api_closure="$(summary_value '.active_release.closures.api')"
site_closure="$(summary_value '.active_release.closures.site')"
cdn_runtime_closure="$(summary_value '.active_release.closures.cdn_runtime')"
dolt_service_closure="$(summary_value '.active_release.closures.dolt_service')"

require_store_path api_closure "$api_closure"
require_store_path site_closure "$site_closure"
require_store_path cdn_runtime_closure "$cdn_runtime_closure"
require_store_path dolt_service_closure "$dolt_service_closure"

printf 'gitops_beta_copy_handoff_closures_ok=true\n'
printf 'deployment=beta\n'
printf 'resident_target=%s\n' "$target"
printf 'handoff_summary=%s\n' "$summary_file"
printf 'release_id=%s\n' "$release_id"
printf 'git_rev=%s\n' "$git_rev"
printf 'dolt_commit=%s\n' "$dolt_commit"
printf 'closure_01_api=%s\n' "$api_closure"
printf 'closure_02_site=%s\n' "$site_closure"
printf 'closure_03_cdn_runtime=%s\n' "$cdn_runtime_closure"
printf 'closure_04_dolt_service=%s\n' "$dolt_service_closure"

"$push_bin" \
  "$target" \
  "$api_closure" \
  "$site_closure" \
  "$cdn_runtime_closure" \
  "$dolt_service_closure"

printf 'remote_store_mutation_performed=true\n'
printf 'remote_deploy_performed=false\n'
printf 'infrastructure_mutation_performed=false\n'
printf 'local_host_mutation_performed=false\n'
