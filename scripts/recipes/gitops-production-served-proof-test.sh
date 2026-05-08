#!/usr/bin/env bash
set -euo pipefail

SCRIPT_PATH="$(readlink -f "${BASH_SOURCE[0]}")"
SCRIPT_DIR="$(cd "$(dirname "$SCRIPT_PATH")" && pwd)"
source "${SCRIPT_DIR}/lib/common.sh"

cd "$RECIPE_REPO_ROOT"

export FISHYSTUFF_GITOPS_OPERATOR_PROOF_TEST_SOURCE_ONLY=1
# Reuse the operator-proof builders for the activation/admission/edge fixture.
# shellcheck source=scripts/recipes/gitops-production-operator-proof-test.sh
source scripts/recipes/gitops-production-operator-proof-test.sh
unset FISHYSTUFF_GITOPS_OPERATOR_PROOF_TEST_SOURCE_ONLY

export FISHYSTUFF_GITOPS_CURRENT_HANDOFF_TEST_SOURCE_ONLY=1
# Reuse the served-state writer that mirrors the activation verifier contract.
# shellcheck source=scripts/recipes/gitops-production-current-handoff-test.sh
source scripts/recipes/gitops-production-current-handoff-test.sh
unset FISHYSTUFF_GITOPS_CURRENT_HANDOFF_TEST_SOURCE_ONLY

pass_count=0

pass() {
  printf '[gitops-production-served-proof-test] pass: %s\n' "$1"
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
    printf '[gitops-production-served-proof-test] expected failure: %s\n' "$name" >&2
    exit 1
  fi
  if ! grep -F "$expected" "$stderr" >/dev/null; then
    printf '[gitops-production-served-proof-test] expected %s stderr to contain %q\n' "$name" "$expected" >&2
    cat "$stderr" >&2
    exit 1
  fi
  pass "$name"
}

make_fake_served_deploy() {
  local path="$1"
  local real_deploy_bin="$2"
  cat >"$path" <<EOF
#!/usr/bin/env bash
set -euo pipefail
case "\$*" in
  gitops\ check-desired-serving\ --state\ *\ --environment\ production)
    printf 'fake_desired_serving_ok\n'
    ;;
  gitops\ inspect-served\ *)
    exec "$real_deploy_bin" "\$@"
    ;;
  *)
    echo "unexpected fake served fishystuff_deploy args: \$*" >&2
    exit 2
    ;;
esac
EOF
  chmod +x "$path"
}

deploy_bin="$(require_deploy_bin)"
root="$(mktemp -d)"
make_fixture "$root"
make_edge_bundle "${root}/edge-bundle"
make_fake_deploy "${root}/fishystuff_deploy"
make_fake_served_deploy "${root}/fishystuff_deploy_served" "$deploy_bin"
write_placeholder_tls "${root}/tls"
cp "${root}/edge-bundle/artifacts/systemd/unit" "${root}/fishystuff-edge.service"

draft="$(cat "${root}/draft.path")"
summary="$(cat "${root}/summary.path")"
admission="$(cat "${root}/admission.path")"
release_id="$(jq -er '.environments.production.active_release' "$draft")"
proof_dir="${root}/operator-proofs"
served_proof_dir="${root}/served-proofs"

write_activation_served_state "${root}/state" "${root}/run" "$draft" "$release_id"

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
  production >"${root}/operator-proof.stdout"

operator_proof="$(awk -F= '$1 == "gitops_production_operator_proof_ok" { print $2 }' "${root}/operator-proof.stdout")"
test -n "$operator_proof"
test -f "$operator_proof"
read -r operator_proof_sha256 _ < <(sha256sum "$operator_proof")

bash scripts/recipes/gitops-production-served-proof.sh \
  "$served_proof_dir" \
  "$draft" \
  "$summary" \
  "$admission" \
  "$operator_proof" \
  "${root}/fishystuff_deploy_served" \
  "${root}/state" \
  "${root}/run" \
  86400 >"${root}/served-proof.stdout"

served_proof="$(awk -F= '$1 == "gitops_production_served_proof_ok" { print $2 }' "${root}/served-proof.stdout")"
test -n "$served_proof"
test -f "$served_proof"
grep -F "gitops_production_served_proof_operator_proof=${operator_proof}" "${root}/served-proof.stdout" >/dev/null
grep -F "gitops_production_served_proof_operator_proof_sha256=${operator_proof_sha256}" "${root}/served-proof.stdout" >/dev/null
grep -F "remote_deploy_performed=false" "${root}/served-proof.stdout" >/dev/null

jq -e \
  --arg draft "$draft" \
  --arg summary "$summary" \
  --arg admission "$admission" \
  --arg operator_proof "$operator_proof" \
  --arg operator_proof_sha256 "$operator_proof_sha256" \
  --arg release_id "$release_id" \
  '
    .schema == "fishystuff.gitops.production-served-proof.v1"
    and .environment == "production"
    and .inputs.draft_file == $draft
    and .inputs.summary_file == $summary
    and .inputs.admission_file == $admission
    and .inputs.operator_proof_file == $operator_proof
    and .inputs.operator_proof_sha256 == $operator_proof_sha256
    and .served.release_id == $release_id
    and .commands.operator_proof_check.success == true
    and .commands.served_verification.success == true
    and .commands.served_verification.kv.gitops_activation_served_ok == $release_id
    and .remote_deploy_performed == false
    and .infrastructure_mutation_performed == false
  ' "$served_proof" >/dev/null
pass "valid production served proof"

draft_copy="${root}/draft-copy.desired.json"
cp "$draft" "$draft_copy"
expect_fail_contains \
  "reject operator proof tuple mismatch" \
  "operator proof draft_file does not match activation draft" \
  bash scripts/recipes/gitops-production-served-proof.sh \
    "${root}/served-proofs-mismatch" \
    "$draft_copy" \
    "$summary" \
    "$admission" \
    "$operator_proof" \
    "${root}/fishystuff_deploy_served" \
    "${root}/state" \
    "${root}/run" \
    86400

jq '.release_id = "wrong-release"' "${root}/state/status/production.json" >"${root}/state/status/production.json.tmp"
mv "${root}/state/status/production.json.tmp" "${root}/state/status/production.json"
expect_fail_contains \
  "reject failed served verification" \
  "gitops_production_served_proof_step_fail=served_verification" \
  bash scripts/recipes/gitops-production-served-proof.sh \
    "${root}/served-proofs-failed" \
    "$draft" \
    "$summary" \
    "$admission" \
    "$operator_proof" \
    "${root}/fishystuff_deploy_served" \
    "${root}/state" \
    "${root}/run" \
    86400

printf '[gitops-production-served-proof-test] %s checks passed\n' "$pass_count"
