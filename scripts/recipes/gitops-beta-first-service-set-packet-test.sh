#!/usr/bin/env bash
set -euo pipefail

SCRIPT_PATH="$(readlink -f "${BASH_SOURCE[0]}")"
SCRIPT_DIR="$(cd "$(dirname "$SCRIPT_PATH")" && pwd)"
source "${SCRIPT_DIR}/lib/common.sh"

cd "$RECIPE_REPO_ROOT"

FISHYSTUFF_GITOPS_BETA_ACTIVATION_DRAFT_TEST_SOURCE_ONLY=1
source scripts/recipes/gitops-beta-activation-draft-test.sh
unset FISHYSTUFF_GITOPS_BETA_ACTIVATION_DRAFT_TEST_SOURCE_ONLY

FISHYSTUFF_GITOPS_BETA_SERVICE_START_PLAN_TEST_SOURCE_ONLY=1
source scripts/recipes/gitops-beta-service-start-plan-test.sh
unset FISHYSTUFF_GITOPS_BETA_SERVICE_START_PLAN_TEST_SOURCE_ONLY

pass_count=0

pass() {
  printf '[gitops-beta-first-service-set-packet-test] pass: %s\n' "$1"
  pass_count="$((pass_count + 1))"
}

root="$(mktemp -d)"
fake_bin="${root}/bin"
mkdir -p "$fake_bin"
cat >"${fake_bin}/secretspec" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
if [[ "${1-}" == "check" && "${2-}" == "--profile" && "${3-}" == "beta-runtime" ]]; then
  exit 0
fi
exit 2
EOF
chmod +x "${fake_bin}/secretspec"
cat >"${fake_bin}/hostname" <<'EOF'
#!/usr/bin/env bash
printf 'operator-dev\n'
EOF
chmod +x "${fake_bin}/hostname"
PATH="${fake_bin}:${PATH}"

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

fixture_root="${root}/fixture"
mkdir -p "$fixture_root"
make_fixture "$fixture_root"
make_fake_mgmt "${fixture_root}/mgmt"
make_fake_deploy "${fixture_root}/fishystuff_deploy"
summary="$(cat "${fixture_root}/summary.path")"
api_meta="$(cat "${fixture_root}/api-meta.path")"
db_probe="$(cat "${fixture_root}/db-probe.path")"
site_cdn_probe="$(cat "${fixture_root}/site-cdn-probe.path")"
admission="${fixture_root}/beta-admission.evidence.json"
draft="${fixture_root}/beta-activation.draft.desired.json"
proofs="${fixture_root}/proofs"
export FISHYSTUFF_FAKE_MGMT_MARKER="${fixture_root}/fake-mgmt-state"

bash scripts/recipes/gitops-beta-write-activation-admission-evidence.sh \
  "$admission" \
  "$summary" \
  "http://127.0.0.1:18192" \
  "$api_meta" \
  "$db_probe" \
  "$site_cdn_probe" >/dev/null
bash scripts/recipes/gitops-beta-activation-draft.sh \
  "$draft" \
  "$summary" \
  "$admission" \
  "${fixture_root}/mgmt" \
  "${fixture_root}/fishystuff_deploy" >/dev/null

bash scripts/recipes/gitops-beta-first-service-set-packet.sh \
  "$summary" \
  "$admission" \
  "$draft" \
  "$proofs" \
  auto \
  auto \
  auto \
  "${fixture_root}/api/runtime.env" \
  "${fixture_root}/dolt/beta.env" \
  "http://127.0.0.1:18192" \
  "${fixture_root}/observations" \
  >"${fixture_root}/packet.stdout"

grep -F "gitops_beta_first_service_set_packet_ok=true" "${fixture_root}/packet.stdout" >/dev/null
grep -F "next_required_action=run_runtime_env_preflight_on_beta_host" "${fixture_root}/packet.stdout" >/dev/null
grep -F "service_start_plan_status=pending_runtime_env" "${fixture_root}/packet.stdout" >/dev/null
grep -F "operator_packet_status=run_runtime_env_preflight_on_beta_host" "${fixture_root}/packet.stdout" >/dev/null
grep -F "operator_packet_api_secretspec_status=ready" "${fixture_root}/packet.stdout" >/dev/null
grep -F "operator_packet_runtime_env_host_preflight_next_required_action=run_on_expected_beta_host" "${fixture_root}/packet.stdout" >/dev/null
grep -F "operator_packet_runtime_env_host_preflight_next_command_01=just gitops-beta-runtime-env-host-preflight api_env_file=${fixture_root}/api/runtime.env dolt_env_file=${fixture_root}/dolt/beta.env" "${fixture_root}/packet.stdout" >/dev/null
grep -F "operator_packet_next_command_01=just gitops-beta-runtime-env-host-preflight api_env_file=${fixture_root}/api/runtime.env dolt_env_file=${fixture_root}/dolt/beta.env" "${fixture_root}/packet.stdout" >/dev/null
grep -F "remote_deploy_performed=false" "${fixture_root}/packet.stdout" >/dev/null
grep -F "infrastructure_mutation_performed=false" "${fixture_root}/packet.stdout" >/dev/null
grep -F "local_host_mutation_performed=false" "${fixture_root}/packet.stdout" >/dev/null

