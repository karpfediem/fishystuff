#!/usr/bin/env bash
set -euo pipefail

SCRIPT_PATH="$(readlink -f "${BASH_SOURCE[0]}")"
SCRIPT_DIR="$(cd "$(dirname "$SCRIPT_PATH")" && pwd)"
source "${SCRIPT_DIR}/lib/common.sh"

cd "$RECIPE_REPO_ROOT"

deployment="${1-}"
require_value "$deployment" "usage: deploy-authority-check.sh <deployment> [service ...]"
deployment="$(canonical_deployment_name "$deployment")"
shift || true

case "$deployment" in
  local)
    echo "deploy authority checks are for remote deployments; local is served by just up/build" >&2
    exit 2
    ;;
esac

allow_api_with_active_dolt=false
allow_api_with_active_dolt_reason=""
requested_services=()

while (( $# > 0 )); do
  case "$1" in
    --allow-api-with-active-dolt)
      allow_api_with_active_dolt=true
      shift
      ;;
    --reason)
      shift
      require_value "${1-}" "--reason requires a non-empty value"
      allow_api_with_active_dolt_reason="$1"
      shift
      ;;
    --reason=*)
      allow_api_with_active_dolt_reason="${1#*=}"
      require_value "$allow_api_with_active_dolt_reason" "--reason requires a non-empty value"
      shift
      ;;
    --*)
      echo "unknown deploy-authority-check flag: $1" >&2
      exit 2
      ;;
    *)
      requested_services+=("$1")
      shift
      ;;
  esac
done

assert_deployment_configuration_safe "$deployment"
if [[ "$deployment" == "beta" ]]; then
  assert_beta_infra_cluster_dns_scope_safe
fi

resident_target="$(deployment_resident_target "$deployment")"
telemetry_target="$(deployment_telemetry_target "$deployment")"
control_target="$(deployment_control_target "$deployment")"
resident_host="$(deployment_resident_hostname "$deployment")"
telemetry_host="$(deployment_telemetry_hostname "$deployment")"
control_host="$(deployment_control_hostname "$deployment")"
tls_challenge="$(deployment_tls_challenge "$deployment")"
tls_dns_provider="$(deployment_tls_dns_provider "$deployment")"
tls_dns_zone="$(deployment_tls_dns_zone "$deployment")"

declare -A selected_services=()
selected_service_order=()

add_selected_service() {
  local service
  service="$(canonical_deploy_service_name "$1")"
  if [[ -n "${selected_services[$service]:-}" ]]; then
    return
  fi
  selected_services["$service"]=1
  selected_service_order+=("$service")
}

expand_requested_service() {
  local service
  service="$(canonical_deploy_service_name "$1")"
  case "$service" in
    edge | site | cdn)
      add_selected_service edge
      add_selected_service site
      add_selected_service cdn
      ;;
    *)
      add_selected_service "$service"
      ;;
  esac
}

join_array() {
  local separator="$1"
  shift
  local joined=""
  local value=""

  for value in "$@"; do
    if [[ -n "$joined" ]]; then
      joined+="$separator"
    fi
    joined+="$value"
  done
  printf '%s' "$joined"
}

readarray -t default_deployment_services < <(deployment_default_services "$deployment")
readarray -t default_mutating_services < <(deployment_default_mutating_services "$deployment")
readarray -t resident_bundle_services < <(deployment_resident_bundle_services "$deployment")
readarray -t manifest_service_candidates < <(deployment_manifest_service_candidates "$deployment")

