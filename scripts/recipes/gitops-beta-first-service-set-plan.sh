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

file_sha256_or_empty() {
  local path="$1"
  local sha=""

  if [[ -f "$path" ]]; then
    read -r sha _ < <(sha256sum "$path")
  fi
  printf '%s' "$sha"
}

print_file_status() {
  local label="$1"
  local path="$2"
  local status="missing"
  local sha=""

  if [[ -f "$path" ]]; then
    status="present"
    sha="$(file_sha256_or_empty "$path")"
  fi
  printf '%s_path=%s\n' "$label" "$path"
  printf '%s_status=%s\n' "$label" "$status"
  if [[ -n "$sha" ]]; then
    printf '%s_sha256=%s\n' "$label" "$sha"
  fi
}

require_beta_summary() {
  local path="$1"
  local environment=""

  environment="$(jq -er '.environment.name | select(type == "string" and length > 0)' "$path")"
  if [[ "$environment" != "beta" ]]; then
    echo "beta first service-set plan requires a beta handoff summary, got: ${environment}" >&2
    exit 2
  fi
}

require_beta_admission() {
  local path="$1"
  local environment=""

  environment="$(jq -er '.environment | select(type == "string" and length > 0)' "$path")"
  if [[ "$environment" != "beta" ]]; then
    echo "beta first service-set plan requires beta admission evidence, got: ${environment}" >&2
    exit 2
  fi
}

require_beta_draft() {
  local path="$1"

  if ! jq -e '
    .cluster == "beta"
    and .mode == "local-apply"
    and (.environments.beta.enabled == true)
    and (.environments.beta.serve == true)
    and (.environments.beta.active_release | type == "string" and length > 0)
    and (.environments.beta.api_upstream | type == "string" and length > 0)
    and (.environments.beta.admission_probe.kind == "api_meta")
  ' "$path" >/dev/null; then
    echo "beta first service-set plan requires a beta local-apply activation draft" >&2
    exit 2
  fi
}

run_service_start_plan_if_ready() {
  local output="$1"

  if [[ "$api_bundle" == "auto" || "$dolt_bundle" == "auto" ]]; then
    printf 'service_start_plan_status=pending_explicit_bundles\n'
    return
  fi
  if [[ ! -f "$api_env_file" || ! -f "$dolt_env_file" ]]; then
    printf 'service_start_plan_status=pending_runtime_env\n'
    return
  fi

  bash scripts/recipes/gitops-beta-service-start-plan.sh \
    "$api_bundle" \
    "$dolt_bundle" \
    "$api_env_file" \
    "$dolt_env_file" >"$output"
  require_kv_equals remote_deploy_performed "$output" false
  require_kv_equals infrastructure_mutation_performed "$output" false
  require_kv_equals local_host_mutation_performed "$output" false
  printf 'service_start_plan_status=ready\n'
}

require_command awk
require_command jq
require_command mktemp
require_command sha256sum

if [[ "$api_upstream" == */ ]]; then
  echo "api_upstream must not end with /" >&2
  exit 2
fi
require_loopback_http_url api_upstream "$api_upstream"

