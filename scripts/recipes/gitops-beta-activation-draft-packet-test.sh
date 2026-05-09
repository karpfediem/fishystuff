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
  printf '[gitops-beta-activation-draft-packet-test] pass: %s\n' "$1"
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
    printf '[gitops-beta-activation-draft-packet-test] expected failure: %s\n' "$name" >&2
    exit 1
  fi
  if ! grep -F "$expected" "$stderr" >/dev/null; then
    printf '[gitops-beta-activation-draft-packet-test] expected %s stderr to contain %q\n' "$name" "$expected" >&2
    cat "$stderr" >&2
    exit 1
  fi
  pass "$name"
}

root="$(mktemp -d)"
make_fixture "$root"
make_fake_mgmt "${root}/mgmt"
make_fake_deploy "${root}/fishystuff_deploy"

summary="$(cat "${root}/summary.path")"
api_meta="$(cat "${root}/api-meta.path")"
db_probe="$(cat "${root}/db-probe.path")"
site_cdn_probe="$(cat "${root}/site-cdn-probe.path")"
admission="${root}/beta-admission.evidence.json"
draft="${root}/beta-activation.draft.desired.json"
proof_dir="${root}/proofs"
api_upstream="http://127.0.0.1:18192"
fake_mgmt_marker="${root}/fake-mgmt-state"
export FISHYSTUFF_FAKE_MGMT_MARKER="$fake_mgmt_marker"

bash scripts/recipes/gitops-beta-activation-draft-packet.sh \
  "$draft" \
  "$summary" \
  "$admission" \
  "$proof_dir" \
  auto \
  "${root}/fishystuff_deploy" \
  "$api_upstream" \
  "${root}/observations" >"${root}/missing-admission.stdout"

grep -F "gitops_beta_activation_draft_packet_ok=true" "${root}/missing-admission.stdout" >/dev/null
grep -F "activation_draft_packet_status=missing_admission" "${root}/missing-admission.stdout" >/dev/null
grep -F "activation_draft_packet_summary_file=${summary}" "${root}/missing-admission.stdout" >/dev/null
grep -F "activation_draft_packet_admission_file=${admission}" "${root}/missing-admission.stdout" >/dev/null
grep -F "activation_draft_packet_draft_file=${draft}" "${root}/missing-admission.stdout" >/dev/null
grep -F "activation_draft_packet_next_command_01=just gitops-beta-admission-packet admission_file=${admission} summary_file=${summary} api_upstream=${api_upstream} observation_dir=${root}/observations draft_file=${draft}" "${root}/missing-admission.stdout" >/dev/null
grep -F "activation_draft_packet_after_success_command=just gitops-beta-activation-draft-packet draft_file=${draft} summary_file=${summary} admission_file=${admission} proof_dir=${proof_dir} edge_bundle=auto deploy_bin=${root}/fishystuff_deploy api_upstream=${api_upstream} observation_dir=${root}/observations" "${root}/missing-admission.stdout" >/dev/null
grep -F "remote_deploy_performed=false" "${root}/missing-admission.stdout" >/dev/null
grep -F "infrastructure_mutation_performed=false" "${root}/missing-admission.stdout" >/dev/null
grep -F "local_host_mutation_performed=false" "${root}/missing-admission.stdout" >/dev/null
pass "missing admission activation draft packet"

bash scripts/recipes/gitops-beta-write-activation-admission-evidence.sh \
  "$admission" \
  "$summary" \
  "$api_upstream" \
  "$api_meta" \
  "$db_probe" \
  "$site_cdn_probe" >/dev/null 2>"${root}/write-admission.stderr"

bash scripts/recipes/gitops-beta-activation-draft-packet.sh \
  "$draft" \
  "$summary" \
  "$admission" \
  "$proof_dir" \
  auto \
  "${root}/fishystuff_deploy" \
  "$api_upstream" \
  "${root}/observations" >"${root}/missing-draft.stdout"

grep -F "activation_draft_packet_status=missing_draft" "${root}/missing-draft.stdout" >/dev/null
grep -F "activation_draft_packet_release_id=beta-release" "${root}/missing-draft.stdout" >/dev/null
grep -F "activation_draft_packet_next_command_01=just gitops-beta-activation-draft output=${draft} summary_file=${summary} admission_file=${admission} deploy_bin=${root}/fishystuff_deploy" "${root}/missing-draft.stdout" >/dev/null
grep -F "activation_draft_packet_after_success_command=just gitops-beta-operator-proof-packet proof_dir=${proof_dir} draft_file=${draft} summary_file=${summary} admission_file=${admission} edge_bundle=auto deploy_bin=${root}/fishystuff_deploy api_upstream=${api_upstream} observation_dir=${root}/observations" "${root}/missing-draft.stdout" >/dev/null
pass "missing draft activation draft packet"

bash scripts/recipes/gitops-beta-activation-draft.sh \
  "$draft" \
  "$summary" \
  "$admission" \
  "${root}/mgmt" \
  "${root}/fishystuff_deploy" >/dev/null 2>"${root}/activation.stderr"

bash scripts/recipes/gitops-beta-activation-draft-packet.sh \
  "$draft" \
  "$summary" \
  "$admission" \
  "$proof_dir" \
  auto \
  "${root}/fishystuff_deploy" \
  "$api_upstream" \
  "${root}/observations" >"${root}/ready.stdout"

grep -F "activation_draft_packet_status=ready" "${root}/ready.stdout" >/dev/null
grep -F "activation_draft_packet_next_command_01=just gitops-beta-operator-proof-packet proof_dir=${proof_dir} draft_file=${draft} summary_file=${summary} admission_file=${admission} edge_bundle=auto deploy_bin=${root}/fishystuff_deploy api_upstream=${api_upstream} observation_dir=${root}/observations" "${root}/ready.stdout" >/dev/null
pass "ready activation draft packet"

bad_draft="${root}/bad-beta-activation.draft.desired.json"
jq '.environments.beta.api_upstream = "http://127.0.0.1:9999"' "$draft" >"$bad_draft"
expect_fail_contains \
  "reject stale activation draft" \
  "activation draft does not match verified handoff and admission evidence" \
  bash scripts/recipes/gitops-beta-activation-draft-packet.sh \
    "$bad_draft" \
    "$summary" \
    "$admission" \
    "$proof_dir" \
    auto \
    "${root}/fishystuff_deploy" \
    "$api_upstream" \
    "${root}/observations"

production_summary="${root}/production-summary.json"
jq '.environment.name = "production"' "$summary" >"$production_summary"
expect_fail_contains \
  "reject production summary" \
  "requires a beta handoff summary" \
  bash scripts/recipes/gitops-beta-activation-draft-packet.sh \
    "$draft" \
    "$production_summary" \
    "$admission" \
    "$proof_dir" \
    auto \
    "${root}/fishystuff_deploy" \
    "$api_upstream" \
    "${root}/observations"

printf '[gitops-beta-activation-draft-packet-test] %s checks passed\n' "$pass_count"
