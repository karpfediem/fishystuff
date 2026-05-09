#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "${SCRIPT_DIR}/lib/common.sh"

output="$(normalize_named_arg output "${1-data/gitops/beta-current.desired.json}")"
dolt_ref="$(normalize_named_arg dolt_ref "${2-beta}")"
mgmt_bin="$(normalize_named_arg mgmt_bin "${3-auto}")"
deploy_bin="$(normalize_named_arg deploy_bin "${4-auto}")"
summary_output="$(normalize_named_arg summary_output "${5-}")"

export FISHYSTUFF_GITOPS_ENVIRONMENT="beta"
export FISHYSTUFF_GITOPS_CURRENT_DESIRED_SCRIPT="scripts/recipes/gitops-beta-current-desired.sh"
export FISHYSTUFF_GITOPS_HANDOFF_SCHEMA="fishystuff.gitops.current-handoff.v1"
export FISHYSTUFF_GITOPS_HANDOFF_REQUIRE_RETAINED="false"
export FISHYSTUFF_GITOPS_HANDOFF_CHECK_DESIRED_SERVING="false"

bash "${SCRIPT_DIR}/gitops-production-current-handoff.sh" \
  "$output" \
  "$dolt_ref" \
  "$mgmt_bin" \
  "$deploy_bin" \
  "$summary_output"
