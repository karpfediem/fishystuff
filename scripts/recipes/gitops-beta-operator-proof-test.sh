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
  printf '[gitops-beta-operator-proof-test] pass: %s\n' "$1"
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
    printf '[gitops-beta-operator-proof-test] expected failure: %s\n' "$name" >&2
    exit 1
  fi
  if ! grep -F "$expected" "$stderr" >/dev/null; then
    printf '[gitops-beta-operator-proof-test] expected %s stderr to contain %q\n' "$name" "$expected" >&2
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
test -n "$proof_file"
test -f "$proof_file"
read -r proof_sha256 _ < <(sha256sum "$proof_file")

jq -e \
  --arg draft "$draft" \
  --arg summary "$summary" \
  --arg admission "$admission" \
  --arg edge_bundle "${root}/edge-bundle" \
  '
    .schema == "fishystuff.gitops.beta-operator-proof.v1"
    and .environment == "beta"
    and .inputs.draft_file == $draft
    and .inputs.summary_file == $summary
    and .inputs.admission_file == $admission
    and .inputs.edge_bundle == $edge_bundle
    and (.inputs.state_dir | contains("/state"))
    and (.inputs.run_dir | contains("/run"))
    and .commands.inventory.success == true
    and .commands.preflight.success == true
    and .commands.host_handoff_plan.success == true
    and .commands.inventory.kv.gitops_beta_host_inventory_ok == "beta"
    and .commands.preflight.kv.gitops_beta_preflight_ok == $draft
    and .commands.host_handoff_plan.kv.gitops_beta_host_handoff_plan_ok == $draft
    and .commands.host_handoff_plan.kv.beta_apply_gate_available == "false"
    and .remote_deploy_performed == false
    and .infrastructure_mutation_performed == false
  ' "$proof_file" >/dev/null

bash scripts/recipes/gitops-check-beta-operator-proof.sh \
  "$proof_file" \
  86400 \
  "$proof_dir" >"${root}/check.stdout"
grep -F "gitops_beta_operator_proof_check_ok=${proof_file}" "${root}/check.stdout" >/dev/null
grep -F "gitops_beta_operator_proof_sha256=${proof_sha256}" "${root}/check.stdout" >/dev/null
grep -F "gitops_beta_operator_proof_environment=beta" "${root}/check.stdout" >/dev/null
grep -F "remote_deploy_performed=false" "${root}/check.stdout" >/dev/null
pass "valid beta operator proof"

if grep -F "production" "${root}/proof.stdout" >/dev/null; then
  printf '[gitops-beta-operator-proof-test] beta operator proof stdout unexpectedly mentions production\n' >&2
  cat "${root}/proof.stdout" >&2
  exit 1
fi
pass "no production strings in beta operator proof stdout"

production_summary="${root}/production-summary.json"
jq '.environment.name = "production"' "$summary" >"$production_summary"
expect_fail_contains \
  "reject production handoff summary" \
  "requires a beta handoff summary" \
  bash scripts/recipes/gitops-beta-operator-proof.sh \
    "${root}/bad-proofs" \
    "$draft" \
    "$production_summary" \
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
    "${root}/tls/privkey.pem"

production_proof="${root}/production-proof.json"
jq '.environment = "production"' "$proof_file" >"$production_proof"
expect_fail_contains \
  "reject production operator proof" \
  "beta operator proof does not record the required successful local checks" \
  bash scripts/recipes/gitops-check-beta-operator-proof.sh \
    "$production_proof" \
    86400 \
    "$proof_dir"

printf '[gitops-beta-operator-proof-test] %s checks passed\n' "$pass_count"
