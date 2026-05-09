#!/usr/bin/env bash
set -euo pipefail

SCRIPT_PATH="$(readlink -f "${BASH_SOURCE[0]}")"
SCRIPT_DIR="$(cd "$(dirname "$SCRIPT_PATH")" && pwd)"
source "${SCRIPT_DIR}/lib/common.sh"

service="$(normalize_named_arg service "${1-api}")"
output="$(normalize_named_arg output "${2-}")"
profile="$(normalize_named_arg profile "${3-beta-runtime}")"

cd "$RECIPE_REPO_ROOT"

if [[ "$profile" != "beta-runtime" ]]; then
  echo "gitops-beta-write-runtime-env-secretspec requires profile=beta-runtime" >&2
  exit 2
fi

if [[ "$service" != "api" ]]; then
  echo "gitops-beta-write-runtime-env-secretspec only supports service=api; write the Dolt env with gitops-beta-write-runtime-env" >&2
  exit 2
fi

exec secretspec run --profile "$profile" -- bash scripts/recipes/gitops-beta-write-runtime-env.sh "$service" "$output"
