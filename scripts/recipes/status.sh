#!/usr/bin/env bash
set -euo pipefail

SCRIPT_PATH="$(readlink -f "${BASH_SOURCE[0]}")"
SCRIPT_DIR="$(cd "$(dirname "$SCRIPT_PATH")" && pwd)"
source "${SCRIPT_DIR}/lib/common.sh"

cd "$RECIPE_REPO_ROOT"

deployment="${1-}"
require_value "$deployment" "usage: status.sh <deployment> [service ...]"
deployment="$(canonical_deployment_name "$deployment")"
shift || true

profile="$(deployment_secretspec_profile "$deployment")"
exec_with_secretspec_profile_if_needed "$profile" bash "$SCRIPT_PATH" "$deployment" "$@"

declare -a requested_services=()
declare -A seen_services=()

add_requested_service() {
  local service
  service="$(canonical_public_service_name "$1")"
  if [[ -n "${seen_services[$service]:-}" ]]; then
    return
  fi
  seen_services["$service"]=1
  requested_services+=("$service")
}

if (( $# == 0 )); then
  while IFS= read -r service; do
    [[ -n "$service" ]] || continue
    add_requested_service "$service"
  done < <(deployment_default_services)
else
  for service in "$@"; do
    add_requested_service "$service"
  done
fi

resident_target="$(deployment_resident_target "$deployment")"
telemetry_target="$(deployment_telemetry_target "$deployment")"
resident_host="$(deployment_resident_hostname "$deployment")"

declare -A remote_unit_active=()
declare -A remote_unit_enabled=()
declare -A remote_root_store_path=()

load_remote_state() {
  local ssh_target="$1"
  shift

  local -a unit_names=()
  local -a root_paths=()
  local -A seen_units=()
  local -A seen_roots=()
  local service=""
  local unit_name=""
  local bundle_gcroot=""
  local content_gcroot=""
  local previous_content_gcroot=""
  local remote_output=""
  local tmp_key=""
  local line_type=""
  local key=""
  local value1=""
  local value2=""

  for service in "$@"; do
    unit_name="$(status_service_remote_unit_name "$service")"
    if [[ -n "$unit_name" && -z "${seen_units[$unit_name]:-}" ]]; then
      seen_units["$unit_name"]=1
      unit_names+=("$unit_name")
    fi

    bundle_gcroot="$(status_service_bundle_gcroot_path "$service")"
    if [[ -n "$bundle_gcroot" && -z "${seen_roots[$bundle_gcroot]:-}" ]]; then
      seen_roots["$bundle_gcroot"]=1
      root_paths+=("$bundle_gcroot")
    fi

    content_gcroot="$(status_service_content_gcroot_path "$service")"
    if [[ -n "$content_gcroot" && -z "${seen_roots[$content_gcroot]:-}" ]]; then
      seen_roots["$content_gcroot"]=1
      root_paths+=("$content_gcroot")
    fi

    previous_content_gcroot="$(status_service_previous_content_gcroot_path "$service")"
    if [[ -n "$previous_content_gcroot" && -z "${seen_roots[$previous_content_gcroot]:-}" ]]; then
      seen_roots["$previous_content_gcroot"]=1
      root_paths+=("$previous_content_gcroot")
    fi
  done

  if (( ${#unit_names[@]} == 0 && ${#root_paths[@]} == 0 )); then
    return
  fi

  tmp_key="$(create_temp_ssh_key_from_env /tmp/fishystuff-status-ssh.XXXXXX)"
  remote_output="$(
    ssh \
      -i "$tmp_key" \
      -o IdentitiesOnly=yes \
      -o StrictHostKeyChecking=accept-new \
      "$ssh_target" \
      /bin/bash -s -- "${#unit_names[@]}" "${#root_paths[@]}" "${unit_names[@]}" "${root_paths[@]}" <<'EOF'
set -euo pipefail

unit_count="${1:?missing unit count}"
root_count="${2:?missing root count}"
shift 2

units=()
for (( idx = 0; idx < unit_count; idx++ )); do
  units+=("${1:?missing unit}")
  shift
done

roots=()
for (( idx = 0; idx < root_count; idx++ )); do
  roots+=("${1:?missing gcroot}")
  shift
done

for unit in "${units[@]}"; do
  active="$(systemctl is-active "$unit" 2>/dev/null || true)"
  enabled="$(systemctl is-enabled "$unit" 2>/dev/null || true)"
  if [[ -z "$active" ]]; then
    active="missing"
  fi
  if [[ -z "$enabled" ]]; then
    enabled="missing"
  fi
  printf 'UNIT\t%s\t%s\t%s\n' "$unit" "$active" "$enabled"
done

for root in "${roots[@]}"; do
  store_path=""
  if [[ -e "$root" ]]; then
    store_path="$(readlink -f "$root")"
  fi
  printf 'ROOT\t%s\t%s\n' "$root" "$store_path"
done
EOF
  )" || {
    rm -f "$tmp_key"
    exit 1
  }
  rm -f "$tmp_key"

  while IFS=$'\t' read -r line_type key value1 value2; do
    case "$line_type" in
      UNIT)
        remote_unit_active["$key"]="$value1"
        remote_unit_enabled["$key"]="$value2"
        ;;
      ROOT)
        remote_root_store_path["$key"]="$value1"
        ;;
    esac
  done <<< "$remote_output"
}

status_target_for_service() {
  local service
  service="$(canonical_public_service_name "$1")"
  case "$service" in
    dashboard | grafana | jaeger | loki | logs | loki-status | otel-collector | prometheus | telemetry | vector)
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

if [[ "$deployment" != "local" ]]; then
  require_value "$resident_target" "deployment $deployment does not define a resident target"
  require_value "$resident_host" "deployment $deployment does not define a resident hostname"
  declare -A services_by_target=()
  for service in "${requested_services[@]}"; do
    target_for_service="$(status_target_for_service "$service")"
    services_by_target["$target_for_service"]+="${service} "
  done
  for target_for_service in "${!services_by_target[@]}"; do
    read -r -a target_services <<< "${services_by_target[$target_for_service]}"
    load_remote_state "$target_for_service" "${target_services[@]}"
  done
fi

print_service_status() {
  local service="$1"
  local direct_url=""
  local unit_name=""
  local bundle_gcroot=""
  local content_gcroot=""
  local previous_content_gcroot=""
  local backing_service=""
  local local_probe_port=""

  echo
  printf '[%s]\n' "$service"

  direct_url="$(status_service_direct_url "$deployment" "$service")"
  if [[ -n "$direct_url" ]]; then
    printf 'url: %s\n' "$direct_url"
  fi

  printf 'open: just open %s %s\n' "$deployment" "$service"

  backing_service="$(status_service_backing_gcroot_service "$service")"
  if [[ -n "$backing_service" && "$backing_service" != "$service" ]]; then
    printf 'backing_service: %s\n' "$backing_service"
  fi

  if [[ "$deployment" == "local" ]]; then
    local_probe_port="$(status_service_local_probe_port "$service")"
    if [[ -n "$local_probe_port" ]]; then
      printf 'probe_port: %s\n' "$local_probe_port"
      if local_port_is_listening "$local_probe_port"; then
        printf 'probe_state: listening\n'
      else
        printf 'probe_state: closed\n'
      fi
    fi
    unit_name="$(status_service_remote_unit_name "$service")"
    if [[ -n "$unit_name" ]]; then
      printf 'unit: %s\n' "$unit_name"
    fi
    return
  fi

  unit_name="$(status_service_remote_unit_name "$service")"
  if [[ -n "$unit_name" ]]; then
    printf 'unit: %s\n' "$unit_name"
    printf 'unit_active: %s\n' "${remote_unit_active[$unit_name]:-unknown}"
    printf 'unit_enabled: %s\n' "${remote_unit_enabled[$unit_name]:-unknown}"
  fi

  bundle_gcroot="$(status_service_bundle_gcroot_path "$service")"
  if [[ -n "$bundle_gcroot" ]]; then
    printf 'bundle_gcroot: %s\n' "$bundle_gcroot"
    printf 'bundle_store_path: %s\n' "${remote_root_store_path[$bundle_gcroot]:-}"
  fi

  content_gcroot="$(status_service_content_gcroot_path "$service")"
  if [[ -n "$content_gcroot" ]]; then
    printf 'content_gcroot: %s\n' "$content_gcroot"
    printf 'content_store_path: %s\n' "${remote_root_store_path[$content_gcroot]:-}"
    previous_content_gcroot="$(status_service_previous_content_gcroot_path "$service")"
    printf 'content_previous_gcroot: %s\n' "$previous_content_gcroot"
    printf 'content_previous_store_path: %s\n' "${remote_root_store_path[$previous_content_gcroot]:-}"
  fi

  case "$service" in
    grafana | dashboard | loki | logs | loki-status | prometheus | vector | jaeger)
      printf 'open_tunnel_ttl_seconds: %s\n' "$(deployment_open_tunnel_ttl_seconds "$deployment")"
      ;;
  esac
}

printf 'deployment: %s\n' "$deployment"
if [[ "$deployment" != "local" ]]; then
  printf 'resident_target: %s\n' "$resident_target"
  if [[ -n "$telemetry_target" ]]; then
    printf 'telemetry_target: %s\n' "$telemetry_target"
  fi
  printf 'resident_host: %s\n' "$resident_host"
fi

for service in "${requested_services[@]}"; do
  print_service_status "$service"
done
