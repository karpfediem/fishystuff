#!/usr/bin/env bash
set -euo pipefail

SCRIPT_PATH="$(readlink -f "${BASH_SOURCE[0]}")"
SCRIPT_DIR="$(cd "$(dirname "$SCRIPT_PATH")" && pwd)"
source "${SCRIPT_DIR}/lib/common.sh"

cd "$RECIPE_REPO_ROOT"

FISHYSTUFF_GITOPS_BETA_ACTIVATION_DRAFT_TEST_SOURCE_ONLY=1
source scripts/recipes/gitops-beta-activation-draft-test.sh
unset FISHYSTUFF_GITOPS_BETA_ACTIVATION_DRAFT_TEST_SOURCE_ONLY

pass_count=0

pass() {
  printf '[gitops-beta-first-service-set-plan-test] pass: %s\n' "$1"
  pass_count="$((pass_count + 1))"
}

expect_fail_contains() {
  local name="$1"
  local expected="$2"
  shift 2
  local root=""
  local stderr=""

  root="$(mktemp -d)"
  stderr="${root}/stderr"
  if "$@" >"${root}/stdout" 2>"$stderr"; then
    printf '[gitops-beta-first-service-set-plan-test] expected failure: %s\n' "$name" >&2
    exit 1
  fi
  if ! grep -F "$expected" "$stderr" >/dev/null; then
    printf '[gitops-beta-first-service-set-plan-test] expected %s stderr to contain %q\n' "$name" "$expected" >&2
    cat "$stderr" >&2
    exit 1
  fi
  pass "$name"
}

root="$(mktemp -d)"
pending_summary="${root}/missing-summary.json"
pending_admission="${root}/missing-admission.json"
pending_draft="${root}/missing-draft.json"
pending_proofs="${root}/missing-proofs"

bash scripts/recipes/gitops-beta-first-service-set-plan.sh \
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
  >"${root}/pending.stdout"

grep -F "gitops_beta_first_service_set_plan_ok=true" "${root}/pending.stdout" >/dev/null
grep -F "service_start_plan_status=pending_explicit_bundles" "${root}/pending.stdout" >/dev/null
grep -F "handoff_summary_status=missing" "${root}/pending.stdout" >/dev/null
grep -F "admission_evidence_status=missing" "${root}/pending.stdout" >/dev/null
grep -F "activation_draft_status=missing" "${root}/pending.stdout" >/dev/null
grep -F "gitops_beta_proof_index_status=missing_proof_dir" "${root}/pending.stdout" >/dev/null
grep -F "just gitops-beta-observe-admission output=${pending_admission}" "${root}/pending.stdout" >/dev/null
grep -F "FISHYSTUFF_GITOPS_ENABLE_BETA_SERVICE_START=1" "${root}/pending.stdout" >/dev/null
grep -F "remote_deploy_performed=false" "${root}/pending.stdout" >/dev/null
grep -F "infrastructure_mutation_performed=false" "${root}/pending.stdout" >/dev/null
grep -F "local_host_mutation_performed=false" "${root}/pending.stdout" >/dev/null
pass "pending first service set plan"

expect_fail_contains \
  "reject public API upstream" \
  "api_upstream must be a loopback HTTP URL" \
  bash scripts/recipes/gitops-beta-first-service-set-plan.sh \
    "$pending_summary" \
    "$pending_admission" \
    "$pending_draft" \
    "$pending_proofs" \
    auto \
    auto \
    auto \
    "${root}/api/runtime.env" \
    "${root}/dolt/beta.env" \
    "https://api.beta.fishystuff.fish" \
    "${root}/observations"

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

bash scripts/recipes/gitops-beta-first-service-set-plan.sh \
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
  >"${fixture_root}/ready.stdout"

grep -F "handoff_summary_status=ready" "${fixture_root}/ready.stdout" >/dev/null
grep -F "api_bundle=${fixture_root}/active-api" "${fixture_root}/ready.stdout" >/dev/null
grep -F "dolt_bundle=${fixture_root}/active-dolt-service" "${fixture_root}/ready.stdout" >/dev/null
grep -F "service_start_plan_status=pending_runtime_env" "${fixture_root}/ready.stdout" >/dev/null
grep -F "service_start_plan_missing_dolt_runtime_env=${fixture_root}/dolt/beta.env" "${fixture_root}/ready.stdout" >/dev/null
grep -F "service_start_plan_missing_api_runtime_env=${fixture_root}/api/runtime.env" "${fixture_root}/ready.stdout" >/dev/null
grep -F "admission_evidence_status=ready" "${fixture_root}/ready.stdout" >/dev/null
grep -F "activation_draft_status=ready" "${fixture_root}/ready.stdout" >/dev/null
grep -F "gitops_beta_proof_index_status=missing_proof_dir" "${fixture_root}/ready.stdout" >/dev/null
grep -F "read_only_runtime_env_check_01=just gitops-beta-check-runtime-env service=dolt env_file=${fixture_root}/dolt/beta.env" "${fixture_root}/ready.stdout" >/dev/null
grep -F "read_only_runtime_env_check_02=just gitops-beta-check-runtime-env service=api env_file=${fixture_root}/api/runtime.env" "${fixture_root}/ready.stdout" >/dev/null
grep -F "guarded_runtime_env_action_01=FISHYSTUFF_GITOPS_ENABLE_BETA_DOLT_RUNTIME_ENV_WRITE=1 just gitops-beta-write-runtime-env service=dolt output=${fixture_root}/dolt/beta.env" "${fixture_root}/ready.stdout" >/dev/null
grep -F "guarded_runtime_env_action_02=FISHYSTUFF_GITOPS_ENABLE_BETA_API_RUNTIME_ENV_WRITE=1 FISHYSTUFF_GITOPS_BETA_API_DATABASE_URL=<beta loopback Dolt DSN from operator secret> just gitops-beta-write-runtime-env service=api output=${fixture_root}/api/runtime.env" "${fixture_root}/ready.stdout" >/dev/null
grep -F "guarded_host_action_03=FISHYSTUFF_GITOPS_ENABLE_BETA_APPLY=1" "${fixture_root}/ready.stdout" >/dev/null
grep -F "guarded_host_action_04=FISHYSTUFF_GITOPS_ENABLE_BETA_EDGE_INSTALL=1" "${fixture_root}/ready.stdout" >/dev/null
pass "ready artifact first service set plan"

production_summary="${root}/production-summary.json"
jq '.environment.name = "production"' "$summary" >"$production_summary"
expect_fail_contains \
  "reject production handoff summary" \
  "requires a beta handoff summary" \
  bash scripts/recipes/gitops-beta-first-service-set-plan.sh \
    "$production_summary" \
    "$admission" \
    "$draft" \
    "$proofs" \
    auto \
    auto \
    auto \
    "${fixture_root}/api/runtime.env" \
    "${fixture_root}/dolt/beta.env" \
    "http://127.0.0.1:18192" \
    "${fixture_root}/observations"

production_admission="${root}/production-admission.json"
jq '.environment = "production"' "$admission" >"$production_admission"
expect_fail_contains \
  "reject production admission evidence" \
  "requires beta admission evidence" \
  bash scripts/recipes/gitops-beta-first-service-set-plan.sh \
    "$summary" \
    "$production_admission" \
    "$draft" \
    "$proofs" \
    auto \
    auto \
    auto \
    "${fixture_root}/api/runtime.env" \
    "${fixture_root}/dolt/beta.env" \
    "http://127.0.0.1:18192" \
    "${fixture_root}/observations"

printf '[gitops-beta-first-service-set-plan-test] %s checks passed\n' "$pass_count"
