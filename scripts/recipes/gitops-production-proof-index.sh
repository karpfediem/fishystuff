#!/usr/bin/env bash
set -euo pipefail

SCRIPT_PATH="$(readlink -f "${BASH_SOURCE[0]}")"
SCRIPT_DIR="$(cd "$(dirname "$SCRIPT_PATH")" && pwd)"
source "${SCRIPT_DIR}/lib/common.sh"

proof_dir="$(normalize_named_arg proof_dir "${1-data/gitops}")"
max_age_seconds="$(normalize_named_arg max_age_seconds "${2-86400}")"
require_complete="$(normalize_named_arg require_complete "${3-false}")"
environment="$(normalize_named_arg environment "${4-${FISHYSTUFF_GITOPS_ENVIRONMENT:-production}}")"

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

latest_file() {
  local dir="$1"
  local pattern="$2"

  find "$dir" -maxdepth 1 -type f -name "$pattern" -printf '%T@ %p\n' \
    | sort -nr \
    | awk 'NR == 1 { $1 = ""; sub(/^ /, ""); print }'
}

file_sha256_or_empty() {
  local path="$1"
  local sha=""
  if [[ -f "$path" ]]; then
    read -r sha _ < <(sha256sum "$path")
  fi
  printf '%s' "$sha"
}

created_epoch_or_empty() {
  local created_at="$1"
  date -u -d "$created_at" '+%s' 2>/dev/null || true
}

age_seconds_or_empty() {
  local created_at="$1"
  local created_epoch=""
  local now_epoch=""

  created_epoch="$(created_epoch_or_empty "$created_at")"
  if [[ -z "$created_epoch" ]]; then
    printf ''
    return
  fi
  now_epoch="$(date -u '+%s')"
  printf '%s' "$((now_epoch - created_epoch))"
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
case "$require_complete" in
  true | false) ;;
  *)
    echo "require_complete must be true or false, got: ${require_complete}" >&2
    exit 2
    ;;
esac
case "$environment" in
  production | beta) ;;
  *)
    echo "unsupported GitOps proof index environment: ${environment}" >&2
    exit 2
    ;;
esac
index_key_prefix="gitops_${environment}_proof_index"
served_proof_schema="fishystuff.gitops.${environment}-served-proof.v1"

proof_dir="$(absolute_path "$proof_dir")"
if [[ ! -d "$proof_dir" ]]; then
  printf 'gitops_proof_index_status=missing_proof_dir\n'
  printf 'gitops_proof_index_dir=%s\n' "$proof_dir"
  printf 'gitops_proof_index_complete=false\n'
  printf '%s_status=missing_proof_dir\n' "$index_key_prefix"
  printf '%s_dir=%s\n' "$index_key_prefix" "$proof_dir"
  printf '%s_complete=false\n' "$index_key_prefix"
  printf 'remote_deploy_performed=false\n'
  printf 'infrastructure_mutation_performed=false\n'
  if [[ "$require_complete" == "true" ]]; then
    exit 2
  fi
  exit 0
fi

operator_proof="$(latest_file "$proof_dir" "${environment}-operator-proof.*.json")"
served_proof="$(latest_file "$proof_dir" "${environment}-served-proof.*.json")"
status="complete"
complete="true"

operator_created_at=""
operator_age_seconds=""
operator_sha256=""
operator_check_status="missing"
operator_draft_file=""
operator_summary_file=""
operator_admission_file=""
operator_release_id=""

served_created_at=""
served_age_seconds=""
served_sha256=""
served_schema_status="missing"
served_operator_proof_file=""
served_operator_proof_sha256=""
served_release_id=""
served_generation=""
served_link_status="missing_served_proof"

operator_check_output="$(mktemp)"
operator_check_stderr="$(mktemp)"
cleanup() {
  rm -f "$operator_check_output" "$operator_check_stderr"
}
trap cleanup EXIT

if [[ -z "$operator_proof" ]]; then
  status="missing_operator_proof"
  complete="false"
