#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "${SCRIPT_DIR}/lib/common.sh"

output="$(normalize_named_arg output "${1-data/gitops/production-current.desired.json}")"
dolt_ref="$(normalize_named_arg dolt_ref "${2-main}")"
mgmt_bin="$(normalize_named_arg mgmt_bin "${3-auto}")"
deploy_bin="$(normalize_named_arg deploy_bin "${4-auto}")"

cd "$RECIPE_REPO_ROOT"

if [[ "$output" == "-" ]]; then
  echo "gitops-production-current-handoff requires a file output, not '-'" >&2
  exit 2
fi

if [[ -z "${FISHYSTUFF_GITOPS_RETAINED_RELEASES_FILE:-}" && -z "${FISHYSTUFF_GITOPS_RETAINED_RELEASES_JSON:-}" ]]; then
  echo "gitops-production-current-handoff requires FISHYSTUFF_GITOPS_RETAINED_RELEASES_FILE or FISHYSTUFF_GITOPS_RETAINED_RELEASES_JSON" >&2
  echo "derive it with: just gitops-retained-releases-json > /tmp/fishystuff-retained-releases.json" >&2
  exit 2
fi

state_file="$output"
if [[ "$state_file" != /* ]]; then
  state_file="${RECIPE_REPO_ROOT}/${state_file}"
fi

bash scripts/recipes/gitops-production-current-desired.sh "$output" "$dolt_ref"
bash scripts/recipes/gitops-check-desired-serving.sh "$deploy_bin" "$state_file" production
bash scripts/recipes/gitops-unify.sh "$mgmt_bin" "$state_file"

printf 'production_current_handoff_ready=%s\n' "$state_file" >&2
