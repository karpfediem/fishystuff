#!/usr/bin/env bash
set -euo pipefail

SCRIPT_PATH="$(readlink -f "${BASH_SOURCE[0]}")"
SCRIPT_DIR="$(cd "$(dirname "$SCRIPT_PATH")" && pwd)"
source "${SCRIPT_DIR}/lib/common.sh"

public_ipv4="$(normalize_named_arg public_ipv4 "${1-}")"
host_name="$(normalize_named_arg host_name "${2-$(deployment_resident_hostname beta)}")"
ssh_user="$(normalize_named_arg ssh_user "${3-root}")"

cd "$RECIPE_REPO_ROOT"

fail() {
  echo "$1" >&2
  exit 2
}

case "${FISHYSTUFF_OPERATOR_SECRETSPEC_PROFILE:-}" in
  production-deploy | prod-deploy | production)
    fail "beta host selection packet must not run with production SecretSpec profile active: ${FISHYSTUFF_OPERATOR_SECRETSPEC_PROFILE}"
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
if [[ -z "$ssh_user" || "$ssh_user" == *"@"* ]]; then
  fail "ssh_user must be a bare SSH username"
fi
if [[ -n "$public_ipv4" && ! "$public_ipv4" =~ ^([0-9]{1,3}\.){3}[0-9]{1,3}$ ]]; then
  fail "public_ipv4 must be an IPv4 address"
fi

expected_hostname="$(deployment_resident_hostname beta)"
selection_status="pending_public_ipv4"
resident_target="root@<new-beta-public-ip>"
if [[ -n "$public_ipv4" ]]; then
  selection_status="ready"
  resident_target="${ssh_user}@${public_ipv4}"
fi

printf 'gitops_beta_host_selection_packet_ok=true\n'
printf 'deployment=beta\n'
printf 'selection_status=%s\n' "$selection_status"
printf 'host_name=%s\n' "$host_name"
printf 'host_expected_hostname=%s\n' "$expected_hostname"
printf 'host_name_matches_expected_hostname=%s\n' "$(deployment_hostname_match_status "$host_name" "$expected_hostname")"
printf 'host_public_ipv4=%s\n' "${public_ipv4:-<required>}"
printf 'resident_target=%s\n' "$resident_target"
printf 'resident_target_source=operator_confirmed_public_ipv4\n'
printf 'public_dns_target_warning=do_not_use_beta_public_dns_for_new_host_until_dns_cutover_is_confirmed\n'
printf 'ssh_probe_performed=false\n'
printf 'host_reconfigure_performed=false\n'
printf 'operator_env_01=FISHYSTUFF_BETA_RESIDENT_TARGET=%s\n' "$resident_target"
printf 'operator_env_02=FISHYSTUFF_BETA_RESIDENT_HOSTNAME=%s\n' "$expected_hostname"
printf 'operator_env_03=FISHYSTUFF_OPERATOR_SECRETSPEC_PROFILE=beta-deploy\n'
printf 'read_only_next_command_01=FISHYSTUFF_OPERATOR_SECRETSPEC_PROFILE=beta-deploy secretspec run --profile beta-deploy -- just gitops-beta-remote-host-preflight target=%s\n' "$resident_target"
printf 'read_only_next_command_02=FISHYSTUFF_BETA_RESIDENT_TARGET=%s just gitops-beta-first-service-set-packet\n' "$resident_target"
printf 'guarded_followup_command_01=FISHYSTUFF_OPERATOR_SECRETSPEC_PROFILE=beta-deploy FISHYSTUFF_GITOPS_ENABLE_BETA_REMOTE_BOOTSTRAP=1 FISHYSTUFF_GITOPS_ENABLE_BETA_REMOTE_DIRECTORIES=1 FISHYSTUFF_GITOPS_ENABLE_BETA_REMOTE_USER_GROUPS=1 secretspec run --profile beta-deploy -- just gitops-beta-remote-host-bootstrap target=%s\n' "$resident_target"
printf 'remote_deploy_performed=false\n'
printf 'infrastructure_mutation_performed=false\n'
printf 'local_host_mutation_performed=false\n'
