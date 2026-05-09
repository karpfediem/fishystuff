#!/usr/bin/env bash
set -euo pipefail

SCRIPT_PATH="$(readlink -f "${BASH_SOURCE[0]}")"
SCRIPT_DIR="$(cd "$(dirname "$SCRIPT_PATH")" && pwd)"
source "${SCRIPT_DIR}/lib/common.sh"

api_runtime_env_path="$(normalize_named_arg api_runtime_env_path "${1-/var/lib/fishystuff/gitops-beta/api/runtime.env}")"
api_release_env_path="$(normalize_named_arg api_release_env_path "${2-/var/lib/fishystuff/gitops-beta/api/beta.env}")"
dolt_runtime_env_path="$(normalize_named_arg dolt_runtime_env_path "${3-/var/lib/fishystuff/gitops-beta/dolt/beta.env}")"

cd "$RECIPE_REPO_ROOT"

fail() {
  echo "$1" >&2
  exit 2
}

require_beta_path() {
  local label="$1"
  local path="$2"
  local expected="$3"

  if [[ "$path" != "$expected" ]]; then
    fail "${label} must be ${expected}, got: ${path}"
  fi
}

require_no_production_profile() {
  local active_profile="${FISHYSTUFF_OPERATOR_SECRETSPEC_PROFILE:-}"

  case "$active_profile" in
    production-deploy | prod-deploy | production)
      fail "beta host bootstrap plan must not run with production SecretSpec profile active: ${active_profile}"
      ;;
  esac
}

require_no_production_profile
assert_deployment_configuration_safe beta
assert_beta_infra_cluster_dns_scope_safe
require_beta_path "API runtime env path" "$api_runtime_env_path" "/var/lib/fishystuff/gitops-beta/api/runtime.env"
require_beta_path "API GitOps release env path" "$api_release_env_path" "/var/lib/fishystuff/gitops-beta/api/beta.env"
require_beta_path "Dolt runtime env path" "$dolt_runtime_env_path" "/var/lib/fishystuff/gitops-beta/dolt/beta.env"

resident_target="$(deployment_resident_target beta)"
resident_hostname="$(deployment_resident_hostname beta)"
telemetry_target="$(deployment_telemetry_target beta)"
telemetry_hostname="$(deployment_telemetry_hostname beta)"
control_target="$(deployment_control_target beta)"
control_hostname="$(deployment_control_hostname beta)"
site_base_url="$(deployment_public_base_url beta site)"
api_base_url="$(deployment_public_base_url beta api)"
cdn_base_url="$(deployment_public_base_url beta cdn)"
telemetry_base_url="$(deployment_public_base_url beta telemetry)"
tls_enabled="$(deployment_tls_enabled beta)"
tls_challenge="$(deployment_tls_challenge beta)"
tls_dns_provider="$(deployment_tls_dns_provider beta)"
tls_dns_zone="$(deployment_tls_dns_zone beta)"
hetzner_cluster="${FISHYSTUFF_HETZNER_CLUSTER:-beta}"
if [[ -z "$hetzner_cluster" ]]; then
  hetzner_cluster="beta"
fi

