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
operator_proof_file="$(normalize_named_arg proof_file "${7-}")"
operator_proof_max_age_seconds="$(normalize_named_arg proof_max_age_seconds "${8-86400}")"
environment="$(normalize_named_arg environment "${9-production}")"

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

case "$environment" in
  production)
    environment_apply_optin="FISHYSTUFF_GITOPS_ENABLE_PRODUCTION_APPLY"
    operator_proof_file_env="FISHYSTUFF_GITOPS_OPERATOR_PROOF_FILE"
    operator_proof_sha_env="FISHYSTUFF_GITOPS_APPLY_OPERATOR_PROOF_SHA256"
    operator_proof_check_script="scripts/recipes/gitops-check-production-operator-proof.sh"
    ;;
  beta)
    environment_apply_optin="FISHYSTUFF_GITOPS_ENABLE_BETA_APPLY"
    operator_proof_file_env="FISHYSTUFF_GITOPS_BETA_OPERATOR_PROOF_FILE"
    operator_proof_sha_env="FISHYSTUFF_GITOPS_BETA_APPLY_OPERATOR_PROOF_SHA256"
    operator_proof_check_script="scripts/recipes/gitops-check-beta-operator-proof.sh"
    ;;
  *)
    echo "unsupported GitOps activation apply environment: ${environment}" >&2
    exit 2
    ;;
esac

