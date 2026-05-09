#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "${SCRIPT_DIR}/lib/common.sh"

output="$(normalize_named_arg output "${1-data/gitops/beta-activation.draft.desired.json}")"
summary_file="$(normalize_named_arg summary_file "${2-data/gitops/beta-current.handoff-summary.json}")"
admission_file="$(normalize_named_arg admission_file "${3-}")"
mgmt_bin="$(normalize_named_arg mgmt_bin "${4-auto}")"
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
    echo "gitops-beta-activation-draft requires a beta handoff summary, got: ${environment}" >&2
    exit 2
  fi
fi

bash "${SCRIPT_DIR}/gitops-production-activation-draft.sh" \
  "$output" \
  "$summary_file" \
  "$admission_file" \
  "$mgmt_bin" \
  "$deploy_bin"
