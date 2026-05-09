#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "${SCRIPT_DIR}/lib/common.sh"

draft_file="$(normalize_named_arg draft_file "${1-data/gitops/beta-activation.draft.desired.json}")"
summary_file="$(normalize_named_arg summary_file "${2-data/gitops/beta-current.handoff-summary.json}")"
admission_file="$(normalize_named_arg admission_file "${3-}")"
deploy_bin="$(normalize_named_arg deploy_bin "${4-auto}")"
state_dir="$(normalize_named_arg state_dir "${5-/var/lib/fishystuff/gitops-beta}")"
run_dir="$(normalize_named_arg run_dir "${6-/run/fishystuff/gitops-beta}")"

cd "$RECIPE_REPO_ROOT"

summary_path="$summary_file"
if [[ "$summary_path" != /* ]]; then
  summary_path="${RECIPE_REPO_ROOT}/${summary_path}"
fi
if [[ -f "$summary_path" ]]; then
  if ! command -v jq >/dev/null; then
    echo "missing required command: jq" >&2
    exit 127
  fi
  environment="$(jq -er '.environment.name | select(type == "string" and length > 0)' "$summary_path")"
  if [[ "$environment" != "beta" ]]; then
    echo "gitops-beta-verify-activation-served requires a beta handoff summary, got: ${environment}" >&2
    exit 2
  fi
fi

bash "${SCRIPT_DIR}/gitops-verify-activation-served.sh" \
  "$draft_file" \
  "$summary_file" \
  "$admission_file" \
  "$deploy_bin" \
  "$state_dir" \
  "$run_dir"