else
  operator_sha256="$(file_sha256_or_empty "$operator_proof")"
  operator_created_at="$(jq -er '.created_at // ""' "$operator_proof" 2>/dev/null || true)"
  operator_age_seconds="$(age_seconds_or_empty "$operator_created_at")"
  operator_draft_file="$(jq -er '.inputs.draft_file // ""' "$operator_proof" 2>/dev/null || true)"
  operator_summary_file="$(jq -er '.inputs.summary_file // ""' "$operator_proof" 2>/dev/null || true)"
  operator_admission_file="$(jq -er '.inputs.admission_file // ""' "$operator_proof" 2>/dev/null || true)"
  operator_release_id="$(jq -er '.commands.host_handoff_plan.kv.release_id // ""' "$operator_proof" 2>/dev/null || true)"
  if bash scripts/recipes/gitops-check-production-operator-proof.sh "$operator_proof" "$max_age_seconds" "" "$environment" >"$operator_check_output" 2>"$operator_check_stderr"; then
    operator_check_status="passed"
  else
    operator_check_status="failed"
    status="operator_proof_failed"
    complete="false"
  fi
fi

if [[ -z "$served_proof" ]]; then
  if [[ "$complete" == "true" ]]; then
    status="missing_served_proof"
  fi
  complete="false"
else
  served_sha256="$(file_sha256_or_empty "$served_proof")"
  served_created_at="$(jq -er '.created_at // ""' "$served_proof" 2>/dev/null || true)"
  served_age_seconds="$(age_seconds_or_empty "$served_created_at")"
  served_operator_proof_file="$(jq -er '.inputs.operator_proof_file // ""' "$served_proof" 2>/dev/null || true)"
  served_operator_proof_sha256="$(jq -er '.inputs.operator_proof_sha256 // ""' "$served_proof" 2>/dev/null || true)"
  served_release_id="$(jq -er '.served.release_id // ""' "$served_proof" 2>/dev/null || true)"
  served_generation="$(jq -er '.served.generation // ""' "$served_proof" 2>/dev/null || true)"
  if jq -e \
    --arg schema "$served_proof_schema" \
    --arg environment "$environment" \
    '
    .schema == $schema
    and .environment == $environment
    and .remote_deploy_performed == false
    and .infrastructure_mutation_performed == false
    and .commands.operator_proof_check.success == true
    and .commands.served_verification.success == true
    and (.inputs.operator_proof_file | type == "string" and length > 0)
    and (.inputs.operator_proof_sha256 | type == "string" and length > 0)
    ' "$served_proof" >/dev/null; then
    served_schema_status="passed"
  else
    served_schema_status="failed"
    if [[ "$complete" == "true" ]]; then
      status="served_proof_failed"
    fi
    complete="false"
  fi

  if [[ -n "$operator_proof" && "$served_schema_status" == "passed" ]]; then
    if [[ "$served_operator_proof_file" == "$operator_proof" && "$served_operator_proof_sha256" == "$operator_sha256" ]]; then
      served_link_status="matches_latest_operator_proof"
    else
      served_link_status="stale_or_mismatched_operator_proof"
      if [[ "$complete" == "true" ]]; then
        status="served_proof_not_linked_to_latest_operator"
      fi
      complete="false"
    fi
  fi
fi

if [[ "$operator_check_status" == "passed" && "$served_schema_status" == "passed" && "$served_link_status" == "matches_latest_operator_proof" ]]; then
  status="complete"
  complete="true"
fi

print_index_field() {
  local key="$1"
  local value="$2"
  printf 'gitops_proof_index_%s=%s\n' "$key" "$value"
  printf '%s_%s=%s\n' "$index_key_prefix" "$key" "$value"
}

print_index_field status "$status"
print_index_field complete "$complete"
print_index_field dir "$proof_dir"
print_index_field environment "$environment"
print_index_field operator_proof "$operator_proof"
print_index_field operator_proof_sha256 "$operator_sha256"
print_index_field operator_check "$operator_check_status"
print_index_field operator_created_at "$operator_created_at"
print_index_field operator_age_seconds "$operator_age_seconds"
print_index_field operator_draft_file "$operator_draft_file"
print_index_field operator_summary_file "$operator_summary_file"
print_index_field operator_admission_file "$operator_admission_file"
print_index_field operator_release_id "$operator_release_id"
print_index_field served_proof "$served_proof"
print_index_field served_proof_sha256 "$served_sha256"
print_index_field served_schema "$served_schema_status"
print_index_field served_link "$served_link_status"
print_index_field served_created_at "$served_created_at"
print_index_field served_age_seconds "$served_age_seconds"
print_index_field served_release_id "$served_release_id"
print_index_field served_generation "$served_generation"
print_index_field served_operator_proof "$served_operator_proof_file"
print_index_field served_operator_proof_sha256 "$served_operator_proof_sha256"
printf 'remote_deploy_performed=false\n'
printf 'infrastructure_mutation_performed=false\n'

if [[ "$require_complete" == "true" && "$complete" != "true" ]]; then
  exit 2
fi
