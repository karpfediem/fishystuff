#!/usr/bin/env bash
set -euo pipefail

SCRIPT_PATH="$(readlink -f "${BASH_SOURCE[0]}")"
SCRIPT_DIR="$(cd "$(dirname "$SCRIPT_PATH")" && pwd)"
source "${SCRIPT_DIR}/lib/common.sh"

cd "$RECIPE_REPO_ROOT"

deployment="${1-}"
manifest_path="${2-}"
require_value "$deployment" "usage: wait-deployment.sh <deployment> <manifest>"
require_value "$manifest_path" "usage: wait-deployment.sh <deployment> <manifest>"
deployment="$(canonical_deployment_name "$deployment")"

profile="$(deployment_secretspec_profile "$deployment")"
exec_with_secretspec_profile_if_needed "$profile" bash "$SCRIPT_PATH" "$deployment" "$manifest_path"

if [[ ! -f "$manifest_path" ]]; then
  echo "deployment manifest does not exist: $manifest_path" >&2
  exit 2
fi

marker="$(jq -r '.deployment_marker // empty' "$manifest_path")"
require_value "$marker" "deployment manifest does not include deployment_marker"

resident_target="$(deployment_resident_target "$deployment")"
telemetry_target="$(deployment_telemetry_target "$deployment")"
require_value "$resident_target" "deployment $deployment does not define a resident target"

timeout_secs="${FISHYSTUFF_DEPLOY_WAIT_TIMEOUT_SECS:-900}"
interval_secs="${FISHYSTUFF_DEPLOY_WAIT_INTERVAL_SECS:-5}"
started_at="$(date +%s)"
tmp_key="$(create_temp_ssh_key_from_env /tmp/fishystuff-deploy-wait-ssh.XXXXXX)"
trap 'rm -f "$tmp_key"' EXIT

targets=("$resident_target")
if [[ -n "$telemetry_target" && "$telemetry_target" != "$resident_target" ]]; then
  targets+=("$telemetry_target")
fi

remote_marker() {
  local target="$1"

  ssh \
    -i "$tmp_key" \
    -o ConnectTimeout=15 \
    -o IdentitiesOnly=yes \
    -o StrictHostKeyChecking=accept-new \
    "$target" \
    'cat /run/fishystuff/deployment-marker 2>/dev/null || true'
}

while true; do
  pending=()
  for target in "${targets[@]}"; do
    current_marker="$(remote_marker "$target")"
    current_marker="${current_marker//$'\r'/}"
    current_marker="${current_marker//$'\n'/}"
    if [[ "$current_marker" != "$marker" ]]; then
      pending+=("$target")
    fi
  done

  if (( ${#pending[@]} == 0 )); then
    printf '[deploy-wait] %s marker applied: %s\n' "$deployment" "$marker"
    exit 0
  fi

  now="$(date +%s)"
  if (( now - started_at >= timeout_secs )); then
    printf '[deploy-wait] %s marker did not apply after %ss: %s\n' \
      "$deployment" "$timeout_secs" "${pending[*]}" >&2
    exit 1
  fi

  printf '[deploy-wait] waiting for marker on: %s\n' "${pending[*]}"
  sleep "$interval_secs"
done
