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
  printf '[gitops-beta-proof-index-test] pass: %s\n' "$1"
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
proof_dir="${root}/proofs"
write_beta_activation_inputs "$root"
make_beta_edge_bundle "${root}/edge-bundle"
make_fake_served_deploy "${root}/fishystuff_deploy_served" "$deploy_bin"
write_beta_placeholder_tls "${root}/tls"
cp "${root}/edge-bundle/artifacts/systemd/unit" "${root}/fishystuff-beta-edge.service"

draft="$(cat "${root}/draft.path")"
summary="$(cat "${root}/summary.path")"
admission="$(cat "${root}/admission.path")"
release_id="$(jq -er '.environments.beta.active_release' "$draft")"
write_beta_served_state "${root}/state" "${root}/run" "$draft" "$release_id"

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
test -n "$operator_proof"
test -f "$operator_proof"
read -r operator_proof_sha256 _ < <(sha256sum "$operator_proof")

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
test -n "$served_proof"
test -f "$served_proof"

bash scripts/recipes/gitops-beta-proof-index.sh "$proof_dir" 86400 false >"${root}/index.stdout"
grep -F "gitops_beta_proof_index_status=complete" "${root}/index.stdout" >/dev/null
grep -F "gitops_beta_proof_index_complete=true" "${root}/index.stdout" >/dev/null
grep -F "gitops_beta_proof_index_operator_proof=${operator_proof}" "${root}/index.stdout" >/dev/null
grep -F "gitops_beta_proof_index_operator_proof_sha256=${operator_proof_sha256}" "${root}/index.stdout" >/dev/null
grep -F "gitops_beta_proof_index_served_proof=${served_proof}" "${root}/index.stdout" >/dev/null
grep -F "gitops_beta_proof_index_served_link=matches_latest_operator_proof" "${root}/index.stdout" >/dev/null
grep -F "remote_deploy_performed=false" "${root}/index.stdout" >/dev/null
pass "complete beta proof index"

operator_only_dir="${root}/operator-only"
mkdir -p "$operator_only_dir"
cp "$operator_proof" "${operator_only_dir}/beta-operator-proof.fixture.json"
bash scripts/recipes/gitops-beta-proof-index.sh "$operator_only_dir" 86400 false >"${root}/operator-only-index.stdout"
grep -F "gitops_beta_proof_index_status=missing_served_proof" "${root}/operator-only-index.stdout" >/dev/null
grep -F "gitops_beta_proof_index_complete=false" "${root}/operator-only-index.stdout" >/dev/null
pass "operator-only beta index reports missing served proof"

if bash scripts/recipes/gitops-beta-proof-index.sh "$operator_only_dir" 86400 true >"${root}/operator-only-strict.stdout" 2>"${root}/operator-only-strict.stderr"; then
  printf '[gitops-beta-proof-index-test] expected failure: strict index rejects missing served proof\n' >&2
  exit 1
fi
grep -F "gitops_beta_proof_index_status=missing_served_proof" "${root}/operator-only-strict.stdout" >/dev/null
pass "strict beta index rejects missing served proof"

newer_operator="${proof_dir}/beta-operator-proof.newer.json"
cp "$operator_proof" "$newer_operator"
bash scripts/recipes/gitops-beta-proof-index.sh "$proof_dir" 86400 false >"${root}/stale-link-index.stdout"
grep -F "gitops_beta_proof_index_status=served_proof_not_linked_to_latest_operator" "${root}/stale-link-index.stdout" >/dev/null
grep -F "gitops_beta_proof_index_served_link=stale_or_mismatched_operator_proof" "${root}/stale-link-index.stdout" >/dev/null
grep -F "gitops_beta_proof_index_complete=false" "${root}/stale-link-index.stdout" >/dev/null
pass "beta index detects served proof not linked to latest operator proof"

bash scripts/recipes/gitops-beta-proof-index.sh "${root}/missing-proof-dir" 86400 false >"${root}/missing-dir-index.stdout"
grep -F "gitops_beta_proof_index_status=missing_proof_dir" "${root}/missing-dir-index.stdout" >/dev/null
grep -F "gitops_beta_proof_index_complete=false" "${root}/missing-dir-index.stdout" >/dev/null
pass "missing beta proof dir is reported"

printf '[gitops-beta-proof-index-test] %s checks passed\n' "$pass_count"
