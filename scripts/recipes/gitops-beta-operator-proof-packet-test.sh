#!/usr/bin/env bash
set -euo pipefail

SCRIPT_PATH="$(readlink -f "${BASH_SOURCE[0]}")"
SCRIPT_DIR="$(cd "$(dirname "$SCRIPT_PATH")" && pwd)"
source "${SCRIPT_DIR}/lib/common.sh"

cd "$RECIPE_REPO_ROOT"

FISHYSTUFF_GITOPS_BETA_HOST_HANDOFF_PLAN_TEST_SOURCE_ONLY=1
source scripts/recipes/gitops-beta-host-handoff-plan-test.sh
unset FISHYSTUFF_GITOPS_BETA_HOST_HANDOFF_PLAN_TEST_SOURCE_ONLY

pass_count=0

pass() {
  printf '[gitops-beta-operator-proof-packet-test] pass: %s\n' "$1"
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
    printf '[gitops-beta-operator-proof-packet-test] expected failure: %s\n' "$name" >&2
    exit 1
  fi
  if ! grep -F "$expected" "$stderr" >/dev/null; then
    printf '[gitops-beta-operator-proof-packet-test] expected %s stderr to contain %q\n' "$name" "$expected" >&2
    cat "$stderr" >&2
    exit 1
  fi
  pass "$name"
}

write_beta_placeholder_tls() {
  local credentials_dir="$1"

  mkdir -p "$credentials_dir"
  openssl req \
    -x509 \
    -newkey rsa:2048 \
    -nodes \
    -keyout "${credentials_dir}/privkey.pem" \
    -out "${credentials_dir}/fullchain.pem" \
    -days 1 \
    -subj "/CN=beta.fishystuff.fish" \
    -addext "subjectAltName=DNS:beta.fishystuff.fish,DNS:api.beta.fishystuff.fish,DNS:cdn.beta.fishystuff.fish,DNS:telemetry.beta.fishystuff.fish" \
    >"${credentials_dir}/openssl.log" 2>&1
}

root="$(mktemp -d)"
write_beta_activation_inputs "$root"
make_beta_edge_bundle "${root}/edge-bundle"
write_beta_placeholder_tls "${root}/tls"
cp "${root}/edge-bundle/artifacts/systemd/unit" "${root}/fishystuff-beta-edge.service"

draft="$(cat "${root}/draft.path")"
summary="$(cat "${root}/summary.path")"
admission="$(cat "${root}/admission.path")"
proof_dir="${root}/proofs"
api_upstream="http://127.0.0.1:18192"

missing_draft="${root}/missing-beta-activation.draft.desired.json"
bash scripts/recipes/gitops-beta-operator-proof-packet.sh \
  "" \
  "$proof_dir" \
  86400 \
  "$missing_draft" \
  "$summary" \
  "$admission" \
  "${root}/edge-bundle" \
  "${root}/fishystuff_deploy" \
  "$api_upstream" \
  "${root}/observations" >"${root}/missing-activation.stdout"

grep -F "gitops_beta_operator_proof_packet_ok=true" "${root}/missing-activation.stdout" >/dev/null
grep -F "operator_proof_packet_status=missing_activation_draft" "${root}/missing-activation.stdout" >/dev/null
grep -F "operator_proof_packet_next_command_01=just gitops-beta-activation-draft-packet draft_file=${missing_draft} summary_file=${summary} admission_file=${admission} proof_dir=${proof_dir} edge_bundle=${root}/edge-bundle deploy_bin=${root}/fishystuff_deploy api_upstream=${api_upstream} observation_dir=${root}/observations" "${root}/missing-activation.stdout" >/dev/null
grep -F "operator_proof_packet_after_success_command=just gitops-beta-operator-proof output_dir=${proof_dir} draft_file=${missing_draft} summary_file=${summary} admission_file=${admission} edge_bundle=${root}/edge-bundle deploy_bin=${root}/fishystuff_deploy" "${root}/missing-activation.stdout" >/dev/null
pass "missing activation draft operator proof packet"

bash scripts/recipes/gitops-beta-operator-proof-packet.sh \
  "" \
  "$proof_dir" \
  86400 \
  "$draft" \
  "$summary" \
  "$admission" \
  "${root}/edge-bundle" \
  "${root}/fishystuff_deploy" \
  "$api_upstream" \
  "${root}/observations" >"${root}/missing-proof.stdout"

