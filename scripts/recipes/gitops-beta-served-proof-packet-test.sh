#!/usr/bin/env bash
set -euo pipefail

SCRIPT_PATH="$(readlink -f "${BASH_SOURCE[0]}")"
SCRIPT_DIR="$(cd "$(dirname "$SCRIPT_PATH")" && pwd)"
source "${SCRIPT_DIR}/lib/common.sh"

cd "$RECIPE_REPO_ROOT"

FISHYSTUFF_GITOPS_BETA_HOST_HANDOFF_PLAN_TEST_SOURCE_ONLY=1
source scripts/recipes/gitops-beta-host-handoff-plan-test.sh
unset FISHYSTUFF_GITOPS_BETA_HOST_HANDOFF_PLAN_TEST_SOURCE_ONLY

FISHYSTUFF_GITOPS_BETA_VERIFY_ACTIVATION_SERVED_TEST_SOURCE_ONLY=1
source scripts/recipes/gitops-beta-verify-activation-served-test.sh
unset FISHYSTUFF_GITOPS_BETA_VERIFY_ACTIVATION_SERVED_TEST_SOURCE_ONLY

pass_count=0

pass() {
  printf '[gitops-beta-served-proof-packet-test] pass: %s\n' "$1"
  pass_count="$((pass_count + 1))"
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
deploy_bin="$(require_deploy_bin)"
write_beta_activation_inputs "$root"
make_beta_edge_bundle "${root}/edge-bundle"
make_fake_served_deploy "${root}/fishystuff_deploy_served" "$deploy_bin"
write_beta_placeholder_tls "${root}/tls"
cp "${root}/edge-bundle/artifacts/systemd/unit" "${root}/fishystuff-beta-edge.service"

draft="$(cat "${root}/draft.path")"
summary="$(cat "${root}/summary.path")"
admission="$(cat "${root}/admission.path")"
release_id="$(jq -er '.environments.beta.active_release' "$draft")"
proof_dir="${root}/proofs"
api_upstream="http://127.0.0.1:18192"

bash scripts/recipes/gitops-beta-served-proof-packet.sh \
  "$proof_dir" \
  86400 \
  "$draft" \
  "$summary" \
  "$admission" \
  "" \
  "${root}/fishystuff_deploy_served" \
  "${root}/state" \
  "${root}/run" \
  "${root}/edge-bundle" \
  "$api_upstream" \
  "${root}/observations" >"${root}/missing-operator.stdout"

grep -F "gitops_beta_served_proof_packet_ok=true" "${root}/missing-operator.stdout" >/dev/null
grep -F "served_proof_packet_status=missing_operator_proof" "${root}/missing-operator.stdout" >/dev/null
grep -F "served_proof_packet_next_command_01=just gitops-beta-operator-proof-packet" "${root}/missing-operator.stdout" >/dev/null
pass "missing operator proof served packet"

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
  "${root}/tls/privkey.pem" >"${root}/operator-proof.stdout"

operator_proof="$(awk -F= '$1 == "gitops_beta_operator_proof_ok" { print $2 }' "${root}/operator-proof.stdout")"
read -r operator_proof_sha256 _ < <(sha256sum "$operator_proof")

bash scripts/recipes/gitops-beta-served-proof-packet.sh \
  "$proof_dir" \
  86400 \
  "$draft" \
  "$summary" \
  "$admission" \
  "$operator_proof" \
  "${root}/fishystuff_deploy_served" \
  "${root}/state" \
  "${root}/run" \
  "${root}/edge-bundle" \
  "$api_upstream" \
  "${root}/observations" >"${root}/missing-served-state.stdout"

grep -F "served_proof_packet_status=missing_served_state" "${root}/missing-served-state.stdout" >/dev/null
grep -F "served_proof_packet_operator_proof_file=${operator_proof}" "${root}/missing-served-state.stdout" >/dev/null
grep -F "served_proof_packet_operator_proof_sha256=${operator_proof_sha256}" "${root}/missing-served-state.stdout" >/dev/null
grep -F "served_proof_packet_next_command_01=FISHYSTUFF_GITOPS_ENABLE_BETA_APPLY=1" "${root}/missing-served-state.stdout" >/dev/null
pass "missing served state packet"

write_beta_served_state "${root}/state" "${root}/run" "$draft" "$release_id"

bash scripts/recipes/gitops-beta-served-proof-packet.sh \
  "$proof_dir" \
  86400 \
  "$draft" \
  "$summary" \
  "$admission" \
  "$operator_proof" \
  "${root}/fishystuff_deploy_served" \
  "${root}/state" \
  "${root}/run" \
  "${root}/edge-bundle" \
  "$api_upstream" \
  "${root}/observations" >"${root}/missing-served-proof.stdout"

grep -F "served_proof_packet_status=missing_served_proof" "${root}/missing-served-proof.stdout" >/dev/null
grep -F "served_proof_packet_served_state_status=verified" "${root}/missing-served-proof.stdout" >/dev/null
grep -F "served_proof_packet_next_command_01=just gitops-beta-served-proof output_dir=${proof_dir} draft_file=${draft} summary_file=${summary} admission_file=${admission} proof_file=${operator_proof} deploy_bin=${root}/fishystuff_deploy_served state_dir=${root}/state run_dir=${root}/run proof_max_age_seconds=86400" "${root}/missing-served-proof.stdout" >/dev/null
pass "missing served proof packet"

bash scripts/recipes/gitops-beta-served-proof.sh \
  "$proof_dir" \
  "$draft" \
  "$summary" \
  "$admission" \
  "$operator_proof" \
  "${root}/fishystuff_deploy_served" \
  "${root}/state" \
  "${root}/run" \
  86400 >"${root}/served-proof.stdout"

served_proof="$(awk -F= '$1 == "gitops_beta_served_proof_ok" { print $2 }' "${root}/served-proof.stdout")"
read -r served_proof_sha256 _ < <(sha256sum "$served_proof")

bash scripts/recipes/gitops-beta-served-proof-packet.sh \
  "$proof_dir" \
  86400 \
  "$draft" \
  "$summary" \
  "$admission" \
  "$operator_proof" \
  "${root}/fishystuff_deploy_served" \
  "${root}/state" \
  "${root}/run" \
  "${root}/edge-bundle" \
  "$api_upstream" \
  "${root}/observations" >"${root}/ready.stdout"

grep -F "served_proof_packet_status=ready" "${root}/ready.stdout" >/dev/null
grep -F "served_proof_packet_served_proof_file=${served_proof}" "${root}/ready.stdout" >/dev/null
grep -F "served_proof_packet_served_proof_sha256=${served_proof_sha256}" "${root}/ready.stdout" >/dev/null
grep -F "served_proof_packet_next_command_01=just gitops-beta-edge-install-packet edge_bundle=${root}/edge-bundle proof_dir=${proof_dir} max_age_seconds=86400 draft_file=${draft} summary_file=${summary} admission_file=${admission} proof_file=${operator_proof} deploy_bin=${root}/fishystuff_deploy_served state_dir=${root}/state run_dir=${root}/run api_upstream=${api_upstream} observation_dir=${root}/observations" "${root}/ready.stdout" >/dev/null
grep -F "remote_deploy_performed=false" "${root}/ready.stdout" >/dev/null
grep -F "infrastructure_mutation_performed=false" "${root}/ready.stdout" >/dev/null
grep -F "local_host_mutation_performed=false" "${root}/ready.stdout" >/dev/null
pass "ready served proof packet"

printf '[gitops-beta-served-proof-packet-test] %s checks passed\n' "$pass_count"