if (( ${#requested_services[@]} == 0 )); then
  used_default_services=true
  for service in "${default_mutating_services[@]}"; do
    add_selected_service "$service"
  done
else
  used_default_services=false
  for service in "${requested_services[@]}"; do
    expand_requested_service "$service"
  done
fi

api_with_active_dolt="false"
if [[ -n "${selected_services[api]:-}" && -z "${selected_services[dolt]:-}" ]]; then
  api_with_active_dolt="true"
  if [[ "$allow_api_with_active_dolt" != "true" || -z "$allow_api_with_active_dolt_reason" ]]; then
    cat >&2 <<'EOF'
refusing authority check:
  API was selected without Dolt, so a matching deploy would reuse active remote Dolt state.

resolution:
  include service "dolt"
  or pass --allow-api-with-active-dolt --reason "why the active Dolt state is compatible"
EOF
    exit 2
  fi
fi

dns_authority="none"
dns_authority_risk="none"
if [[ "$tls_challenge" == "dns-01" ]]; then
  dns_authority="${tls_dns_provider:-<missing>}:${tls_dns_zone:-<missing>}"
  if [[ "$deployment" == "beta" && "$tls_dns_provider" == "cloudflare" && "$tls_dns_zone" == "fishystuff.fish" ]]; then
    dns_authority_risk="accepted_parent_zone_scope_until_split"
  fi
fi

deploy_private_key_loaded="false"
if [[ -n "${HETZNER_SSH_PRIVATE_KEY:-}" ]]; then
  deploy_private_key_loaded="true"
fi

cloudflare_token_loaded="false"
if [[ -n "${CLOUDFLARE_API_TOKEN:-}" ]]; then
  cloudflare_token_loaded="true"
fi

origin_smoke_mode="public"
if [[ "$deployment" == "production" && "${FISHYSTUFF_DEPLOY_PUBLIC_SMOKE:-false}" != "true" ]]; then
  origin_smoke_mode="origin-ip"
fi
remote_mutation_state="${RECIPE_DEPLOY_AUTHORITY_REMOTE_MUTATION:-none}"

printf 'deployment: %s\n' "$deployment"
printf 'authority_check: passed\n'
printf 'secretspec_profile: %s\n' "$(deployment_secretspec_profile "$deployment")"
printf 'active_secretspec_profile: %s\n' "${FISHYSTUFF_OPERATOR_SECRETSPEC_PROFILE:-<not-loaded>}"
printf 'deploy_private_key_loaded: %s\n' "$deploy_private_key_loaded"
printf 'cloudflare_api_token_loaded: %s\n' "$cloudflare_token_loaded"
printf 'deployment_environment: %s\n' "$(deployment_environment_name "$deployment")"
printf 'dolt_remote_branch: %s\n' "$(deployment_dolt_remote_branch "$deployment")"
printf 'resident_target: %s\n' "${resident_target:-<none>}"
printf 'resident_expected_hostname: %s\n' "${resident_host:-<none>}"
printf 'telemetry_target: %s\n' "${telemetry_target:-<none>}"
printf 'telemetry_expected_hostname: %s\n' "${telemetry_host:-<none>}"
printf 'control_target: %s\n' "${control_target:-<none>}"
printf 'control_expected_hostname: %s\n' "${control_host:-<none>}"
printf 'site_base_url: %s\n' "$(deployment_public_base_url "$deployment" site)"
printf 'api_base_url: %s\n' "$(deployment_public_base_url "$deployment" api)"
printf 'cdn_base_url: %s\n' "$(deployment_public_base_url "$deployment" cdn)"
printf 'telemetry_base_url: %s\n' "$(deployment_public_base_url "$deployment" telemetry)"
printf 'tls_enabled: %s\n' "$(deployment_tls_enabled "$deployment")"
printf 'tls_challenge: %s\n' "$tls_challenge"
printf 'tls_dns_provider: %s\n' "${tls_dns_provider:-<none>}"
printf 'tls_dns_zone: %s\n' "${tls_dns_zone:-<none>}"
printf 'dns_mutation_authority: %s\n' "$dns_authority"
printf 'dns_mutation_authority_risk: %s\n' "$dns_authority_risk"
printf 'requested_services: %s\n' "$(join_array ',' "${requested_services[@]}")"
printf 'uses_default_mutating_services: %s\n' "$used_default_services"
printf 'selected_mutating_services: %s\n' "$(join_array ',' "${selected_service_order[@]}")"
printf 'api_with_active_dolt: %s\n' "$api_with_active_dolt"
printf 'default_status_services: %s\n' "$(join_array ',' "${default_deployment_services[@]}")"
printf 'default_mutating_services: %s\n' "$(join_array ',' "${default_mutating_services[@]}")"
printf 'resident_bundle_services: %s\n' "$(join_array ',' "${resident_bundle_services[@]}")"
printf 'resident_manifest_service_candidates: %s\n' "$(join_array ',' "${manifest_service_candidates[@]}")"
printf 'post_deploy_smoke_mode: %s\n' "$origin_smoke_mode"
printf 'remote_mutation: %s\n' "$remote_mutation_state"
