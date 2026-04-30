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
allow_api_with_active_dolt=false
allow_api_with_active_dolt_reason=""
used_default_services=false

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

remote_exec_with_operator_key() {
  local ssh_target="$1"
  shift
  local tmp_key=""

  tmp_key="$(create_temp_ssh_key_from_env /tmp/fishystuff-deploy-ssh.XXXXXX)"
  trap 'rm -f "$tmp_key"' RETURN
  ssh \
    -i "$tmp_key" \
    -o IdentitiesOnly=yes \
    -o StrictHostKeyChecking=accept-new \
    "$ssh_target" \
    "$@"
  rm -f "$tmp_key"
  trap - RETURN
}

copy_remote_store_paths_to_local() {
  local ssh_target="$1"
  shift
  local tmp_key=""
  local remote_nix_daemon_path=""
  local nix_copy_target=""

  if (( $# == 0 )); then
    return 0
  fi

  tmp_key="$(create_temp_ssh_key_from_env /tmp/fishystuff-deploy-nix-copy.XXXXXX)"
  trap 'rm -f "$tmp_key"' RETURN

  remote_nix_daemon_path="$(detect_remote_nix_daemon_path "$ssh_target" "$tmp_key")"
  require_value "$remote_nix_daemon_path" "could not detect remote nix-daemon path on $ssh_target"
  nix_copy_target="$(build_nix_copy_target "$ssh_target" "$tmp_key" "$remote_nix_daemon_path")"

  printf '[deploy] copying %s retained store path(s) from %s\n' "$#" "$ssh_target" >&2
  nix copy --no-check-sigs --from "$nix_copy_target" "$@"

  rm -f "$tmp_key"
  trap - RETURN
}

collect_remote_cdn_retained_roots() {
  local gcroot_path=""
  local remote_store_path=""
  local joined=""
  local -a gcroot_paths=()
  local -a retained_roots=()
  local -A seen_roots=()

  gcroot_paths+=("$(deploy_service_gcroot_path cdn)")
  gcroot_paths+=("$(status_service_previous_content_gcroot_path cdn)")

  for gcroot_path in "${gcroot_paths[@]}"; do
    [[ -n "$gcroot_path" ]] || continue
    remote_store_path="$(bash "$SCRIPT_DIR/remote-gcroot-target.sh" "$resident_target" "$gcroot_path")"
    [[ -n "$remote_store_path" ]] || continue
    case "$remote_store_path" in
      /nix/store/*) ;;
      *)
        echo "remote CDN gcroot is not a Nix store path: $remote_store_path" >&2
        exit 2
        ;;
    esac
    if [[ -z "${seen_roots[$remote_store_path]+x}" ]]; then
      seen_roots["$remote_store_path"]=1
      retained_roots+=("$remote_store_path")
    fi
  done

  copy_remote_store_paths_to_local "$resident_target" "${retained_roots[@]}"

  for remote_store_path in "${retained_roots[@]}"; do
    if [[ -n "$joined" ]]; then
      joined+=":"
    fi
    joined+="$remote_store_path"
  done
  printf '%s' "$joined"
}

production_origin_post_apply() {
  local restart_api="false"
  local refresh_dolt="false"
  local restart_edge="false"

  if [[ -n "${selected_services[api]:-}" ]]; then
    restart_api="true"
  fi
  if [[ -n "${selected_services[dolt]:-}" ]]; then
    refresh_dolt="true"
  fi
  if [[ -n "${selected_services[edge]:-}" || -n "${selected_services[site]:-}" || -n "${selected_services[cdn]:-}" ]]; then
    restart_edge="true"
  fi

  remote_exec_with_operator_key "$resident_target" /bin/bash -s -- "$restart_api" "$refresh_dolt" "$restart_edge" <<'EOF'
set -euo pipefail
restart_api="${1:?missing api restart flag}"
refresh_dolt="${2:?missing dolt refresh flag}"
restart_edge="${3:?missing edge restart flag}"

if [[ "$refresh_dolt" == "true" ]]; then
  systemctl restart fishystuff-dolt.service
  systemctl is-active --quiet fishystuff-dolt.service
fi

if [[ "$restart_api" == "true" ]]; then
  systemctl restart fishystuff-api.service
  systemctl is-active --quiet fishystuff-api.service
fi

if [[ "$restart_edge" == "true" ]]; then
  systemctl restart fishystuff-edge.service
  systemctl is-active --quiet fishystuff-edge.service
fi
EOF
}

write_remote_deployment_marker() {
  local marker="$1"
  require_value "$marker" "cannot write empty deployment marker"
  remote_exec_with_operator_key "$resident_target" /bin/bash -s -- "$marker" <<'EOF'
set -euo pipefail
marker="${1:?missing deployment marker}"
install -d -m 0755 /run/fishystuff
printf '%s\n' "$marker" > /run/fishystuff/deployment-marker
chmod 0644 /run/fishystuff/deployment-marker
EOF
}

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
      echo "unknown deploy flag: $1" >&2
      exit 2
      ;;
    *)
      requested_services+=("$1")
      shift
      ;;
  esac
done

discovered_resident_ipv4=""
if [[ "$deployment" == "production" && -z "$resident_target" ]]; then
  discovered_resident_ipv4="$(hetzner_server_public_ipv4 "$resident_host")"
  resident_target="root@$discovered_resident_ipv4"
  printf '[deploy] discovered production resident target for %s: %s\n' "$resident_host" "$resident_target" >&2
fi
require_value "$resident_target" "deployment $deployment does not define a resident target"

if (( ${#requested_services[@]} == 0 )); then
  used_default_services=true
  while IFS= read -r service; do
    [[ -n "$service" ]] || continue
    add_selected_service "$service"
  done < <(deployment_default_mutating_services "$deployment")
else
  for service in "${requested_services[@]}"; do
    expand_requested_service "$service"
  done
fi

if [[ -n "${selected_services[api]:-}" && -z "${selected_services[dolt]:-}" ]]; then
  if [[ "$allow_api_with_active_dolt" != "true" || -z "$allow_api_with_active_dolt_reason" ]]; then
    cat >&2 <<'EOF'
refusing deploy:
  API was selected without Dolt, so this would reuse the active remote Dolt state.

resolution:
  include service "dolt"
  or pass --allow-api-with-active-dolt --reason "why the active Dolt state is compatible"
EOF
    exit 2
  fi

  printf 'warning: deploying API against active Dolt state; reason: %s\n' "$allow_api_with_active_dolt_reason" >&2
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

if [[ -z "${selected_services[dolt]:-}" ]]; then
  backend_args+=("dolt_refresh_enabled=false")
  backend_args+=("dolt_repo_snapshot_mode=off")
fi

if [[ -n "${selected_services[cdn]:-}" ]]; then
  cdn_retained_roots="$(collect_remote_cdn_retained_roots)"
  if [[ -n "$cdn_retained_roots" ]]; then
    backend_args+=("cdn_retained_roots=$cdn_retained_roots")
  fi
fi

FISHYSTUFF_DEPLOY_EXPECTED_MANIFEST="$expected_manifest" \
  bash "${SCRIPT_DIR}/mgmt-resident-push-full-stack.sh" "${backend_args[@]}"

if [[ "${FISHYSTUFF_DEPLOY_SMOKE:-true}" == "true" ]]; then
  if [[ "$deployment" == "production" && "${FISHYSTUFF_DEPLOY_PUBLIC_SMOKE:-false}" != "true" ]]; then
    expected_marker="$(jq -r '.deployment_marker // empty' "$expected_manifest")"
    production_origin_post_apply
    origin_ipv4="${FISHYSTUFF_ORIGIN_SMOKE_IPV4:-${FISHYSTUFF_SMOKE_ORIGIN_IPV4:-}}"
    if [[ -z "$origin_ipv4" ]]; then
      origin_ipv4="$discovered_resident_ipv4"
    fi
    if [[ -z "$origin_ipv4" ]]; then
      origin_ipv4="$(extract_ipv4_from_ssh_target "$resident_target")"
    fi
    require_value "$origin_ipv4" "production deploy uses origin smoke by default; set FISHYSTUFF_ORIGIN_SMOKE_IPV4 or FISHYSTUFF_DEPLOY_PUBLIC_SMOKE=true"
    bash "${SCRIPT_DIR}/origin-smoke.sh" "$deployment" "$origin_ipv4"
    write_remote_deployment_marker "$expected_marker"
    bash "${SCRIPT_DIR}/wait-deployment.sh" "$deployment" "$expected_manifest"
  else
    bash "${SCRIPT_DIR}/wait-deployment.sh" "$deployment" "$expected_manifest"
    bash "${SCRIPT_DIR}/smoke.sh" "$deployment"
  fi
fi
