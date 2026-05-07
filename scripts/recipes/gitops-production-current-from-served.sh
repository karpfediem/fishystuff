#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "${SCRIPT_DIR}/lib/common.sh"

output="$(normalize_named_arg output "${1-data/gitops/production-current.desired.json}")"
state_dir="$(normalize_named_arg state_dir "${2-/var/lib/fishystuff/gitops}")"
environment="$(normalize_named_arg environment "${3-production}")"
retained_output="$(normalize_named_arg retained_output "${4-}")"
dolt_ref="$(normalize_named_arg dolt_ref "${5-main}")"
mgmt_bin="$(normalize_named_arg mgmt_bin "${6-auto}")"
deploy_bin="$(normalize_named_arg deploy_bin "${7-auto}")"
summary_output="$(normalize_named_arg summary_output "${8-}")"

cd "$RECIPE_REPO_ROOT"

if [[ "$output" == "-" ]]; then
  echo "gitops-production-current-from-served requires a file output, not '-'" >&2
  exit 2
fi

if [[ "$environment" != "production" ]]; then
  echo "gitops-production-current-from-served currently supports only environment=production" >&2
  exit 2
fi

state_file="$output"
if [[ "$state_file" != /* ]]; then
  state_file="${RECIPE_REPO_ROOT}/${state_file}"
fi

retained_file="$retained_output"
if [[ -z "$retained_file" ]]; then
  retained_file="${state_file%.desired.json}.retained-releases.json"
  if [[ "$retained_file" == "$state_file" ]]; then
    retained_file="${state_file}.retained-releases.json"
  fi
elif [[ "$retained_file" != /* ]]; then
  retained_file="${RECIPE_REPO_ROOT}/${retained_file}"
fi

mkdir -p "$(dirname "$retained_file")"
tmp="$(mktemp "$(dirname "$retained_file")/.${retained_file##*/}.XXXXXX")"
bash scripts/recipes/gitops-retained-releases-json.sh \
  "$deploy_bin" \
  "$environment" \
  "$state_dir" \
  >"$tmp"
mv "$tmp" "$retained_file"

FISHYSTUFF_GITOPS_RETAINED_RELEASES_FILE="$retained_file" \
  bash scripts/recipes/gitops-production-current-handoff.sh \
    "$output" \
    "$dolt_ref" \
    "$mgmt_bin" \
    "$deploy_bin" \
    "$summary_output"

printf 'production_current_retained_releases=%s\n' "$retained_file" >&2
