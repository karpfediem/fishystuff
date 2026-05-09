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
  printf '[gitops-beta-served-proof-test] pass: %s\n' "$1"
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
    printf '[gitops-beta-served-proof-test] expected failure: %s\n' "$name" >&2
    exit 1
  fi
  if ! grep -F "$expected" "$stderr" >/dev/null; then
    printf '[gitops-beta-served-proof-test] expected %s stderr to contain %q\n' "$name" "$expected" >&2
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
grep -F "gitops_beta_served_proof_operator_proof=${operator_proof}" "${root}/served-proof.stdout" >/dev/null
grep -F "gitops_beta_served_proof_operator_proof_sha256=${operator_proof_sha256}" "${root}/served-proof.stdout" >/dev/null
grep -F "remote_deploy_performed=false" "${root}/served-proof.stdout" >/dev/null

jq -e \
  --arg draft "$draft" \
  --arg summary "$summary" \
  --arg admission "$admission" \
  --arg operator_proof "$operator_proof" \
  --arg operator_proof_sha256 "$operator_proof_sha256" \
  --arg release_id "$release_id" \
  '
    .schema == "fishystuff.gitops.beta-served-proof.v1"
    and .environment == "beta"
    and .inputs.draft_file == $draft
    and .inputs.summary_file == $summary
    and .inputs.admission_file == $admission
    and .inputs.operator_proof_file == $operator_proof
    and .inputs.operator_proof_sha256 == $operator_proof_sha256
    and .served.release_id == $release_id
    and .commands.operator_proof_check.success == true
    and .commands.served_verification.success == true
    and .commands.served_verification.kv.gitops_activation_served_environment == "beta"
    and .commands.served_verification.kv.gitops_activation_served_ok == $release_id
    and .remote_deploy_performed == false
    and .infrastructure_mutation_performed == false
  ' "$served_proof" >/dev/null
pass "valid beta served proof"

if grep -F "production" "${root}/served-proof.stdout" >/dev/null; then
  printf '[gitops-beta-served-proof-test] beta served proof stdout unexpectedly mentions production\n' >&2
  cat "${root}/served-proof.stdout" >&2
  exit 1
fi
pass "no production strings in beta served proof stdout"

production_operator_proof="${root}/production-operator-proof.json"
jq '.environment = "production"' "$operator_proof" >"$production_operator_proof"
expect_fail_contains \
  "reject production operator proof" \
  "operator proof environment does not match served proof" \
  bash scripts/recipes/gitops-beta-served-proof.sh \
    "${root}/bad-served-proofs" \
    "$draft" \
    "$summary" \
    "$admission" \
    "$production_operator_proof" \
    "${root}/fishystuff_deploy_served" \
    "${root}/state" \
    "${root}/run" \
    86400

jq '.release_id = "wrong-release"' "${root}/state/status/beta.json" >"${root}/state/status/beta.json.tmp"
mv "${root}/state/status/beta.json.tmp" "${root}/state/status/beta.json"
expect_fail_contains \
  "reject failed beta served verification" \
  "gitops_beta_served_proof_step_fail=served_verification" \
  bash scripts/recipes/gitops-beta-served-proof.sh \
    "${root}/failed-served-proofs" \
    "$draft" \
    "$summary" \
    "$admission" \
    "$operator_proof" \
    "${root}/fishystuff_deploy_served" \
    "${root}/state" \
    "${root}/run" \
    86400

printf '[gitops-beta-served-proof-test] %s checks passed\n' "$pass_count"
