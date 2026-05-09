#!/usr/bin/env bash
set -euo pipefail

SCRIPT_PATH="$(readlink -f "${BASH_SOURCE[0]}")"
SCRIPT_DIR="$(cd "$(dirname "$SCRIPT_PATH")" && pwd)"
source "${SCRIPT_DIR}/lib/common.sh"

cd "$RECIPE_REPO_ROOT"

pass_count=0

pass() {
  printf '[gitops-beta-first-service-set-packet-test] pass: %s\n' "$1"
  pass_count="$((pass_count + 1))"
}

root="$(mktemp -d)"
pending_summary="${root}/missing-summary.json"
pending_admission="${root}/missing-admission.json"
pending_draft="${root}/missing-draft.json"
pending_proofs="${root}/missing-proofs"

bash scripts/recipes/gitops-beta-first-service-set-packet.sh \
  "$pending_summary" \
  "$pending_admission" \
  "$pending_draft" \
  "$pending_proofs" \
  auto \
  auto \
  auto \
  "${root}/api/runtime.env" \
  "${root}/dolt/beta.env" \
  "http://127.0.0.1:18192" \
  "${root}/observations" \
  >"${root}/packet.stdout"

grep -F "gitops_beta_first_service_set_packet_ok=true" "${root}/packet.stdout" >/dev/null
grep -F "next_required_action=generate_current_handoff" "${root}/packet.stdout" >/dev/null
grep -F "service_start_plan_status=pending_explicit_bundles" "${root}/packet.stdout" >/dev/null
grep -F "operator_packet_status=generate_current_handoff" "${root}/packet.stdout" >/dev/null
grep -F "operator_packet_next_command_01=FISHYSTUFF_OPERATOR_ROOT=${RECIPE_REPO_ROOT} just gitops-beta-current-handoff summary_output=${pending_summary}" "${root}/packet.stdout" >/dev/null
grep -F "remote_deploy_performed=false" "${root}/packet.stdout" >/dev/null
grep -F "infrastructure_mutation_performed=false" "${root}/packet.stdout" >/dev/null
grep -F "local_host_mutation_performed=false" "${root}/packet.stdout" >/dev/null

if grep -E '^(phase_|read_only_step_|guarded_host_action_|guarded_runtime_env_action_)' "${root}/packet.stdout" >/dev/null; then
  printf '[gitops-beta-first-service-set-packet-test] packet leaked full runbook lines\n' >&2
  exit 1
fi
pass "pending packet"

printf '[gitops-beta-first-service-set-packet-test] %s checks passed\n' "$pass_count"
