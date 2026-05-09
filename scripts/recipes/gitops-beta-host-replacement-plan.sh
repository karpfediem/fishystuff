#!/usr/bin/env bash
set -euo pipefail

SCRIPT_PATH="$(readlink -f "${BASH_SOURCE[0]}")"
SCRIPT_DIR="$(cd "$(dirname "$SCRIPT_PATH")" && pwd)"
source "${SCRIPT_DIR}/lib/common.sh"

old_server_name="$(normalize_named_arg old_server_name "${1-site-nbg1-beta}")"
replacement_server_name="$(normalize_named_arg replacement_server_name "${2-site-nbg1-beta-v2}")"
proof_dir="$(normalize_named_arg proof_dir "${3-data/gitops}")"

cd "$RECIPE_REPO_ROOT"

kv_value() {
  local key="$1"
  local file="$2"
  awk -F= -v key="$key" '$1 == key { print substr($0, length(key) + 2); exit }' "$file"
}

require_kv_equals() {
  local key="$1"
  local file="$2"
  local expected="$3"
  local value=""

  value="$(kv_value "$key" "$file")"
  if [[ "$value" != "$expected" ]]; then
    echo "${key} expected ${expected}, got: ${value}" >&2
    exit 2
  fi
}

case "${FISHYSTUFF_OPERATOR_SECRETSPEC_PROFILE:-}" in
  production-deploy | prod-deploy | production)
    echo "beta host replacement plan must not run with production SecretSpec profile active: ${FISHYSTUFF_OPERATOR_SECRETSPEC_PROFILE}" >&2
    exit 2
    ;;
esac

tmp_dir="$(mktemp -d)"
cleanup() {
  rm -rf "$tmp_dir"
}
trap cleanup EXIT

inventory_packet="${tmp_dir}/inventory.out"
proof_index="${tmp_dir}/proof-index.out"

bash scripts/recipes/gitops-beta-hetzner-inventory-packet.sh "$old_server_name" "$replacement_server_name" >"$inventory_packet"
require_kv_equals remote_deploy_performed "$inventory_packet" false
require_kv_equals infrastructure_mutation_performed "$inventory_packet" false
require_kv_equals local_host_mutation_performed "$inventory_packet" false

proof_complete="false"
if bash scripts/recipes/gitops-beta-proof-index.sh "$proof_dir" 86400 false >"$proof_index"; then
  proof_complete="$(kv_value gitops_beta_proof_index_complete "$proof_index")"
else
  proof_complete="false"
fi

inventory_status="$(kv_value inventory_status "$inventory_packet")"
old_status="$(kv_value old_server_status "$inventory_packet")"
replacement_status="$(kv_value replacement_server_status "$inventory_packet")"
replacement_ipv4="$(kv_value replacement_server_public_ipv4 "$inventory_packet")"
next_required_action="inspect_replacement_plan"

if [[ "$inventory_status" != "ready" ]]; then
  next_required_action="load_beta_deploy_credentials_for_inventory"
elif [[ "$replacement_status" == "missing" ]]; then
  next_required_action="create_replacement_beta_host_after_confirmation"
elif [[ "$replacement_status" == "present" && -n "$replacement_ipv4" && "$proof_complete" != "true" ]]; then
  next_required_action="bootstrap_and_prove_replacement_beta_host"
elif [[ "$replacement_status" == "present" && "$proof_complete" == "true" && "$old_status" == "present" ]]; then
  next_required_action="retire_old_beta_host_after_confirmation"
elif [[ "$replacement_status" == "present" && "$old_status" == "missing" ]]; then
  next_required_action="old_beta_host_already_absent"
fi

printf 'gitops_beta_host_replacement_plan_ok=true\n'
printf 'deployment=beta\n'
printf 'old_server_name=%s\n' "$old_server_name"
printf 'replacement_server_name=%s\n' "$replacement_server_name"
printf 'inventory_status=%s\n' "$inventory_status"
printf 'old_server_status=%s\n' "${old_status:-unknown}"
printf 'replacement_server_status=%s\n' "${replacement_status:-unknown}"
if [[ -n "$replacement_ipv4" ]]; then
  printf 'replacement_server_public_ipv4=%s\n' "$replacement_ipv4"
fi
printf 'beta_proof_index_complete=%s\n' "$proof_complete"
printf 'next_required_action=%s\n' "$next_required_action"
printf 'read_only_step_01=just gitops-beta-deploy-credentials-packet\n'
printf 'read_only_step_02=FISHYSTUFF_OPERATOR_SECRETSPEC_PROFILE=beta-deploy secretspec run --profile beta-deploy -- just gitops-beta-hetzner-inventory-packet old_server_name=%s replacement_server_name=%s\n' "$old_server_name" "$replacement_server_name"
printf 'read_only_step_03=just gitops-beta-host-provision-plan host_name=%s\n' "$replacement_server_name"
if [[ -n "$replacement_ipv4" ]]; then
  printf 'read_only_step_04=just gitops-beta-host-selection-packet public_ipv4=%s host_name=%s\n' "$replacement_ipv4" "$replacement_server_name"
else
  printf 'read_only_step_04=just gitops-beta-host-selection-packet public_ipv4=<replacement-public-ip> host_name=%s\n' "$replacement_server_name"
fi
printf 'read_only_step_05=just gitops-beta-first-service-set-packet\n'
printf 'retirement_blocker_01=do not retire old beta host until replacement proof index is complete\n'
printf 'retirement_blocker_02=do not use beta public DNS for replacement bootstrap until DNS is intentionally moved\n'
printf 'retirement_blocker_03=do not delete production-looking server names\n'
printf 'manual_create_confirmation_01=create replacement server only after confirming Hetzner server-count limit and beta deploy SSH key upload\n'
printf 'guarded_create_command_01=FISHYSTUFF_OPERATOR_SECRETSPEC_PROFILE=beta-deploy FISHYSTUFF_GITOPS_ENABLE_BETA_HETZNER_CREATE=1 FISHYSTUFF_GITOPS_BETA_HETZNER_CREATE_SERVER_NAME=%s secretspec run --profile beta-deploy -- just gitops-beta-hetzner-create-host server_name=%s\n' "$replacement_server_name" "$replacement_server_name"
printf 'manual_retire_confirmation_01=retire old_server_name=%s only after replacement serves beta and proof index is complete\n' "$old_server_name"
printf 'hcloud_create_command_emitted=false\n'
printf 'hetzner_api_create_command_available=true\n'
printf 'hcloud_delete_command_emitted=false\n'
printf 'ssh_command_emitted=false\n'
printf 'dns_mutation_command_emitted=false\n'
printf 'remote_deploy_performed=false\n'
printf 'infrastructure_mutation_performed=false\n'
printf 'local_host_mutation_performed=false\n'
