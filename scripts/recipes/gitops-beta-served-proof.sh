#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "${SCRIPT_DIR}/lib/common.sh"

output_dir="$(normalize_named_arg output_dir "${1-data/gitops}")"
draft_file="$(normalize_named_arg draft_file "${2-data/gitops/beta-activation.draft.desired.json}")"
summary_file="$(normalize_named_arg summary_file "${3-data/gitops/beta-current.handoff-summary.json}")"
admission_file="$(normalize_named_arg admission_file "${4-}")"
operator_proof_file="$(normalize_named_arg proof_file "${5-}")"
deploy_bin="$(normalize_named_arg deploy_bin "${6-auto}")"
state_dir="$(normalize_named_arg state_dir "${7-/var/lib/fishystuff/gitops-beta}")"
run_dir="$(normalize_named_arg run_dir "${8-/run/fishystuff/gitops-beta}")"
proof_max_age_seconds="$(normalize_named_arg proof_max_age_seconds "${9-86400}")"

cd "$RECIPE_REPO_ROOT"

if [[ -z "$operator_proof_file" ]]; then
  operator_proof_file="${FISHYSTUFF_GITOPS_BETA_OPERATOR_PROOF_FILE:-}"
fi

bash "${SCRIPT_DIR}/gitops-production-served-proof.sh" \
  "$output_dir" \
  "$draft_file" \
  "$summary_file" \
  "$admission_file" \
  "$operator_proof_file" \
  "$deploy_bin" \
  "$state_dir" \
  "$run_dir" \
  "$proof_max_age_seconds" \
  beta
