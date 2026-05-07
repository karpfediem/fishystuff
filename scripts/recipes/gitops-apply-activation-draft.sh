#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "${SCRIPT_DIR}/lib/common.sh"

draft_file="$(normalize_named_arg draft_file "${1-data/gitops/production-activation.draft.desired.json}")"
summary_file="$(normalize_named_arg summary_file "${2-data/gitops/production-current.handoff-summary.json}")"
admission_file="$(normalize_named_arg admission_file "${3-}")"
mgmt_bin="$(normalize_named_arg mgmt_bin "${4-auto}")"
deploy_bin="$(normalize_named_arg deploy_bin "${5-auto}")"
converged_timeout="$(normalize_named_arg converged_timeout "${6-45}")"

cd "$RECIPE_REPO_ROOT"

require_command() {
  local command_name="$1"
  if ! command -v "$command_name" >/dev/null; then
    echo "missing required command: ${command_name}" >&2
    exit 2
  fi
}

require_positive_int() {
  local name="$1"
  local value="$2"
  if [[ ! "$value" =~ ^[1-9][0-9]*$ ]]; then
    echo "$name must be a positive integer, got: ${value:-<empty>}" >&2
    exit 2
  fi
}

require_command jq
require_command sha256sum
require_positive_int converged_timeout "$converged_timeout"

if [[ "$draft_file" != /* ]]; then
  draft_file="${RECIPE_REPO_ROOT}/${draft_file}"
fi
if [[ "$summary_file" != /* ]]; then
  summary_file="${RECIPE_REPO_ROOT}/${summary_file}"
fi
if [[ -z "$admission_file" ]]; then
  admission_file="${FISHYSTUFF_GITOPS_ADMISSION_EVIDENCE_FILE:-}"
fi
if [[ -z "$admission_file" ]]; then
  echo "gitops-apply-activation-draft requires admission_file or FISHYSTUFF_GITOPS_ADMISSION_EVIDENCE_FILE" >&2
  exit 2
fi
if [[ "$admission_file" != /* ]]; then
  admission_file="${RECIPE_REPO_ROOT}/${admission_file}"
fi

if [[ "${FISHYSTUFF_GITOPS_ENABLE_PRODUCTION_APPLY:-}" != "1" ]]; then
  echo "gitops-apply-activation-draft requires FISHYSTUFF_GITOPS_ENABLE_PRODUCTION_APPLY=1" >&2
  exit 2
fi
if [[ "${FISHYSTUFF_GITOPS_ENABLE_LOCAL_APPLY:-}" != "1" ]]; then
  echo "gitops-apply-activation-draft requires FISHYSTUFF_GITOPS_ENABLE_LOCAL_APPLY=1" >&2
  exit 2
fi
if [[ -z "${FISHYSTUFF_GITOPS_APPLY_DRAFT_SHA256:-}" ]]; then
  echo "gitops-apply-activation-draft requires FISHYSTUFF_GITOPS_APPLY_DRAFT_SHA256 from gitops-review-activation-draft" >&2
  exit 2
fi

bash scripts/recipes/gitops-review-activation-draft.sh "$draft_file" "$summary_file" "$admission_file" "$deploy_bin"

read -r draft_sha256 _ < <(sha256sum "$draft_file")
if [[ "$FISHYSTUFF_GITOPS_APPLY_DRAFT_SHA256" != "$draft_sha256" ]]; then
  echo "FISHYSTUFF_GITOPS_APPLY_DRAFT_SHA256 does not match activation draft: expected ${draft_sha256}" >&2
  exit 2
fi

if [[ "$mgmt_bin" == "auto" ]]; then
  mgmt_flake="${FISHYSTUFF_GITOPS_MGMT_FLAKE:-git+file:///home/carp/code/mgmt-fishystuff-beta?rev=8ff41165c88368b84828ea2e37c24414be3f9532#minimal}"
  mgmt_out="$(nix build "$mgmt_flake" --no-link --print-out-paths)"
  mgmt_bin="${mgmt_out}/bin/mgmt"
fi

if [[ "$mgmt_bin" == */* && ! -x "$mgmt_bin" ]]; then
  echo "mgmt binary is missing or not executable: $mgmt_bin" >&2
  exit 2
fi

cd "$RECIPE_REPO_ROOT/gitops"
export FISHYSTUFF_GITOPS_STATE_FILE="$draft_file"

"$mgmt_bin" run --tmp-prefix --no-pgp lang --no-watch --converged-timeout "$converged_timeout" main.mcl

printf 'gitops_activation_apply_ok=%s\n' "$draft_file"
printf 'gitops_activation_apply_draft_sha256=%s\n' "$draft_sha256"
printf 'remote_deploy_performed=false\n'
printf 'infrastructure_mutation_performed=false\n'
