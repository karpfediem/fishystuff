#!/usr/bin/env bash
set -euo pipefail

SCRIPT_PATH="$(readlink -f "${BASH_SOURCE[0]}")"
SCRIPT_DIR="$(cd "$(dirname "$SCRIPT_PATH")" && pwd)"
source "${SCRIPT_DIR}/lib/common.sh"

server_name="$(normalize_named_arg server_name "${1-site-nbg1-beta-v2}")"
server_type="$(normalize_named_arg server_type "${2-cx33}")"
image="$(normalize_named_arg image "${3-debian-13}")"
datacenter="$(normalize_named_arg datacenter "${4-nbg1-dc3}")"

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

require_env_value() {
  local name="$1"
  local expected="$2"
  local value="${!name-}"

  if [[ "$value" != "$expected" ]]; then
    fail "gitops-beta-hetzner-create-host requires ${name}=${expected}"
  fi
}

kv_value() {
  local key="$1"
  local file="$2"
  awk -F= -v key="$key" '$1 == key { print substr($0, length(key) + 2); exit }' "$file"
}

case "${FISHYSTUFF_OPERATOR_SECRETSPEC_PROFILE:-}" in
  beta-deploy) ;;
  "")
    fail "gitops-beta-hetzner-create-host requires FISHYSTUFF_OPERATOR_SECRETSPEC_PROFILE=beta-deploy"
    ;;
  *)
    fail "gitops-beta-hetzner-create-host must not run with non-beta SecretSpec profile active: ${FISHYSTUFF_OPERATOR_SECRETSPEC_PROFILE}"
    ;;
esac

assert_deployment_configuration_safe beta
assert_beta_infra_cluster_dns_scope_safe
require_env_value FISHYSTUFF_GITOPS_ENABLE_BETA_HETZNER_CREATE 1
require_env_value FISHYSTUFF_GITOPS_BETA_HETZNER_CREATE_SERVER_NAME "$server_name"

if [[ "$server_name" == "site-nbg1-prod" || "$server_name" == *production* || "$server_name" == *prod* ]]; then
  fail "beta replacement server_name must not look like production: ${server_name}"
fi
if [[ "$server_name" == "site-nbg1-beta" ]]; then
  fail "refusing to create replacement with old beta server name; use a distinct name such as site-nbg1-beta-v2"
fi
if [[ "$datacenter" != "nbg1-dc3" ]]; then
  fail "first beta replacement create is intentionally restricted to datacenter=nbg1-dc3"
fi
if [[ -z "${HETZNER_API_TOKEN:-}" ]]; then
  fail "HETZNER_API_TOKEN is required; run through beta-deploy SecretSpec"
fi

require_command curl
require_command jq
require_command mktemp

ssh_key_name="${HETZNER_SSH_KEY_NAME:-fishystuff-beta-deploy}"
resident_hostname="$(deployment_resident_hostname beta)"

tmp_dir="$(mktemp -d)"
cleanup() {
  rm -rf "$tmp_dir"
}
trap cleanup EXIT

inventory_packet="${tmp_dir}/inventory.out"
bash scripts/recipes/gitops-beta-hetzner-inventory-packet.sh site-nbg1-beta "$server_name" >"$inventory_packet"
if [[ "$(kv_value inventory_status "$inventory_packet")" != "ready" ]]; then
  cat "$inventory_packet" >&2
  fail "Hetzner inventory must be ready before creating replacement host"
fi
if [[ "$(kv_value replacement_server_status "$inventory_packet")" != "missing" ]]; then
  cat "$inventory_packet" >&2
  fail "replacement server already exists: ${server_name}"
fi

user_data="${tmp_dir}/cloud-init.yaml"
cat >"$user_data" <<EOF
#cloud-config
preserve_hostname: false
hostname: ${resident_hostname}
manage_etc_hosts: true
EOF

payload="${tmp_dir}/create.json"
jq -n \
  --arg name "$server_name" \
  --arg server_type "$server_type" \
  --arg image "$image" \
  --arg datacenter "$datacenter" \
  --arg ssh_key "$ssh_key_name" \
  --arg user_data "$(cat "$user_data")" \
  '{
    name: $name,
    server_type: $server_type,
    image: $image,
    datacenter: $datacenter,
    ssh_keys: [$ssh_key],
    start_after_create: true,
    user_data: $user_data,
    labels: {
      "fishystuff.deployment": "beta",
      "fishystuff.role": "resident",
      "fishystuff.gitops_service_set": "true",
      "fishystuff.replacement_for": "site-nbg1-beta"
    }
  }' >"$payload"

response="${tmp_dir}/created.json"
curl -fsS \
  -X POST \
  -H "Authorization: Bearer ${HETZNER_API_TOKEN}" \
  -H "Content-Type: application/json" \
  -d @"$payload" \
  "https://api.hetzner.cloud/v1/servers" >"$response"

printf 'gitops_beta_hetzner_create_host_ok=true\n'
printf 'deployment=beta\n'
printf 'server_name=%s\n' "$server_name"
printf 'resident_hostname=%s\n' "$resident_hostname"
printf 'server_id=%s\n' "$(jq -r '.server.id' "$response")"
printf 'server_status=%s\n' "$(jq -r '.server.status' "$response")"
printf 'server_public_ipv4=%s\n' "$(jq -r '.server.public_net.ipv4.ip // ""' "$response")"
printf 'server_type=%s\n' "$server_type"
printf 'server_image=%s\n' "$image"
printf 'server_datacenter=%s\n' "$datacenter"
printf 'ssh_key_name=%s\n' "$ssh_key_name"
printf 'next_read_only_command_01=just gitops-beta-host-selection-packet public_ipv4=%s host_name=%s\n' "$(jq -r '.server.public_net.ipv4.ip // "<pending>"' "$response")" "$server_name"
printf 'next_read_only_command_02=FISHYSTUFF_BETA_RESIDENT_TARGET=root@%s just gitops-beta-runtime-env-host-preflight\n' "$(jq -r '.server.public_net.ipv4.ip // "<pending>"' "$response")"
printf 'remote_deploy_performed=false\n'
printf 'infrastructure_mutation_performed=true\n'
printf 'local_host_mutation_performed=false\n'
