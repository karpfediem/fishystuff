#!/usr/bin/env bash
set -euo pipefail

SCRIPT_PATH="$(readlink -f "${BASH_SOURCE[0]}")"
SCRIPT_DIR="$(cd "$(dirname "$SCRIPT_PATH")" && pwd)"
source "${SCRIPT_DIR}/lib/common.sh"

retire_server_name="$(normalize_named_arg retire_server_name "${1-site-nbg1-beta}")"
retire_server_id="$(normalize_named_arg retire_server_id "${2:-}")"
retire_server_ipv4="$(normalize_named_arg retire_server_ipv4 "${3:-}")"
active_server_name="$(normalize_named_arg active_server_name "${4-site-nbg1-beta-v2}")"
active_server_ipv4="$(normalize_named_arg active_server_ipv4 "${5:-}")"
curl_bin="$(normalize_named_arg curl_bin "${6:-curl}")"

cd "$RECIPE_REPO_ROOT"

fail() {
  echo "$1" >&2
  exit 2
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

require_env_value() {
  local name="$1"
  local expected="$2"
  local value="${!name-}"

  if [[ "$value" != "$expected" ]]; then
    fail "gitops-beta-hetzner-retire-host requires ${name}=${expected}"
  fi
}

require_beta_deploy_profile() {
  local active_profile="${FISHYSTUFF_OPERATOR_SECRETSPEC_PROFILE:-}"

  case "$active_profile" in
    beta-deploy)
      ;;
    production-deploy | prod-deploy | production)
      fail "gitops-beta-hetzner-retire-host must not run with production SecretSpec profile active: ${active_profile}"
      ;;
    *)
      fail "gitops-beta-hetzner-retire-host requires FISHYSTUFF_OPERATOR_SECRETSPEC_PROFILE=beta-deploy"
      ;;
  esac
}

require_safe_server_name() {
  local label="$1"
  local value="$2"

  if [[ -z "$value" ]]; then
    fail "${label} is required"
  fi
  if [[ "$value" == *production* || "$value" == *prod* ]]; then
    fail "${label} must not look like production: ${value}"
  fi
  if [[ ! "$value" =~ ^[A-Za-z0-9][A-Za-z0-9._-]*$ ]]; then
    fail "${label} contains unsupported characters: ${value}"
  fi
}

require_numeric_id() {
  local label="$1"
  local value="$2"

  if [[ ! "$value" =~ ^[0-9]+$ ]]; then
    fail "${label} must be a numeric Hetzner server id, got: ${value:-<empty>}"
  fi
}

