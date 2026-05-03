#!/usr/bin/env bash
set -euo pipefail

SCRIPT_PATH="$(readlink -f "${BASH_SOURCE[0]}")"
SCRIPT_DIR="$(cd "$(dirname "$SCRIPT_PATH")" && pwd)"
source "${SCRIPT_DIR}/lib/common.sh"

cd "$RECIPE_REPO_ROOT"

deployment="${1-}"
require_value "$deployment" "usage: deploy-safety-check.sh <deployment>"
deployment="$(canonical_deployment_name "$deployment")"

assert_deployment_configuration_safe "$deployment"

printf '[deploy-safety] %s passed\n' "$deployment"
printf '  secretspec_profile: %s\n' "$(deployment_secretspec_profile "$deployment")"
printf '  deployment_environment: %s\n' "$(deployment_environment_name "$deployment")"
printf '  dolt_remote_branch: %s\n' "$(deployment_dolt_remote_branch "$deployment")"
printf '  site: %s\n' "$(deployment_public_base_url "$deployment" site)"
printf '  api: %s\n' "$(deployment_public_base_url "$deployment" api)"
printf '  cdn: %s\n' "$(deployment_public_base_url "$deployment" cdn)"
printf '  telemetry: %s\n' "$(deployment_public_base_url "$deployment" telemetry)"