grep -F "operator_proof_packet_status=missing_operator_proof" "${root}/missing-proof.stdout" >/dev/null
grep -F "operator_proof_packet_release_id=beta-release" "${root}/missing-proof.stdout" >/dev/null
grep -F "operator_proof_packet_next_command_01=just gitops-beta-operator-proof output_dir=${proof_dir} draft_file=${draft} summary_file=${summary} admission_file=${admission} edge_bundle=${root}/edge-bundle deploy_bin=${root}/fishystuff_deploy" "${root}/missing-proof.stdout" >/dev/null
grep -F "remote_deploy_performed=false" "${root}/missing-proof.stdout" >/dev/null
grep -F "infrastructure_mutation_performed=false" "${root}/missing-proof.stdout" >/dev/null
grep -F "local_host_mutation_performed=false" "${root}/missing-proof.stdout" >/dev/null
pass "missing proof operator proof packet"

bash scripts/recipes/gitops-beta-operator-proof.sh \
  "$proof_dir" \
  "$draft" \
  "$summary" \
  "$admission" \
  "${root}/edge-bundle" \
  "${root}/fishystuff_deploy" \
  false \
  "" \
  "" \
  "${root}/state" \
  "${root}/run" \
  "${root}/fishystuff-beta-edge.service" \
  "${root}/tls/fullchain.pem" \
  "${root}/tls/privkey.pem" >"${root}/proof.stdout"

proof_file="$(awk -F= '$1 == "gitops_beta_operator_proof_ok" { print $2 }' "${root}/proof.stdout")"
read -r proof_sha256 _ < <(sha256sum "$proof_file")

bash scripts/recipes/gitops-beta-operator-proof-packet.sh \
  "" \
  "$proof_dir" \
  86400 \
  "$draft" \
  "$summary" \
  "$admission" \
  "${root}/edge-bundle" \
  "${root}/fishystuff_deploy" \
  "$api_upstream" \
  "${root}/observations" >"${root}/ready.stdout"

grep -F "operator_proof_packet_status=ready" "${root}/ready.stdout" >/dev/null
grep -F "operator_proof_packet_proof_file=${proof_file}" "${root}/ready.stdout" >/dev/null
grep -F "operator_proof_packet_proof_sha256=${proof_sha256}" "${root}/ready.stdout" >/dev/null
grep -F "operator_proof_packet_next_command_01=FISHYSTUFF_GITOPS_ENABLE_BETA_APPLY=1 FISHYSTUFF_GITOPS_ENABLE_LOCAL_APPLY=1 FISHYSTUFF_GITOPS_BETA_APPLY_OPERATOR_PROOF_SHA256=${proof_sha256} just gitops-beta-apply-activation-draft draft_file=${draft} summary_file=${summary} admission_file=${admission} deploy_bin=${root}/fishystuff_deploy proof_file=${proof_file} proof_max_age_seconds=86400" "${root}/ready.stdout" >/dev/null
pass "ready operator proof packet"

production_summary="${root}/production-summary.json"
jq '.environment.name = "production"' "$summary" >"$production_summary"
expect_fail_contains \
  "reject production summary" \
  "requires a beta handoff summary" \
  bash scripts/recipes/gitops-beta-operator-proof-packet.sh \
    "" \
    "$proof_dir" \
    86400 \
    "$draft" \
    "$production_summary" \
    "$admission" \
    "${root}/edge-bundle" \
    "${root}/fishystuff_deploy" \
    "$api_upstream" \
    "${root}/observations"

jq '.generation = 99' "$draft" >"${root}/mutated-draft.json"
mv "${root}/mutated-draft.json" "$draft"
expect_fail_contains \
  "reject stale operator proof" \
  "operator proof draft_sha256 does not match current file" \
  bash scripts/recipes/gitops-beta-operator-proof-packet.sh \
    "$proof_file" \
    "$proof_dir" \
    86400 \
    "$draft" \
    "$summary" \
    "$admission" \
    "${root}/edge-bundle" \
    "${root}/fishystuff_deploy" \
    "$api_upstream" \
    "${root}/observations"

printf '[gitops-beta-operator-proof-packet-test] %s checks passed\n' "$pass_count"