if grep -E '^(phase_|read_only_step_|guarded_host_action_|guarded_runtime_env_action_)' "${fixture_root}/packet.stdout" >/dev/null; then
  printf '[gitops-beta-first-service-set-packet-test] packet leaked full runbook lines\n' >&2
  exit 1
fi
pass "runtime env preflight packet"

cat >"${fake_bin}/hostname" <<'EOF'
#!/usr/bin/env bash
printf 'site-nbg1-beta\n'
EOF
chmod +x "${fake_bin}/hostname"

host_root="${root}/host-runtime"
host_api_env="${host_root}/api/runtime.env"
host_dolt_env="${host_root}/dolt/beta.env"

bash scripts/recipes/gitops-beta-first-service-set-packet.sh \
  "$summary" \
  "$admission" \
  "$draft" \
  "$proofs" \
  auto \
  auto \
  auto \
  "$host_api_env" \
  "$host_dolt_env" \
  "http://127.0.0.1:18192" \
  "${fixture_root}/observations" \
  >"${fixture_root}/bootstrap-packet.stdout"

grep -F "next_required_action=bootstrap_beta_host" "${fixture_root}/bootstrap-packet.stdout" >/dev/null
grep -F "operator_packet_status=bootstrap_beta_host" "${fixture_root}/bootstrap-packet.stdout" >/dev/null
grep -F "operator_packet_runtime_env_host_preflight_next_required_action=bootstrap_beta_host" "${fixture_root}/bootstrap-packet.stdout" >/dev/null
grep -F "operator_packet_next_command_01=FISHYSTUFF_GITOPS_ENABLE_BETA_HOST_BOOTSTRAP=1 FISHYSTUFF_GITOPS_ENABLE_BETA_HOST_DIRECTORIES=1 FISHYSTUFF_GITOPS_ENABLE_BETA_HOST_USER_GROUPS=1 just gitops-beta-host-bootstrap-apply" "${fixture_root}/bootstrap-packet.stdout" >/dev/null
grep -F "operator_packet_after_success_command=just gitops-beta-runtime-env-host-preflight api_env_file=${host_api_env} dolt_env_file=${host_dolt_env}" "${fixture_root}/bootstrap-packet.stdout" >/dev/null
grep -F "remote_deploy_performed=false" "${fixture_root}/bootstrap-packet.stdout" >/dev/null
grep -F "infrastructure_mutation_performed=false" "${fixture_root}/bootstrap-packet.stdout" >/dev/null
grep -F "local_host_mutation_performed=false" "${fixture_root}/bootstrap-packet.stdout" >/dev/null
pass "runtime env bootstrap packet"

mkdir -p "${host_root}/api" "${host_root}/dolt"
bash scripts/recipes/gitops-beta-first-service-set-packet.sh \
  "$summary" \
  "$admission" \
  "$draft" \
  "$proofs" \
  auto \
  auto \
  auto \
  "$host_api_env" \
  "$host_dolt_env" \
  "http://127.0.0.1:18192" \
  "${fixture_root}/observations" \
  >"${fixture_root}/write-env-packet.stdout"

grep -F "next_required_action=write_beta_runtime_env" "${fixture_root}/write-env-packet.stdout" >/dev/null
grep -F "operator_packet_status=write_beta_runtime_env" "${fixture_root}/write-env-packet.stdout" >/dev/null
grep -F "operator_packet_runtime_env_host_preflight_next_required_action=write_beta_runtime_env" "${fixture_root}/write-env-packet.stdout" >/dev/null
grep -F "operator_packet_runtime_env_host_preflight_ready=true" "${fixture_root}/write-env-packet.stdout" >/dev/null
grep -F "operator_packet_before_write_command=just gitops-beta-runtime-env-host-preflight api_env_file=${host_api_env} dolt_env_file=${host_dolt_env}" "${fixture_root}/write-env-packet.stdout" >/dev/null
grep -F "operator_packet_next_command_01=FISHYSTUFF_GITOPS_ENABLE_BETA_DOLT_RUNTIME_ENV_WRITE=1 just gitops-beta-write-runtime-env service=dolt output=${host_dolt_env}" "${fixture_root}/write-env-packet.stdout" >/dev/null
grep -F "operator_packet_next_command_02=FISHYSTUFF_GITOPS_ENABLE_BETA_API_RUNTIME_ENV_WRITE=1 just gitops-beta-write-runtime-env-secretspec service=api output=${host_api_env} profile=beta-runtime" "${fixture_root}/write-env-packet.stdout" >/dev/null
grep -F "operator_packet_after_success_command=just gitops-beta-service-start-packet api_bundle=${fixture_root}/active-api dolt_bundle=${fixture_root}/active-dolt-service api_env_file=${host_api_env} dolt_env_file=${host_dolt_env} summary_file=${summary}" "${fixture_root}/write-env-packet.stdout" >/dev/null
grep -F "remote_deploy_performed=false" "${fixture_root}/write-env-packet.stdout" >/dev/null
grep -F "infrastructure_mutation_performed=false" "${fixture_root}/write-env-packet.stdout" >/dev/null
grep -F "local_host_mutation_performed=false" "${fixture_root}/write-env-packet.stdout" >/dev/null
pass "runtime env write packet"

printf '[gitops-beta-first-service-set-packet-test] %s checks passed\n' "$pass_count"
