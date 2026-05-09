#!/usr/bin/env bash
set -euo pipefail

SCRIPT_PATH="$(readlink -f "${BASH_SOURCE[0]}")"
SCRIPT_DIR="$(cd "$(dirname "$SCRIPT_PATH")" && pwd)"
source "${SCRIPT_DIR}/lib/common.sh"

host_name="$(normalize_named_arg host_name "${1-$(deployment_resident_hostname beta)}")"
server_type="$(normalize_named_arg server_type "${2-cx33}")"
image="$(normalize_named_arg image "${3-debian-13}")"
location="$(normalize_named_arg location "${4-nbg1}")"
datacenter="$(normalize_named_arg datacenter "${5-nbg1-dc3}")"

cd "$RECIPE_REPO_ROOT"

fail() {
  echo "$1" >&2
  exit 2
}

kv_value() {
  local key="$1"
  local file="$2"
  awk -F= -v key="$key" '$1 == key { print substr($0, length(key) + 2); exit }' "$file"
}

case "${FISHYSTUFF_OPERATOR_SECRETSPEC_PROFILE:-}" in
  production-deploy | prod-deploy | production)
    fail "beta host provision plan must not run with production SecretSpec profile active: ${FISHYSTUFF_OPERATOR_SECRETSPEC_PROFILE}"
    ;;
esac

assert_deployment_configuration_safe beta
assert_beta_infra_cluster_dns_scope_safe

if [[ -z "$host_name" ]]; then
  fail "host_name is required"
fi
if [[ "$host_name" == "site-nbg1-prod" || "$host_name" == *production* || "$host_name" == *prod* ]]; then
  fail "beta host_name must not look like production: ${host_name}"
fi
if [[ "$location" != "nbg1" ]]; then
  fail "first beta host provision plan is intentionally restricted to location=nbg1"
fi
if [[ "$datacenter" != "nbg1-dc3" ]]; then
  fail "first beta host provision plan is intentionally restricted to datacenter=nbg1-dc3"
fi
if [[ -z "$server_type" ]]; then
  fail "server_type is required"
fi
if [[ -z "$image" ]]; then
  fail "image is required"
fi

tmp_dir="$(mktemp -d)"
cleanup() {
  rm -rf "$tmp_dir"
}
trap cleanup EXIT

credentials_packet="${tmp_dir}/deploy-credentials.out"
bash scripts/recipes/gitops-beta-deploy-credentials-packet.sh >"$credentials_packet"

require_kv_false() {
  local key="$1"
  local value=""

  value="$(kv_value "$key" "$credentials_packet")"
  if [[ "$value" != "false" ]]; then
    fail "${key} expected false from credential packet, got: ${value}"
  fi
}

require_kv_false remote_deploy_performed
require_kv_false infrastructure_mutation_performed
require_kv_false local_host_mutation_performed

credential_status="$(kv_value beta_deploy_credentials_status "$credentials_packet")"
ssh_key_name="$(kv_value beta_deploy_ssh_key_name "$credentials_packet")"
ssh_key_pair_match="$(kv_value beta_deploy_ssh_key_pair_match "$credentials_packet")"
credential_next_action="$(kv_value beta_deploy_credentials_next_required_action "$credentials_packet")"
if [[ -z "$ssh_key_name" ]]; then
  ssh_key_name="fishystuff-beta-deploy"
fi

provision_status="pending_beta_deploy_credentials"
provision_ready="false"
if [[ "$credential_status" == "present" && "$ssh_key_pair_match" == "true" ]]; then
  provision_status="ready_for_manual_confirmation"
  provision_ready="true"
fi

resident_hostname="$(deployment_resident_hostname beta)"
resident_target="$(deployment_resident_target beta)"
site_base_url="$(deployment_public_base_url beta site)"
api_base_url="$(deployment_public_base_url beta api)"
cdn_base_url="$(deployment_public_base_url beta cdn)"
telemetry_base_url="$(deployment_public_base_url beta telemetry)"

