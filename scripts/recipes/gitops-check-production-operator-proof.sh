#!/usr/bin/env bash
set -euo pipefail

SCRIPT_PATH="$(readlink -f "${BASH_SOURCE[0]}")"
SCRIPT_DIR="$(cd "$(dirname "$SCRIPT_PATH")" && pwd)"
source "${SCRIPT_DIR}/lib/common.sh"

proof_file="$(normalize_named_arg proof_file "${1-}")"
max_age_seconds="$(normalize_named_arg max_age_seconds "${2-86400}")"
proof_dir="$(normalize_named_arg proof_dir "${3-data/gitops}")"

cd "$RECIPE_REPO_ROOT"

require_command() {
  local command_name="$1"
  if ! command -v "$command_name" >/dev/null 2>&1; then
    echo "missing required command: ${command_name}" >&2
    exit 127
  fi
}

absolute_path() {
  local path="$1"
  if [[ "$path" == /* ]]; then
    printf '%s' "$path"
    return
  fi
  printf '%s/%s' "$RECIPE_REPO_ROOT" "$path"
}

latest_proof_file() {
  local dir="$1"
  find "$dir" -maxdepth 1 -type f -name 'production-operator-proof.*.json' -printf '%T@ %p\n' \
    | sort -nr \
    | awk 'NR == 1 { $1 = ""; sub(/^ /, ""); print }'
}

require_current_sha256() {
  local label="$1"
  local path="$2"
  local expected="$3"
  local actual=""

  if [[ -z "$path" || -z "$expected" ]]; then
    echo "operator proof is missing ${label} path or sha256" >&2
    exit 2
  fi
  if [[ ! -f "$path" ]]; then
    echo "operator proof ${label} file does not exist: ${path}" >&2
    exit 2
  fi
  read -r actual _ < <(sha256sum "$path")
  if [[ "$actual" != "$expected" ]]; then
    echo "operator proof ${label}_sha256 does not match current file" >&2
    echo "proof:  ${expected}" >&2
    echo "actual: ${actual}" >&2
    echo "file:   ${path}" >&2
    exit 2
  fi
}

require_command awk
require_command date
require_command find
require_command jq
require_command sha256sum
require_command sort

case "$max_age_seconds" in
  '' | *[!0-9]*)
    echo "max_age_seconds must be a non-negative integer, got: ${max_age_seconds}" >&2
    exit 2
    ;;
esac

if [[ -z "$proof_file" ]]; then
  proof_dir="$(absolute_path "$proof_dir")"
  if [[ ! -d "$proof_dir" ]]; then
    echo "production operator proof directory does not exist: ${proof_dir}" >&2
    exit 2
  fi
  proof_file="$(latest_proof_file "$proof_dir")"
  if [[ -z "$proof_file" ]]; then
    echo "no production operator proof artifact found in ${proof_dir}" >&2
    exit 2
  fi
elif [[ "$proof_file" != /* ]]; then
  proof_file="${RECIPE_REPO_ROOT}/${proof_file}"
fi

if [[ ! -f "$proof_file" ]]; then
  echo "production operator proof does not exist: ${proof_file}" >&2
  exit 2
fi
read -r proof_sha256 _ < <(sha256sum "$proof_file")

if ! jq -e '
  .schema == "fishystuff.gitops.production-operator-proof.v1"
  and (.created_at | type == "string" and length > 0)
  and (.environment | type == "string" and length > 0)
  and .remote_deploy_performed == false
  and .infrastructure_mutation_performed == false
  and .commands.inventory.success == true
  and .commands.preflight.success == true
  and .commands.host_handoff_plan.success == true
  and .commands.inventory.kv.remote_deploy_performed == "false"
  and .commands.inventory.kv.infrastructure_mutation_performed == "false"
  and .commands.preflight.kv.remote_deploy_performed == "false"
  and .commands.preflight.kv.infrastructure_mutation_performed == "false"
  and .commands.host_handoff_plan.kv.remote_deploy_performed == "false"
  and .commands.host_handoff_plan.kv.infrastructure_mutation_performed == "false"
  and .commands.inventory.kv.edge_bundle_check_ok == "true"
  and .commands.inventory.kv.edge_caddy_validate == "true"
  and .commands.host_handoff_plan.kv.edge_caddy_validate == "true"
  ' "$proof_file" >/dev/null; then
  echo "production operator proof does not record the required successful local checks" >&2
  exit 2
fi

if ! jq -e '
  .environment as $environment
  | .inputs.draft_file as $draft
  | .inputs.summary_file as $summary
  | .inputs.admission_file as $admission
  | .commands.inventory.kv.gitops_production_host_inventory_ok == $environment
  and .commands.preflight.kv.gitops_production_preflight_ok == $draft
  and .commands.preflight.kv.handoff_summary == $summary
  and .commands.preflight.kv.admission_evidence == $admission
  and .commands.host_handoff_plan.kv.gitops_production_host_handoff_plan_ok == $draft
  and .commands.host_handoff_plan.kv.handoff_summary == $summary
  ' "$proof_file" >/dev/null; then
  echo "production operator proof command outputs do not match proof inputs" >&2
  exit 2
fi

created_at="$(jq -er '.created_at' "$proof_file")"
created_epoch="$(date -u -d "$created_at" '+%s' 2>/dev/null || true)"
if [[ -z "$created_epoch" ]]; then
  echo "production operator proof created_at is not parseable: ${created_at}" >&2
  exit 2
fi
now_epoch="$(date -u '+%s')"
age_seconds="$((now_epoch - created_epoch))"
if (( age_seconds < -300 )); then
  echo "production operator proof created_at is in the future: ${created_at}" >&2
  exit 2
fi
if (( max_age_seconds > 0 && age_seconds > max_age_seconds )); then
  echo "production operator proof is stale: age ${age_seconds}s exceeds max_age_seconds=${max_age_seconds}" >&2
  exit 2
fi

draft_file="$(jq -er '.inputs.draft_file' "$proof_file")"
summary_file="$(jq -er '.inputs.summary_file' "$proof_file")"
admission_file="$(jq -er '.inputs.admission_file' "$proof_file")"
draft_sha256="$(jq -er '.inputs.draft_sha256' "$proof_file")"
summary_sha256="$(jq -er '.inputs.summary_sha256' "$proof_file")"
admission_sha256="$(jq -er '.inputs.admission_sha256' "$proof_file")"

require_current_sha256 "draft" "$draft_file" "$draft_sha256"
require_current_sha256 "summary" "$summary_file" "$summary_sha256"
require_current_sha256 "admission" "$admission_file" "$admission_sha256"

printf 'gitops_production_operator_proof_check_ok=%s\n' "$proof_file"
printf 'gitops_production_operator_proof_sha256=%s\n' "$proof_sha256"
printf 'gitops_production_operator_proof_environment=%s\n' "$(jq -er '.environment' "$proof_file")"
printf 'gitops_production_operator_proof_created_at=%s\n' "$created_at"
printf 'gitops_production_operator_proof_age_seconds=%s\n' "$age_seconds"
printf 'gitops_production_operator_proof_max_age_seconds=%s\n' "$max_age_seconds"
printf 'gitops_production_operator_proof_draft_sha256=%s\n' "$draft_sha256"
printf 'gitops_production_operator_proof_summary_sha256=%s\n' "$summary_sha256"
printf 'gitops_production_operator_proof_admission_sha256=%s\n' "$admission_sha256"
printf 'remote_deploy_performed=false\n'
printf 'infrastructure_mutation_performed=false\n'
