#!/usr/bin/env bash
set -euo pipefail

SCRIPT_PATH="$(readlink -f "${BASH_SOURCE[0]}")"
SCRIPT_DIR="$(cd "$(dirname "$SCRIPT_PATH")" && pwd)"
source "${SCRIPT_DIR}/lib/common.sh"

old_server_name="$(normalize_named_arg old_server_name "${1-site-nbg1-beta}")"
replacement_server_name="$(normalize_named_arg replacement_server_name "${2-site-nbg1-beta-v2}")"

cd "$RECIPE_REPO_ROOT"

fail() {
  echo "$1" >&2
  exit 2
}

require_command() {
  local command_name="$1"
  if ! command -v "$command_name" >/dev/null 2>&1; then
    echo "missing required command: ${command_name}" >&2
    exit 127
  fi
}

safe_server_name() {
  local label="$1"
  local value="$2"

  if [[ -z "$value" ]]; then
    fail "${label} is required"
  fi
  if [[ "$value" == "site-nbg1-prod" || "$value" == *production* || "$value" == *prod* ]]; then
    fail "${label} must not look like production: ${value}"
  fi
  if [[ ! "$value" =~ ^[A-Za-z0-9][A-Za-z0-9._-]*$ ]]; then
    fail "${label} contains unsupported characters: ${value}"
  fi
}

server_field() {
  local file="$1"
  local selector="$2"

  jq -r --arg name "$selector" '
    [.servers[]? | select(.name == $name)] | first // empty
  ' "$file"
}

print_server() {
  local label="$1"
  local name="$2"
  local response="$3"
  local server_json=""
  local count=""

  count="$(jq -r --arg name "$name" '[.servers[]? | select(.name == $name)] | length' "$response")"
  printf '%s_server_name=%s\n' "$label" "$name"
  printf '%s_server_count=%s\n' "$label" "$count"
  if [[ "$count" == "0" ]]; then
    printf '%s_server_status=missing\n' "$label"
    return
  fi
  if [[ "$count" != "1" ]]; then
    printf '%s_server_status=ambiguous\n' "$label"
    return
  fi

  server_json="$(server_field "$response" "$name")"
  printf '%s_server_status=present\n' "$label"
  printf '%s_server_id=%s\n' "$label" "$(jq -r '.id' <<<"$server_json")"
  printf '%s_server_hcloud_status=%s\n' "$label" "$(jq -r '.status' <<<"$server_json")"
  printf '%s_server_public_ipv4=%s\n' "$label" "$(jq -r '.public_net.ipv4.ip // ""' <<<"$server_json")"
  printf '%s_server_type=%s\n' "$label" "$(jq -r '.server_type.name // ""' <<<"$server_json")"
  printf '%s_server_datacenter=%s\n' "$label" "$(jq -r '.datacenter.name // ""' <<<"$server_json")"
  printf '%s_server_image=%s\n' "$label" "$(jq -r '.image.name // .image.description // ""' <<<"$server_json")"
  printf '%s_server_label_deployment=%s\n' "$label" "$(jq -r '.labels["fishystuff.deployment"] // ""' <<<"$server_json")"
  printf '%s_server_label_role=%s\n' "$label" "$(jq -r '.labels["fishystuff.role"] // ""' <<<"$server_json")"
}

case "${FISHYSTUFF_OPERATOR_SECRETSPEC_PROFILE:-}" in
  production-deploy | prod-deploy | production)
    fail "beta Hetzner inventory packet must not run with production SecretSpec profile active: ${FISHYSTUFF_OPERATOR_SECRETSPEC_PROFILE}"
    ;;
esac

assert_deployment_configuration_safe beta
assert_beta_infra_cluster_dns_scope_safe
safe_server_name old_server_name "$old_server_name"
safe_server_name replacement_server_name "$replacement_server_name"
if [[ "$old_server_name" == "$replacement_server_name" ]]; then
  fail "old_server_name and replacement_server_name must differ during beta replacement"
fi

require_command jq
require_command mktemp

printf 'gitops_beta_hetzner_inventory_packet_ok=true\n'
printf 'deployment=beta\n'
printf 'old_server_name=%s\n' "$old_server_name"
printf 'replacement_server_name=%s\n' "$replacement_server_name"

if [[ -z "${HETZNER_API_TOKEN:-}" ]]; then
  printf 'inventory_status=unavailable\n'
  printf 'inventory_unavailable_reason=missing_HETZNER_API_TOKEN\n'
  printf 'inventory_next_command=FISHYSTUFF_OPERATOR_SECRETSPEC_PROFILE=beta-deploy secretspec run --profile beta-deploy -- just gitops-beta-hetzner-inventory-packet old_server_name=%s replacement_server_name=%s\n' "$old_server_name" "$replacement_server_name"
  printf 'remote_deploy_performed=false\n'
  printf 'infrastructure_mutation_performed=false\n'
  printf 'local_host_mutation_performed=false\n'
  exit 0
fi

require_command curl

tmp_dir="$(mktemp -d)"
cleanup() {
  rm -rf "$tmp_dir"
}
trap cleanup EXIT

response="${tmp_dir}/servers.json"
curl -fsS \
  -H "Authorization: Bearer ${HETZNER_API_TOKEN}" \
  -H "Content-Type: application/json" \
  "https://api.hetzner.cloud/v1/servers?per_page=50" >"$response"

printf 'inventory_status=ready\n'
print_server old "$old_server_name" "$response"
print_server replacement "$replacement_server_name" "$response"
printf 'remote_deploy_performed=false\n'
printf 'infrastructure_mutation_performed=false\n'
printf 'local_host_mutation_performed=false\n'