printf 'gitops_beta_host_provision_plan_ok=true\n'
printf 'deployment=beta\n'
printf 'host_role=resident\n'
printf 'provision_plan_status=%s\n' "$provision_status"
printf 'provision_ready=%s\n' "$provision_ready"
printf 'manual_confirmation_required=true\n'
printf 'host_name=%s\n' "$host_name"
printf 'host_expected_hostname=%s\n' "$resident_hostname"
printf 'host_name_matches_expected_hostname=%s\n' "$(deployment_hostname_match_status "$host_name" "$resident_hostname")"
printf 'host_location=%s\n' "$location"
printf 'host_datacenter=%s\n' "$datacenter"
printf 'host_server_type=%s\n' "$server_type"
printf 'host_image=%s\n' "$image"
printf 'host_ssh_key_name=%s\n' "$ssh_key_name"
printf 'host_public_ipv4_source=manual_after_explicit_provision_confirmation\n'
printf 'resident_target_default=%s\n' "$resident_target"
printf 'resident_target_dns_cutover_warning=do_not_use_public_beta_dns_for_new_host_until_operator_confirms_it_points_at_the_new_host\n'
printf 'bootstrap_target_recommendation=root@<new-beta-public-ip>\n'
printf 'site_base_url=%s\n' "$site_base_url"
printf 'api_base_url=%s\n' "$api_base_url"
printf 'cdn_base_url=%s\n' "$cdn_base_url"
printf 'telemetry_base_url=%s\n' "$telemetry_base_url"
printf 'hetzner_label_01=fishystuff.deployment=beta\n'
printf 'hetzner_label_02=fishystuff.role=resident\n'
printf 'hetzner_label_03=fishystuff.gitops_service_set=true\n'
printf 'hetzner_label_04=fishystuff.location=%s\n' "$location"
printf 'beta_deploy_credentials_status=%s\n' "$credential_status"
printf 'beta_deploy_credentials_ssh_key_pair_match=%s\n' "$ssh_key_pair_match"
printf 'beta_deploy_credentials_next_required_action=%s\n' "$credential_next_action"
if [[ -n "$(kv_value beta_deploy_ssh_public_key_fingerprint "$credentials_packet")" ]]; then
  printf 'beta_deploy_credentials_ssh_public_key_fingerprint=%s\n' "$(kv_value beta_deploy_ssh_public_key_fingerprint "$credentials_packet")"
fi
printf 'read_only_check_01=just gitops-beta-deploy-credentials-packet\n'
printf 'read_only_check_02=just deploy-key-boundary-check\n'
printf 'read_only_check_03=just gitops-beta-host-bootstrap-plan\n'
printf 'read_only_check_04=just gitops-beta-first-service-set-packet\n'
printf 'manual_confirmation_step_01=confirm no existing active Hetzner server already owns host_name=%s\n' "$host_name"
printf 'manual_confirmation_step_02=confirm the beta deploy SSH key is uploaded to Hetzner under host_ssh_key_name=%s\n' "$ssh_key_name"
printf 'manual_confirmation_step_03=after explicit operator confirmation, provision/select exactly one beta resident host with the fields in this packet\n'
printf 'manual_confirmation_step_04=use bootstrap_target_recommendation with the new host public IPv4 until beta DNS is intentionally updated\n'
printf 'after_host_exists_read_only_step_01=just gitops-beta-runtime-env-host-preflight\n'
printf 'after_host_exists_guarded_step_01=FISHYSTUFF_GITOPS_ENABLE_BETA_HOST_BOOTSTRAP=1 FISHYSTUFF_GITOPS_ENABLE_BETA_HOST_DIRECTORIES=1 FISHYSTUFF_GITOPS_ENABLE_BETA_HOST_USER_GROUPS=1 just gitops-beta-host-bootstrap-apply\n'
printf 'hcloud_command_emitted=false\n'
printf 'ssh_command_emitted=false\n'
printf 'dns_mutation_command_emitted=false\n'
printf 'remote_deploy_performed=false\n'
printf 'infrastructure_mutation_performed=false\n'
printf 'local_host_mutation_performed=false\n'
