#!/usr/bin/env bash
set -euo pipefail

SCRIPT_PATH="$(readlink -f "${BASH_SOURCE[0]}")"
SCRIPT_DIR="$(cd "$(dirname "$SCRIPT_PATH")" && pwd)"
source "${SCRIPT_DIR}/lib/common.sh"

cd "$RECIPE_REPO_ROOT"

export FISHYSTUFF_GITOPS_OPERATOR_PROOF_TEST_SOURCE_ONLY=1
# Reuse the operator-proof fixture builders so the checker validates the same
# artifact shape produced by the real proof wrapper.
# shellcheck source=scripts/recipes/gitops-production-operator-proof-test.sh
source scripts/recipes/gitops-production-operator-proof-test.sh
unset FISHYSTUFF_GITOPS_OPERATOR_PROOF_TEST_SOURCE_ONLY

pass_count=0

pass() {
  printf '[gitops-check-production-operator-proof-test] pass: %s\n' "$1"
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
    printf '[gitops-check-production-operator-proof-test] expected failure: %s\n' "$name" >&2
    exit 1
  fi
  if ! grep -F "$expected" "$stderr" >/dev/null; then
    printf '[gitops-check-production-operator-proof-test] expected %s stderr to contain %q\n' "$name" "$expected" >&2
    cat "$stderr" >&2
    exit 1
  fi
  pass "$name"
}

root="$(mktemp -d)"
make_fixture "$root"
make_edge_bundle "${root}/edge-bundle"
make_fake_deploy "${root}/fishystuff_deploy"
write_placeholder_tls "${root}/tls"
write_inventory_state "${root}/state" "${root}/run" "${root}/site-root" "${root}/cdn-root"
cp "${root}/edge-bundle/artifacts/systemd/unit" "${root}/fishystuff-edge.service"

draft="$(cat "${root}/draft.path")"
summary="$(cat "${root}/summary.path")"
admission="$(cat "${root}/admission.path")"
proof_dir="${root}/proofs"

bash scripts/recipes/gitops-production-operator-proof.sh \
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
  "${root}/fishystuff-edge.service" \
  "${root}/tls/fullchain.pem" \
  "${root}/tls/privkey.pem" \
  production >"${root}/proof.stdout"

proof_file="$(awk -F= '$1 == "gitops_production_operator_proof_ok" { print $2 }' "${root}/proof.stdout")"
test -n "$proof_file"
test -f "$proof_file"
read -r proof_sha256 _ < <(sha256sum "$proof_file")

bash scripts/recipes/gitops-check-production-operator-proof.sh \
  "$proof_file" \
  86400 \
  "$proof_dir" >"${root}/check.stdout"
grep -F "gitops_production_operator_proof_check_ok=${proof_file}" "${root}/check.stdout" >/dev/null
grep -F "gitops_production_operator_proof_sha256=${proof_sha256}" "${root}/check.stdout" >/dev/null
grep -F "gitops_production_operator_proof_environment=production" "${root}/check.stdout" >/dev/null
grep -F "remote_deploy_performed=false" "${root}/check.stdout" >/dev/null
grep -F "infrastructure_mutation_performed=false" "${root}/check.stdout" >/dev/null
pass "valid current proof"

bash scripts/recipes/gitops-check-production-operator-proof.sh \
  "" \
  86400 \
  "$proof_dir" >"${root}/check-latest.stdout"
grep -F "gitops_production_operator_proof_check_ok=${proof_file}" "${root}/check-latest.stdout" >/dev/null
pass "latest proof lookup"

jq '.created_at = "2000-01-01T00:00:00Z"' "$proof_file" >"${root}/stale-proof.json"
expect_fail_contains \
  "reject stale proof" \
  "production operator proof is stale" \
  bash scripts/recipes/gitops-check-production-operator-proof.sh \
    "${root}/stale-proof.json" \
    1 \
    "$proof_dir"

jq '.remote_deploy_performed = true' "$proof_file" >"${root}/mutating-proof.json"
expect_fail_contains \
  "reject mutating proof flags" \
  "production operator proof does not record the required successful local checks" \
  bash scripts/recipes/gitops-check-production-operator-proof.sh \
    "${root}/mutating-proof.json" \
    86400 \
    "$proof_dir"

expect_fail_contains \
  "reject missing proof directory" \
  "production operator proof directory does not exist" \
  bash scripts/recipes/gitops-check-production-operator-proof.sh \
    "" \
    86400 \
    "${root}/missing-proof-dir"

printf '\n' >>"$draft"
expect_fail_contains \
  "reject changed draft file" \
  "operator proof draft_sha256 does not match current file" \
  bash scripts/recipes/gitops-check-production-operator-proof.sh \
    "$proof_file" \
    86400 \
    "$proof_dir"

printf '[gitops-check-production-operator-proof-test] %s checks passed\n' "$pass_count"
