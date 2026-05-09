#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "${SCRIPT_DIR}/lib/common.sh"

proof_dir="$(normalize_named_arg proof_dir "${1-data/gitops}")"
max_age_seconds="$(normalize_named_arg max_age_seconds "${2-86400}")"
require_complete="$(normalize_named_arg require_complete "${3-false}")"

cd "$RECIPE_REPO_ROOT"

bash "${SCRIPT_DIR}/gitops-production-proof-index.sh" \
  "$proof_dir" \
  "$max_age_seconds" \
  "$require_complete" \
  beta