summary_file="$(absolute_path "$summary_file")"
admission_file="$(absolute_path "$admission_file")"
draft_file="$(absolute_path "$draft_file")"
proof_dir="$(absolute_path "$proof_dir")"
observation_dir="$(absolute_path "$observation_dir")"
if [[ "$api_env_file" != /* ]]; then
  api_env_file="$(absolute_path "$api_env_file")"
fi
if [[ "$dolt_env_file" != /* ]]; then
  dolt_env_file="$(absolute_path "$dolt_env_file")"
fi

tmp_dir="$(mktemp -d)"
cleanup() {
  rm -rf "$tmp_dir"
}
trap cleanup EXIT

bootstrap_plan="${tmp_dir}/bootstrap-plan.out"
current_handoff_plan="${tmp_dir}/current-handoff-plan.out"
service_start_plan="${tmp_dir}/service-start-plan.out"
proof_index="${tmp_dir}/proof-index.out"
: >"$service_start_plan"

bash scripts/recipes/gitops-beta-host-bootstrap-plan.sh >"$bootstrap_plan"
require_kv_equals remote_deploy_performed "$bootstrap_plan" false
require_kv_equals infrastructure_mutation_performed "$bootstrap_plan" false
require_kv_equals local_host_mutation_performed "$bootstrap_plan" false

current_desired_file="${summary_file%.handoff-summary.json}.desired.json"
if [[ "$current_desired_file" == "$summary_file" ]]; then
  current_desired_file="${summary_file}.desired.json"
fi
FISHYSTUFF_GITOPS_ENVIRONMENT=beta \
  FISHYSTUFF_GITOPS_CLUSTER=beta \
  bash scripts/recipes/gitops-beta-current-handoff-plan.sh \
    "$current_desired_file" \
    beta \
    auto \
    auto \
    "$summary_file" >"$current_handoff_plan"

printf 'gitops_beta_first_service_set_plan_ok=true\n'
printf 'environment=beta\n'
printf 'api_upstream=%s\n' "$api_upstream"
printf 'edge_bundle=%s\n' "$edge_bundle"
printf 'api_bundle=%s\n' "$api_bundle"
printf 'dolt_bundle=%s\n' "$dolt_bundle"
printf 'api_env_file=%s\n' "$api_env_file"
printf 'dolt_env_file=%s\n' "$dolt_env_file"

printf 'host_bootstrap_plan_status=ready\n'
awk -F= '$1 ~ /^(gitops_beta_current_handoff_plan_ok|handoff_plan_status|handoff_can_run|closure_build_required|mgmt_build_required|cdn_runtime_closure_status|cdn_runtime_operator_root_status|dolt_commit_status|dolt_remote_status)$/ { print }' "$current_handoff_plan"
run_service_start_plan_if_ready "$service_start_plan"

summary_status="missing"
if [[ -f "$summary_file" ]]; then
  require_beta_summary "$summary_file"
  bash scripts/recipes/gitops-check-handoff-summary.sh "$summary_file" >/dev/null
  summary_status="ready"
fi
printf 'handoff_summary_status=%s\n' "$summary_status"
print_file_status handoff_summary "$summary_file"

admission_status="missing"
if [[ -f "$admission_file" ]]; then
  require_beta_admission "$admission_file"
  admission_status="ready"
fi
printf 'admission_evidence_status=%s\n' "$admission_status"
print_file_status admission_evidence "$admission_file"

draft_status="missing"
if [[ -f "$draft_file" ]]; then
  require_beta_draft "$draft_file"
  draft_status="ready"
fi
printf 'activation_draft_status=%s\n' "$draft_status"
print_file_status activation_draft "$draft_file"

if bash scripts/recipes/gitops-beta-proof-index.sh "$proof_dir" 86400 false >"$proof_index"; then
  awk -F= '$1 ~ /^(gitops_proof_index_|gitops_beta_proof_index_)/ { print }' "$proof_index"
else
  echo "beta proof index read failed" >&2
  exit 2
fi

api_unit_sha256="$(kv_value gitops_beta_service_start_plan_api_unit_sha256 "$service_start_plan")"
dolt_unit_sha256="$(kv_value gitops_beta_service_start_plan_dolt_unit_sha256 "$service_start_plan")"
if [[ -z "$api_unit_sha256" ]]; then
  api_unit_sha256="<checked beta API unit hash>"
fi
if [[ -z "$dolt_unit_sha256" ]]; then
  dolt_unit_sha256="<checked beta Dolt unit hash>"
fi

printf 'phase_01=bootstrap host-local beta directories and beta Dolt user/group\n'
printf 'phase_02=write beta runtime env files and start beta Dolt before beta API\n'
printf 'phase_03=generate beta current desired state and handoff summary\n'
printf 'phase_04=observe loopback beta admission evidence\n'
printf 'phase_05=write activation draft and beta operator proof\n'
printf 'phase_06=apply activation draft through mgmt and publish served proof/index\n'
printf 'phase_07=install beta edge only after complete served proof chain\n'
printf 'read_only_step_01=just gitops-beta-host-bootstrap-plan\n'
printf 'read_only_step_02=just gitops-beta-service-start-plan api_bundle=%s dolt_bundle=%s api_env_file=%s dolt_env_file=%s\n' "$api_bundle" "$dolt_bundle" "$api_env_file" "$dolt_env_file"
printf 'read_only_step_03=just gitops-beta-current-handoff summary_output=%s\n' "$summary_file"
printf 'read_only_step_04=just gitops-beta-observe-admission output=%s summary_file=%s api_upstream=%s observation_dir=%s\n' "$admission_file" "$summary_file" "$api_upstream" "$observation_dir"
printf 'read_only_step_05=just gitops-beta-activation-draft output=%s summary_file=%s admission_file=%s\n' "$draft_file" "$summary_file" "$admission_file"
printf 'read_only_step_06=just gitops-beta-operator-proof output_dir=%s draft_file=%s summary_file=%s admission_file=%s edge_bundle=%s\n' "$proof_dir" "$draft_file" "$summary_file" "$admission_file" "$edge_bundle"
printf 'read_only_step_07=just gitops-beta-proof-index proof_dir=%s require_complete=true\n' "$proof_dir"
printf 'guarded_host_action_01=FISHYSTUFF_GITOPS_ENABLE_BETA_HOST_BOOTSTRAP=1 FISHYSTUFF_GITOPS_ENABLE_BETA_HOST_DIRECTORIES=1 FISHYSTUFF_GITOPS_ENABLE_BETA_HOST_USER_GROUPS=1 just gitops-beta-host-bootstrap-apply\n'
printf 'guarded_host_action_02=FISHYSTUFF_GITOPS_ENABLE_BETA_SERVICE_START=1 FISHYSTUFF_GITOPS_ENABLE_BETA_DOLT_INSTALL=1 FISHYSTUFF_GITOPS_ENABLE_BETA_DOLT_RESTART=1 FISHYSTUFF_GITOPS_ENABLE_BETA_API_INSTALL=1 FISHYSTUFF_GITOPS_ENABLE_BETA_API_RESTART=1 FISHYSTUFF_GITOPS_BETA_DOLT_UNIT_SHA256=%s FISHYSTUFF_GITOPS_BETA_API_UNIT_SHA256=%s just gitops-beta-start-services api_bundle=%s dolt_bundle=%s api_env_file=%s dolt_env_file=%s\n' "$dolt_unit_sha256" "$api_unit_sha256" "$api_bundle" "$dolt_bundle" "$api_env_file" "$dolt_env_file"
printf 'guarded_host_action_03=FISHYSTUFF_GITOPS_ENABLE_BETA_APPLY=1 FISHYSTUFF_GITOPS_ENABLE_LOCAL_APPLY=1 FISHYSTUFF_GITOPS_BETA_APPLY_OPERATOR_PROOF_SHA256=<checked beta proof hash> just gitops-beta-apply-activation-draft draft_file=%s summary_file=%s admission_file=%s proof_file=<checked beta operator proof file>\n' "$draft_file" "$summary_file" "$admission_file"
printf 'guarded_host_action_04=FISHYSTUFF_GITOPS_ENABLE_BETA_EDGE_INSTALL=1 FISHYSTUFF_GITOPS_ENABLE_BETA_EDGE_RESTART=1 FISHYSTUFF_GITOPS_BETA_EDGE_SERVED_PROOF_SHA256=<checked beta served proof hash> FISHYSTUFF_GITOPS_BETA_EDGE_UNIT_SHA256=<checked beta edge unit hash> just gitops-beta-install-edge edge_bundle=%s proof_dir=%s\n' "$edge_bundle" "$proof_dir"
printf 'post_apply_read_only_step_01=just gitops-beta-verify-activation-served draft_file=%s summary_file=%s admission_file=%s\n' "$draft_file" "$summary_file" "$admission_file"
printf 'post_apply_read_only_step_02=just gitops-beta-served-proof draft_file=%s summary_file=%s admission_file=%s proof_file=<checked beta operator proof file>\n' "$draft_file" "$summary_file" "$admission_file"
printf 'post_apply_read_only_step_03=just gitops-beta-proof-index proof_dir=%s require_complete=true\n' "$proof_dir"
printf 'remote_deploy_performed=false\n'
printf 'infrastructure_mutation_performed=false\n'
printf 'local_host_mutation_performed=false\n'
