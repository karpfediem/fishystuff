#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "${SCRIPT_DIR}/lib/common.sh"

draft_file="$(normalize_named_arg draft_file "${1-data/gitops/beta-activation.draft.desired.json}")"
summary_file="$(normalize_named_arg summary_file "${2-data/gitops/beta-current.handoff-summary.json}")"
admission_file="$(normalize_named_arg admission_file "${3-}")"
edge_bundle="$(normalize_named_arg edge_bundle "${4-auto}")"
deploy_bin="$(normalize_named_arg deploy_bin "${5-auto}")"

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
    echo "gitops-beta-host-handoff-plan requires a beta handoff summary, got: ${environment}" >&2
    exit 2
  fi
fi

FISHYSTUFF_GITOPS_ENVIRONMENT=beta \
  bash "${SCRIPT_DIR}/gitops-production-host-handoff-plan.sh" \
    "$draft_file" \
    "$summary_file" \
    "$admission_file" \
    "$edge_bundle" \
    "$deploy_bin" \
    beta
