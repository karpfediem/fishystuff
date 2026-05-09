#!/usr/bin/env bash
set -euo pipefail

SCRIPT_PATH="$(readlink -f "${BASH_SOURCE[0]}")"
SCRIPT_DIR="$(cd "$(dirname "$SCRIPT_PATH")" && pwd)"
source "${SCRIPT_DIR}/lib/common.sh"

summary_file="$(normalize_named_arg summary_file "${1-data/gitops/beta-current.handoff-summary.json}")"
admission_file="$(normalize_named_arg admission_file "${2-data/gitops/beta-admission.evidence.json}")"
draft_file="$(normalize_named_arg draft_file "${3-data/gitops/beta-activation.draft.desired.json}")"
proof_dir="$(normalize_named_arg proof_dir "${4-data/gitops}")"
api_bundle="$(normalize_named_arg api_bundle "${5-auto}")"
dolt_bundle="$(normalize_named_arg dolt_bundle "${6-auto}")"
edge_bundle="$(normalize_named_arg edge_bundle "${7-auto}")"
api_env_file="$(normalize_named_arg api_env_file "${8-/var/lib/fishystuff/gitops-beta/api/runtime.env}")"
dolt_env_file="$(normalize_named_arg dolt_env_file "${9-/var/lib/fishystuff/gitops-beta/dolt/beta.env}")"
api_upstream="$(normalize_named_arg api_upstream "${10-http://127.0.0.1:18192}")"
observation_dir="$(normalize_named_arg observation_dir "${11-data/gitops/beta-admission-observations}")"

cd "$RECIPE_REPO_ROOT"

kv_value() {
  local key="$1"
  local file="$2"
  awk -F= -v key="$key" '$1 == key { print substr($0, length(key) + 2); exit }' "$file"
}

require_kv_equals() {
  local key="$1"
  local file="$2"
  local expected="$3"
  local value=""

  value="$(kv_value "$key" "$file")"
  if [[ "$value" != "$expected" ]]; then
    echo "${key} expected ${expected}, got: ${value}" >&2
    exit 2
  fi
}

tmp_dir="$(mktemp -d)"
cleanup() {
  rm -rf "$tmp_dir"
}
trap cleanup EXIT

plan_output="${tmp_dir}/first-service-set-plan.out"
if ! bash scripts/recipes/gitops-beta-first-service-set-plan.sh \
  "$summary_file" \
  "$admission_file" \
  "$draft_file" \
  "$proof_dir" \
  "$api_bundle" \
  "$dolt_bundle" \
  "$edge_bundle" \
  "$api_env_file" \
  "$dolt_env_file" \
  "$api_upstream" \
  "$observation_dir" >"$plan_output" 2>&1; then
  cat "$plan_output" >&2 || true
  exit 2
fi

require_kv_equals remote_deploy_performed "$plan_output" false
require_kv_equals infrastructure_mutation_performed "$plan_output" false
require_kv_equals local_host_mutation_performed "$plan_output" false

printf 'gitops_beta_first_service_set_packet_ok=true\n'
awk -F= '
  $1 == "next_required_action" { print; next }
  $1 == "service_start_plan_status" { print; next }
  $1 ~ /^operator_packet_/ { print; next }
' "$plan_output"
printf 'remote_deploy_performed=false\n'
printf 'infrastructure_mutation_performed=false\n'
printf 'local_host_mutation_performed=false\n'