require_ipv4() {
  local label="$1"
  local value="$2"

  if [[ ! "$value" =~ ^([0-9]{1,3}\.){3}[0-9]{1,3}$ ]]; then
    fail "${label} must be an IPv4 address, got: ${value:-<empty>}"
  fi
  IFS=. read -r a b c d <<<"$value"
  for octet in "$a" "$b" "$c" "$d"; do
    if (( 10#$octet > 255 )); then
      fail "${label} must be an IPv4 address, got: ${value}"
    fi
  done
}

hcloud_get() {
  local url="$1"
  "$curl_bin" \
    -fsS \
    -H "Authorization: Bearer ${HETZNER_API_TOKEN:?}" \
    -H "Content-Type: application/json" \
    "$url"
}

hcloud_delete() {
  local url="$1"
  "$curl_bin" \
    -fsS \
    -X DELETE \
    -H "Authorization: Bearer ${HETZNER_API_TOKEN:?}" \
    -H "Content-Type: application/json" \
    "$url"
}

server_by_id() {
  local file="$1"
  local id="$2"

  jq -r --argjson id "$id" '
    [.servers[]? | select(.id == $id)] | first // empty
  ' "$file"
}

server_count_by_name() {
  local file="$1"
  local name="$2"

  jq -r --arg name "$name" '[.servers[]? | select(.name == $name)] | length' "$file"
}

server_count_by_id() {
  local file="$1"
  local id="$2"

  jq -r --argjson id "$id" '[.servers[]? | select(.id == $id)] | length' "$file"
}

fetch_inventory() {
  local output="$1"

  hcloud_get "https://api.hetzner.cloud/v1/servers?per_page=50" >"$output"
}

require_beta_deploy_profile
assert_deployment_configuration_safe beta
assert_beta_infra_cluster_dns_scope_safe
require_env_value FISHYSTUFF_GITOPS_ENABLE_BETA_HETZNER_RETIRE 1
require_env_value FISHYSTUFF_GITOPS_BETA_HETZNER_RETIRE_SERVER_NAME "$retire_server_name"
require_env_value FISHYSTUFF_GITOPS_BETA_HETZNER_RETIRE_SERVER_ID "$retire_server_id"
require_env_value FISHYSTUFF_GITOPS_BETA_HETZNER_RETIRE_SERVER_IPV4 "$retire_server_ipv4"
if [[ -n "$active_server_ipv4" ]]; then
  require_env_value FISHYSTUFF_GITOPS_BETA_HETZNER_ACTIVE_SERVER_IPV4 "$active_server_ipv4"
fi

if [[ -z "${HETZNER_API_TOKEN:-}" ]]; then
  fail "HETZNER_API_TOKEN is required; run through beta-deploy SecretSpec"
fi
require_safe_server_name retire_server_name "$retire_server_name"
require_safe_server_name active_server_name "$active_server_name"
if [[ "$retire_server_name" == "$active_server_name" ]]; then
  fail "retire_server_name and active_server_name must differ"
fi
require_numeric_id retire_server_id "$retire_server_id"
require_ipv4 retire_server_ipv4 "$retire_server_ipv4"
if [[ -n "$active_server_ipv4" ]]; then
  require_ipv4 active_server_ipv4 "$active_server_ipv4"
fi
if [[ "$retire_server_ipv4" == "$active_server_ipv4" ]]; then
  fail "retire_server_ipv4 must not equal active_server_ipv4"
fi
require_command_or_executable jq jq
require_command_or_executable "$curl_bin" curl_bin

tmp_dir="$(mktemp -d)"
cleanup() {
  rm -rf "$tmp_dir"
}
trap cleanup EXIT

inventory="${tmp_dir}/servers.json"
fetch_inventory "$inventory"

active_count="$(server_count_by_name "$inventory" "$active_server_name")"
if [[ "$active_count" != "1" ]]; then
  fail "expected exactly one active beta server named ${active_server_name}, got ${active_count}"
fi
active_server="$(jq -r --arg name "$active_server_name" '[.servers[]? | select(.name == $name)] | first' "$inventory")"
resolved_active_ipv4="$(jq -r '.public_net.ipv4.ip // ""' <<<"$active_server")"
if [[ -n "$active_server_ipv4" && "$resolved_active_ipv4" != "$active_server_ipv4" ]]; then
  fail "active server ${active_server_name} has IPv4 ${resolved_active_ipv4}, expected ${active_server_ipv4}"
fi
active_deployment_label="$(jq -r '.labels["fishystuff.deployment"] // ""' <<<"$active_server")"
if [[ "$active_deployment_label" != "beta" ]]; then
  fail "active server ${active_server_name} must carry fishystuff.deployment=beta"
fi

target_count_by_id="$(server_count_by_id "$inventory" "$retire_server_id")"
if [[ "$target_count_by_id" == "0" ]]; then
  printf 'gitops_beta_hetzner_retire_host_ok=true\n'
  printf 'deployment=beta\n'
  printf 'retire_status=already_absent\n'
  printf 'retire_server_name=%s\n' "$retire_server_name"
  printf 'retire_server_id=%s\n' "$retire_server_id"
  printf 'retire_server_ipv4=%s\n' "$retire_server_ipv4"
  printf 'active_server_name=%s\n' "$active_server_name"
  printf 'active_server_ipv4=%s\n' "$resolved_active_ipv4"
  printf 'infrastructure_mutation_performed=false\n'
  printf 'remote_deploy_performed=false\n'
  printf 'production_mutation_performed=false\n'
  exit 0
fi
if [[ "$target_count_by_id" != "1" ]]; then
  fail "expected exactly one Hetzner server with id ${retire_server_id}, got ${target_count_by_id}"
fi
target_server="$(server_by_id "$inventory" "$retire_server_id")"
resolved_target_name="$(jq -r '.name // ""' <<<"$target_server")"
resolved_target_ipv4="$(jq -r '.public_net.ipv4.ip // ""' <<<"$target_server")"
target_gitops_label="$(jq -r '.labels["fishystuff.gitops_service_set"] // ""' <<<"$target_server")"
if [[ "$resolved_target_name" != "$retire_server_name" ]]; then
  fail "retire server id ${retire_server_id} is named ${resolved_target_name}, expected ${retire_server_name}"
fi
if [[ "$resolved_target_ipv4" != "$retire_server_ipv4" ]]; then
  fail "retire server ${retire_server_name} has IPv4 ${resolved_target_ipv4}, expected ${retire_server_ipv4}"
fi
if [[ "$target_gitops_label" == "true" ]]; then
  fail "refusing to retire a server labelled as the GitOps service set: ${retire_server_name}"
fi

target_count_by_name="$(server_count_by_name "$inventory" "$retire_server_name")"
if [[ "$target_count_by_name" != "1" ]]; then
  fail "expected exactly one retire beta server named ${retire_server_name}, got ${target_count_by_name}"
fi

delete_response="${tmp_dir}/delete.json"
hcloud_delete "https://api.hetzner.cloud/v1/servers/${retire_server_id}" >"$delete_response"
delete_action_status="$(jq -r '.action.status // "unknown"' "$delete_response")"
delete_action_command="$(jq -r '.action.command // "unknown"' "$delete_response")"

absent="false"
for _ in {1..30}; do
  sleep 2
  fetch_inventory "$inventory"
  if [[ "$(server_count_by_id "$inventory" "$retire_server_id")" == "0" ]]; then
    absent="true"
    break
  fi
done
if [[ "$absent" != "true" ]]; then
  fail "retire server id ${retire_server_id} was not absent after delete request"
fi

printf 'gitops_beta_hetzner_retire_host_ok=true\n'
printf 'deployment=beta\n'
printf 'retire_status=deleted\n'
printf 'retire_server_name=%s\n' "$retire_server_name"
printf 'retire_server_id=%s\n' "$retire_server_id"
printf 'retire_server_ipv4=%s\n' "$retire_server_ipv4"
printf 'active_server_name=%s\n' "$active_server_name"
printf 'active_server_ipv4=%s\n' "$resolved_active_ipv4"
printf 'delete_action_command=%s\n' "$delete_action_command"
printf 'delete_action_status=%s\n' "$delete_action_status"
printf 'infrastructure_mutation_performed=true\n'
printf 'remote_deploy_performed=false\n'
printf 'production_mutation_performed=false\n'