printf 'gitops_beta_host_bootstrap_plan_ok=true\n'
printf 'deployment=beta\n'
printf 'deployment_environment=beta\n'
printf 'resident_target=%s\n' "$resident_target"
printf 'resident_expected_hostname=%s\n' "$resident_hostname"
printf 'telemetry_target=%s\n' "$telemetry_target"
printf 'telemetry_expected_hostname=%s\n' "$telemetry_hostname"
printf 'control_target=%s\n' "$control_target"
printf 'control_expected_hostname=%s\n' "$control_hostname"
printf 'site_base_url=%s\n' "$site_base_url"
printf 'api_base_url=%s\n' "$api_base_url"
printf 'cdn_base_url=%s\n' "$cdn_base_url"
printf 'telemetry_base_url=%s\n' "$telemetry_base_url"
printf 'tls_enabled=%s\n' "$tls_enabled"
printf 'tls_challenge=%s\n' "$tls_challenge"
printf 'tls_dns_provider=%s\n' "$tls_dns_provider"
printf 'tls_dns_zone=%s\n' "$tls_dns_zone"
printf 'hetzner_cluster_dns_label=%s\n' "$hetzner_cluster"
printf 'api_loopback_upstream=http://127.0.0.1:18192\n'
printf 'dolt_loopback_sql=mysql://<user>:<password>@127.0.0.1:3316/fishystuff\n'
printf 'edge_admin_address=127.0.0.1:2119\n'
printf 'required_system_group_01=fishystuff-beta-dolt\n'
printf 'required_system_user_01=fishystuff-beta-dolt:fishystuff-beta-dolt\n'
printf 'required_directory_01_path=/var/lib/fishystuff/gitops-beta\n'
printf 'required_directory_01_mode=0750\n'
printf 'required_directory_02_path=/var/lib/fishystuff/gitops-beta/api\n'
printf 'required_directory_02_mode=0750\n'
printf 'required_directory_03_path=/var/lib/fishystuff/gitops-beta/dolt\n'
printf 'required_directory_03_mode=0750\n'
printf 'required_directory_04_path=/var/lib/fishystuff/gitops-beta/dolt-cache/fishystuff\n'
printf 'required_directory_04_mode=0750\n'
printf 'required_directory_05_path=/var/lib/fishystuff/gitops-beta/served/beta\n'
printf 'required_directory_05_mode=0755\n'
printf 'required_directory_06_path=/run/fishystuff/gitops-beta\n'
printf 'required_directory_06_mode=0750\n'
printf 'required_directory_07_path=/run/fishystuff/beta-edge/tls\n'
printf 'required_directory_07_mode=0700\n'
printf 'required_directory_08_path=/var/lib/fishystuff/beta-dolt\n'
printf 'required_directory_08_mode=0750\n'
printf 'api_runtime_env_path=%s\n' "$api_runtime_env_path"
printf 'api_release_env_path=%s\n' "$api_release_env_path"
printf 'dolt_runtime_env_path=%s\n' "$dolt_runtime_env_path"
printf 'service_unit_01=fishystuff-beta-dolt.service\n'
printf 'service_unit_02=fishystuff-beta-api.service\n'
printf 'service_unit_03=fishystuff-beta-edge.service\n'
printf 'read_only_readiness_check_01=just deploy-safety-check beta\n'
printf 'read_only_readiness_check_02=just deploy-authority-check beta dolt api edge site cdn\n'
printf 'read_only_readiness_check_03=just gitops-beta-service-bundles-test\n'
printf 'read_only_readiness_check_04=just gitops-beta-edge-handoff-bundle\n'
printf 'read_only_readiness_check_05=just gitops-beta-current-validate\n'
printf 'manual_bootstrap_step_01=provision or select a distinct beta host; do not reuse production credentials or production service names\n'
printf 'manual_bootstrap_step_02=create the required beta directories and the fishystuff-beta-dolt user/group if they do not already exist\n'
printf 'manual_bootstrap_step_03=materialize/copy beta API, Dolt, and edge service-bundle closures to the beta host\n'
printf 'manual_bootstrap_step_04=generate the beta current desired state and handoff summary with just gitops-beta-current-handoff\n'
printf 'manual_bootstrap_step_05=materialize/copy the exact release closures named by the beta handoff summary\n'
printf 'manual_bootstrap_step_06=write checked beta runtime env files with just gitops-beta-write-runtime-env service=dolt and service=api\n'
printf 'manual_bootstrap_step_07=review just gitops-beta-service-start-plan before installing or restarting API/Dolt units\n'
printf 'manual_bootstrap_step_08=start fishystuff-beta-dolt.service before fishystuff-beta-api.service through the guarded install-service commands from the start plan\n'
printf 'manual_bootstrap_step_09=collect admission evidence against http://127.0.0.1:18192 before any beta local apply claims serving readiness\n'
printf 'handoff_to_service_start_plan=just gitops-beta-service-start-plan api_bundle=auto dolt_bundle=auto api_env_file=%s dolt_env_file=%s\n' "$api_runtime_env_path" "$dolt_runtime_env_path"
printf 'handoff_to_activation_plan=just gitops-beta-host-handoff-plan draft_file=data/gitops/beta-activation.draft.desired.json summary_file=data/gitops/beta-current.handoff-summary.json admission_file=data/gitops/beta-admission.evidence.json\n'
printf 'refusal_condition_01=refuse if any checked URL or target resolves to production hostnames or production service names\n'
printf 'refusal_condition_02=refuse if production SecretSpec profile or production SSH key is active for beta bootstrap\n'
printf 'refusal_condition_03=refuse if API runtime env is not %s or API release env is not %s\n' "$api_runtime_env_path" "$api_release_env_path"
printf 'refusal_condition_04=refuse if Dolt runtime env is not %s\n' "$dolt_runtime_env_path"
printf 'refusal_condition_05=refuse if beta service-start plan has not been reviewed immediately before API/Dolt install\n'
printf 'remote_deploy_performed=false\n'
printf 'infrastructure_mutation_performed=false\n'
printf 'local_host_mutation_performed=false\n'
