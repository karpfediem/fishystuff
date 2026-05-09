#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "${SCRIPT_DIR}/lib/common.sh"

output="$(normalize_named_arg output "${1-data/gitops/beta-admission.evidence.json}")"
summary_file="$(normalize_named_arg summary_file "${2-data/gitops/beta-current.handoff-summary.json}")"
api_upstream="$(normalize_named_arg api_upstream "${3-}")"
api_meta_source="$(normalize_named_arg api_meta_source "${4-}")"
db_probe_file="$(normalize_named_arg db_probe_file "${5-}")"
site_cdn_probe_file="$(normalize_named_arg site_cdn_probe_file "${6-}")"

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
    echo "gitops-beta-write-activation-admission-evidence requires a beta handoff summary, got: ${environment}" >&2
    exit 2
  fi
fi

bash "${SCRIPT_DIR}/gitops-write-activation-admission-evidence.sh" \
  "$output" \
  "$summary_file" \
  "$api_upstream" \
  "$api_meta_source" \
  "$db_probe_file" \
  "$site_cdn_probe_file"
