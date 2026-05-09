#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "${SCRIPT_DIR}/lib/common.sh"

proof_file="$(normalize_named_arg proof_file "${1-}")"
max_age_seconds="$(normalize_named_arg max_age_seconds "${2-86400}")"
proof_dir="$(normalize_named_arg proof_dir "${3-data/gitops}")"

cd "$RECIPE_REPO_ROOT"

bash "${SCRIPT_DIR}/gitops-check-production-operator-proof.sh" \
  "$proof_file" \
  "$max_age_seconds" \
  "$proof_dir" \
  beta
