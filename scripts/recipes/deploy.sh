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
resident_host="$(deployment_resident_hostname "$deployment")"
require_value "$resident_target" "deployment $deployment does not define a resident target"
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
  "host=$resident_host"
  "deployment_environment=$(deployment_environment_name "$deployment")"
  "site_base_url=$(deployment_manifest_public_url "$deployment" "site")"
  "api_base_url=$(deployment_manifest_public_url "$deployment" "api")"
  "cdn_base_url=$(deployment_manifest_public_url "$deployment" "cdn")"
  "telemetry_base_url=$(deployment_manifest_public_url "$deployment" "telemetry")"
  "tls_enabled=$(deployment_tls_enabled "$deployment")"
  "tls_acme_email=$(deployment_tls_acme_email "$deployment")"
  "tls_challenge=$(deployment_tls_challenge "$deployment")"
  "tls_directory_url=$(deployment_tls_directory_url "$deployment")"
  "services_csv=$backend_services_csv"
)

for service in "${RECIPE_DEFAULT_DEPLOYMENT_SERVICES[@]}"; do
  if [[ -n "${selected_services[$service]:-}" ]]; then
    continue
  fi
  gcroot_path="$(deploy_service_gcroot_path "$service")"
  remote_store_path="$(bash "$SCRIPT_DIR/remote-gcroot-target.sh" "$resident_target" "$gcroot_path")"
  if [[ -z "$remote_store_path" ]]; then
    echo "remote gcroot is empty for $service on $resident_target: $gcroot_path" >&2
    exit 1
  fi
  backend_args+=("$(deploy_service_override_arg_name "$service")=$remote_store_path")
done

exec bash "${SCRIPT_DIR}/mgmt-resident-push-full-stack.sh" "${backend_args[@]}"
