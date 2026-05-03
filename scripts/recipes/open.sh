#!/usr/bin/env bash
set -euo pipefail

SCRIPT_PATH="$(readlink -f "${BASH_SOURCE[0]}")"
SCRIPT_DIR="$(cd "$(dirname "$SCRIPT_PATH")" && pwd)"
source "${SCRIPT_DIR}/lib/common.sh"

cd "$RECIPE_REPO_ROOT"

deployment="${1-}"
require_value "$deployment" "usage: open.sh <deployment> [service ...]"
deployment="$(canonical_deployment_name "$deployment")"
shift || true
assert_deployment_configuration_safe "$deployment"

if (( $# == 0 )); then
  set -- site
fi

services=()
needs_tunnel=0
for service in "$@"; do
  service="$(canonical_public_service_name "$service")"
  services+=("$service")
  if [[ "$deployment" != "local" ]]; then
    case "$service" in
      grafana | dashboard | loki | logs | loki-status | prometheus | vector | jaeger)
        needs_tunnel=1
        ;;
    esac
  fi
done

profile="$(deployment_open_secretspec_profile "$deployment")"
if (( needs_tunnel )); then
  exec_with_secretspec_profile_if_needed "$profile" bash "$SCRIPT_PATH" "$deployment" "${services[@]}"
  assert_deployment_configuration_safe "$deployment"
fi

tmp_key=""
if (( needs_tunnel )); then
  tmp_key="$(create_temp_ssh_key_from_env /tmp/fishystuff-open.XXXXXX)"
  trap 'rm -f "$tmp_key"' EXIT
fi

ensure_tunnel() {
  local service="$1"
  local tunnel_target=""
  local tunnel_role=""
  local tunnel_expected_host=""
  local preferred_local_port=""
  local local_port=""
  local remote_port=""
  local socket_path=""
  local ttl_seconds=""
  local -a ssh_base=()
  local tunnel_pid=""
  local max_port_tries=32
  local max_ready_polls=20
  local try_index=0
  local ready_poll=0

  tunnel_target="$(deployment_tunnel_target "$deployment" "$service")"
  require_value "$tunnel_target" "deployment $deployment does not define a tunnel target"
  case "$service" in
    grafana | dashboard | jaeger | loki | logs | loki-status | prometheus | vector)
      if [[ -n "$(deployment_telemetry_target "$deployment")" ]]; then
        tunnel_role="telemetry"
        tunnel_expected_host="$(deployment_telemetry_hostname "$deployment")"
      else
        tunnel_role="resident"
        tunnel_expected_host="$(deployment_resident_hostname "$deployment")"
      fi
      ;;
    *)
      tunnel_role="resident"
      tunnel_expected_host="$(deployment_resident_hostname "$deployment")"
      ;;
  esac
  assert_remote_deployment_host "$deployment" "$tunnel_role" "$tunnel_target" "$tunnel_expected_host"
  preferred_local_port="$(deployment_open_tunnel_local_port "$service")"
  local_port="$preferred_local_port"
  remote_port="$(deployment_open_tunnel_remote_port "$service")"
  ttl_seconds="$(deployment_open_tunnel_ttl_seconds "$deployment")"
  require_value "$preferred_local_port" "service $service does not define a local tunnel port"
  require_value "$remote_port" "service $service does not define a remote tunnel port"

  for (( try_index = 0; try_index < max_port_tries; try_index++ )); do
    socket_path="/tmp/fishystuff-open-${deployment}-${service}-${local_port}.sock"
    ssh_base=(
      ssh
      -i "$tmp_key"
      -o IdentitiesOnly=yes
      -o StrictHostKeyChecking=accept-new
      -o ExitOnForwardFailure=yes
      -o ServerAliveInterval=30
      -o ServerAliveCountMax=3
      -S "$socket_path"
    )

    if "${ssh_base[@]}" -O check "$tunnel_target" >/dev/null 2>&1; then
      "${ssh_base[@]}" -O exit "$tunnel_target" >/dev/null 2>&1 || true
      rm -f "$socket_path"
      sleep 0.25
    fi

    if local_port_is_listening "$local_port"; then
      local_port="$((local_port + 1))"
      continue
    fi

    rm -f "$socket_path" "${socket_path}.cleanup.pid"
    if (( ttl_seconds > 0 )); then
      nohup timeout "$ttl_seconds" \
        "${ssh_base[@]}" \
        -N \
        -M \
        -L "127.0.0.1:${local_port}:127.0.0.1:${remote_port}" \
        "$tunnel_target" >/dev/null 2>&1 &
    else
      nohup \
        "${ssh_base[@]}" \
        -N \
        -M \
        -L "127.0.0.1:${local_port}:127.0.0.1:${remote_port}" \
        "$tunnel_target" >/dev/null 2>&1 &
    fi
    tunnel_pid="$!"

    for (( ready_poll = 0; ready_poll < max_ready_polls; ready_poll++ )); do
      if "${ssh_base[@]}" -O check "$tunnel_target" >/dev/null 2>&1; then
        printf '%s' "$local_port"
        return
      fi
      if ! kill -0 "$tunnel_pid" 2>/dev/null; then
        break
      fi
      sleep 0.25
    done

    if local_port_is_listening "$local_port"; then
      printf '%s' "$local_port"
      return
    fi

    if local_port_is_listening "$local_port"; then
      local_port="$((local_port + 1))"
      continue
    fi

    return 1
  done

  echo "could not allocate a free local port for $service tunnel starting at $preferred_local_port" >&2
  exit 1
}

open_url() {
  local url="$1"
  printf 'open %s\n' "$url"
  if [[ "${FS_SKIP_OPEN:-0}" == "1" ]]; then
    return
  fi
  if command -v xdg-open >/dev/null 2>&1; then
    xdg-open "$url" >/dev/null 2>&1 &
    return
  fi
  echo "xdg-open not found; open the URL above manually" >&2
}

for service in "${services[@]}"; do
  if [[ "$deployment" == "local" ]]; then
    open_url "$(deployment_open_url "$deployment" "$service")"
    continue
  fi
  case "$service" in
    site | map | api | cdn | telemetry)
      open_url "$(deployment_open_url "$deployment" "$service")"
      ;;
    grafana | dashboard | loki | logs | loki-status | prometheus | vector | jaeger)
      tunnel_local_port="$(ensure_tunnel "$service")"
      open_url "$(deployment_open_tunnel_url "$deployment" "$service" "$tunnel_local_port")"
      ;;
    *)
      echo "service $service is not openable" >&2
      exit 2
      ;;
  esac
done
