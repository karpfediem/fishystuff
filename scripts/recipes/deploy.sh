#!/usr/bin/env bash
set -euo pipefail

SCRIPT_PATH="$(readlink -f "${BASH_SOURCE[0]}")"
SCRIPT_DIR="$(cd "$(dirname "$SCRIPT_PATH")" && pwd)"
source "${SCRIPT_DIR}/lib/common.sh"

cd "$RECIPE_REPO_ROOT"

deployment="${1-}"
require_value "$deployment" "usage: deploy.sh <deployment> [service ...]"
deployment="$(canonical_deployment_name "$deployment")"
shift || true

case "$deployment" in
  local)
    echo "deploy does not target the local development stack; use just up or just build" >&2
    exit 2
    ;;
esac

profile="$(deployment_secretspec_profile "$deployment")"
exec_with_secretspec_profile_if_needed "$profile" bash "$SCRIPT_PATH" "$deployment" "$@"

resident_target="$(deployment_resident_target "$deployment")"
telemetry_target="$(deployment_telemetry_target "$deployment")"
control_target="$(deployment_control_target "$deployment")"
resident_host="$(deployment_resident_hostname "$deployment")"
telemetry_host="$(deployment_telemetry_hostname "$deployment")"
prod_host="$(deployment_prod_hostname "$deployment")"
tls_challenge="$(deployment_tls_challenge "$deployment")"
tls_dns_provider="$(deployment_tls_dns_provider "$deployment")"
tls_dns_zone="$(deployment_tls_dns_zone "$deployment")"
require_value "$resident_target" "deployment $deployment does not define a resident target"
require_value "$control_target" "deployment $deployment does not define a control target"
require_value "$resident_host" "deployment $deployment does not define a resident hostname"

declare -A selected_services=()

add_selected_service() {
  local service
  service="$(canonical_deploy_service_name "$1")"
  selected_services["$service"]=1
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

gcroot_lookup_target_for_service() {
  local service="$1"
  case "$service" in
    grafana | jaeger | loki | otel-collector | prometheus | vector)
      if [[ -n "$telemetry_target" ]]; then
        printf '%s' "$telemetry_target"
      else
        printf '%s' "$resident_target"
      fi
      ;;
    *)
      printf '%s' "$resident_target"
      ;;
  esac
}

if (( $# == 0 )); then
  while IFS= read -r service; do
    [[ -n "$service" ]] || continue
    expand_requested_service "$service"
  done < <(deployment_default_services)
else
  for service in "$@"; do
    expand_requested_service "$service"
  done
fi

readarray -t resident_services < <(deployment_resident_bundle_services)
backend_services=()
for service in "${resident_services[@]}"; do
  backend_services+=("$(deploy_service_backend_name "$service")")
done
backend_services_csv="$(IFS=,; printf '%s' "${backend_services[*]}")"

backend_args=(
  "target=$resident_target"
  "telemetry_target=$telemetry_target"
  "deploy_target=$control_target"
  "host=$resident_host"
  "telemetry_host=$telemetry_host"
  "prod_host=$prod_host"
  "deployment_environment=$(deployment_environment_name "$deployment")"
  "site_base_url=$(deployment_manifest_public_url "$deployment" "site")"
  "api_base_url=$(deployment_manifest_public_url "$deployment" "api")"
  "cdn_base_url=$(deployment_manifest_public_url "$deployment" "cdn")"
  "telemetry_base_url=$(deployment_manifest_public_url "$deployment" "telemetry")"
  "dolt_remote_branch=$(deployment_dolt_remote_branch "$deployment")"
  "tls_enabled=$(deployment_tls_enabled "$deployment")"
  "tls_acme_email=$(deployment_tls_acme_email "$deployment")"
  "tls_challenge=$tls_challenge"
  "tls_directory_url=$(deployment_tls_directory_url "$deployment")"
  "services_csv=$backend_services_csv"
)
expected_manifest=""
if [[ "${FISHYSTUFF_DEPLOY_SMOKE:-true}" == "true" ]]; then
  expected_manifest="$(mktemp /tmp/fishystuff-deploy-manifest.XXXXXX.json)"
  trap 'rm -f "$expected_manifest"' EXIT
fi

if [[ "$tls_challenge" == "dns-01" ]]; then
  require_value "$tls_dns_provider" "deployment $deployment uses DNS-01 but does not define a TLS DNS provider"
  require_value "$tls_dns_zone" "deployment $deployment uses DNS-01 but does not define a TLS DNS zone"
  backend_args+=("tls_dns_provider=$tls_dns_provider")
  backend_args+=("tls_dns_env_json=$(jq -cn --arg zone "$tls_dns_zone" '{CLOUDFLARE_ZONE_NAME: $zone}')")
fi

for service in "${RECIPE_DEFAULT_DEPLOYMENT_SERVICES[@]}"; do
  if [[ -n "${selected_services[$service]:-}" ]]; then
    continue
  fi
  gcroot_path="$(deploy_service_gcroot_path "$service")"
  gcroot_target="$(gcroot_lookup_target_for_service "$service")"
  remote_store_path="$(bash "$SCRIPT_DIR/remote-gcroot-target.sh" "$gcroot_target" "$gcroot_path")"
  if [[ -z "$remote_store_path" ]]; then
    echo "remote gcroot is empty for $service on $gcroot_target: $gcroot_path" >&2
    exit 1
  fi
  backend_args+=("$(deploy_service_override_arg_name "$service")=$remote_store_path")
  if [[ "$service" == "vector" ]]; then
    vector_agent_store_path="$(bash "$SCRIPT_DIR/remote-gcroot-target.sh" "$resident_target" "$gcroot_path")"
    if [[ -z "$vector_agent_store_path" ]]; then
      echo "remote gcroot is empty for vector agent on $resident_target: $gcroot_path" >&2
      exit 1
    fi
    backend_args+=("vector_agent_bundle=$vector_agent_store_path")
  fi
done

FISHYSTUFF_DEPLOY_EXPECTED_MANIFEST="$expected_manifest" \
  bash "${SCRIPT_DIR}/mgmt-resident-push-full-stack.sh" "${backend_args[@]}"

if [[ "${FISHYSTUFF_DEPLOY_SMOKE:-true}" == "true" ]]; then
  bash "${SCRIPT_DIR}/wait-deployment.sh" "$deployment" "$expected_manifest"
  bash "${SCRIPT_DIR}/smoke.sh" "$deployment"
fi