absolute_path() {
  local path="$1"
  if [[ "$path" == /* ]]; then
    printf '%s' "$path"
    return
  fi
  printf '%s/%s' "$RECIPE_REPO_ROOT" "$path"
}

if [[ "$draft_file" != /* ]]; then
  draft_file="$(absolute_path "$draft_file")"
fi
if [[ "$summary_file" != /* ]]; then
  summary_file="$(absolute_path "$summary_file")"
fi
if [[ -z "$admission_file" ]]; then
  admission_file="${FISHYSTUFF_GITOPS_ADMISSION_EVIDENCE_FILE:-}"
fi

if [[ "${!environment_apply_optin:-}" != "1" ]]; then
  echo "gitops-apply-activation-draft requires ${environment_apply_optin}=1" >&2
  exit 2
fi
if [[ "${FISHYSTUFF_GITOPS_ENABLE_LOCAL_APPLY:-}" != "1" ]]; then
  echo "gitops-apply-activation-draft requires FISHYSTUFF_GITOPS_ENABLE_LOCAL_APPLY=1" >&2
  exit 2
fi
if [[ "$environment" == "beta" ]]; then
  deployment_require_current_hostname_match beta gitops-beta-apply-activation-draft
fi
if [[ -z "$operator_proof_file" ]]; then
  operator_proof_file="${!operator_proof_file_env:-}"
fi
if [[ -z "$operator_proof_file" ]]; then
  echo "gitops-apply-activation-draft requires proof_file or ${operator_proof_file_env}" >&2
  exit 2
fi
operator_proof_file="$(absolute_path "$operator_proof_file")"
if [[ ! -f "$operator_proof_file" ]]; then
  echo "${environment} operator proof does not exist: ${operator_proof_file}" >&2
  exit 2
fi
operator_proof_sha_expected="${!operator_proof_sha_env:-}"
if [[ -z "$operator_proof_sha_expected" ]]; then
  echo "gitops-apply-activation-draft requires ${operator_proof_sha_env} from gitops-check-${environment}-operator-proof" >&2
  exit 2
fi

read -r operator_proof_sha256 _ < <(sha256sum "$operator_proof_file")
if [[ "$operator_proof_sha_expected" != "$operator_proof_sha256" ]]; then
  echo "${operator_proof_sha_env} does not match operator proof: expected ${operator_proof_sha256}" >&2
  exit 2
fi

proof_check_output="$(mktemp)"
cleanup() {
  rm -f "$proof_check_output"
}
trap cleanup EXIT

summary_environment="$(jq -er '.environment.name | select(type == "string" and length > 0)' "$summary_file")"
if [[ "$summary_environment" != "$environment" ]]; then
  echo "activation apply environment does not match handoff summary" >&2
  echo "apply:   ${environment}" >&2
  echo "summary: ${summary_environment}" >&2
  exit 2
fi

proof_environment="$(jq -er '.environment | select(type == "string" and length > 0)' "$operator_proof_file")"
if [[ "$proof_environment" != "$environment" ]]; then
  echo "activation apply environment does not match operator proof" >&2
  echo "apply: ${environment}" >&2
  echo "proof: ${proof_environment}" >&2
  exit 2
fi

bash "$operator_proof_check_script" "$operator_proof_file" "$operator_proof_max_age_seconds" "" >"$proof_check_output"

proof_draft_file="$(absolute_path "$(jq -er '.inputs.draft_file' "$operator_proof_file")")"
proof_summary_file="$(absolute_path "$(jq -er '.inputs.summary_file' "$operator_proof_file")")"
proof_admission_file="$(absolute_path "$(jq -er '.inputs.admission_file' "$operator_proof_file")")"
proof_draft_sha256="$(jq -er '.inputs.draft_sha256' "$operator_proof_file")"
proof_summary_sha256="$(jq -er '.inputs.summary_sha256' "$operator_proof_file")"
proof_admission_sha256="$(jq -er '.inputs.admission_sha256' "$operator_proof_file")"

if [[ -z "$admission_file" ]]; then
  admission_file="$proof_admission_file"
fi
if [[ "$admission_file" != /* ]]; then
  admission_file="$(absolute_path "$admission_file")"
fi

if [[ "$draft_file" != "$proof_draft_file" ]]; then
  echo "operator proof draft_file does not match activation draft" >&2
  echo "apply: ${draft_file}" >&2
  echo "proof: ${proof_draft_file}" >&2
  exit 2
fi
if [[ "$summary_file" != "$proof_summary_file" ]]; then
  echo "operator proof summary_file does not match handoff summary" >&2
  echo "apply: ${summary_file}" >&2
  echo "proof: ${proof_summary_file}" >&2
  exit 2
fi
if [[ "$admission_file" != "$proof_admission_file" ]]; then
  echo "operator proof admission_file does not match admission evidence" >&2
  echo "apply: ${admission_file}" >&2
  echo "proof: ${proof_admission_file}" >&2
  exit 2
fi

bash scripts/recipes/gitops-review-activation-draft.sh "$draft_file" "$summary_file" "$admission_file" "$deploy_bin"

read -r draft_sha256 _ < <(sha256sum "$draft_file")
read -r summary_sha256 _ < <(sha256sum "$summary_file")
read -r admission_sha256 _ < <(sha256sum "$admission_file")
if [[ "$draft_sha256" != "$proof_draft_sha256" ]]; then
  echo "operator proof draft_sha256 does not match activation draft: expected ${draft_sha256}" >&2
  exit 2
fi
if [[ "$summary_sha256" != "$proof_summary_sha256" ]]; then
  echo "operator proof summary_sha256 does not match handoff summary: expected ${summary_sha256}" >&2
  exit 2
fi
if [[ "$admission_sha256" != "$proof_admission_sha256" ]]; then
  echo "operator proof admission_sha256 does not match admission evidence: expected ${admission_sha256}" >&2
  exit 2
fi

if [[ "$mgmt_bin" == "auto" ]]; then
  mgmt_flake="${FISHYSTUFF_GITOPS_MGMT_FLAKE:-${RECIPE_REPO_ROOT}#mgmt-gitops}"
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
printf 'gitops_activation_apply_operator_proof=%s\n' "$operator_proof_file"
printf 'gitops_activation_apply_operator_proof_sha256=%s\n' "$operator_proof_sha256"
printf 'gitops_activation_apply_draft_sha256=%s\n' "$draft_sha256"
printf 'gitops_activation_apply_environment=%s\n' "$environment"
printf 'gitops_%s_activation_apply_ok=%s\n' "$environment" "$draft_file"
printf 'gitops_%s_activation_apply_operator_proof=%s\n' "$environment" "$operator_proof_file"
printf 'gitops_%s_activation_apply_operator_proof_sha256=%s\n' "$environment" "$operator_proof_sha256"
printf 'gitops_%s_activation_apply_draft_sha256=%s\n' "$environment" "$draft_sha256"
printf 'remote_deploy_performed=false\n'
printf 'infrastructure_mutation_performed=false\n'
