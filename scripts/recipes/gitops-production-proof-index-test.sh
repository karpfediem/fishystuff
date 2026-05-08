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
  printf '[gitops-production-proof-index-test] pass: %s\n' "$1"
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
    printf '[gitops-production-proof-index-test] expected failure: %s\n' "$name" >&2
    exit 1
  fi
  if ! grep -F "$expected" "$stderr" >/dev/null; then
    printf '[gitops-production-proof-index-test] expected %s stderr to contain %q\n' "$name" "$expected" >&2
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
proof_dir="${root}/proofs"
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
  "$proof_dir" \
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

bash scripts/recipes/gitops-production-proof-index.sh "$proof_dir" 86400 false >"${root}/index.stdout"
grep -F "gitops_production_proof_index_status=complete" "${root}/index.stdout" >/dev/null
grep -F "gitops_production_proof_index_complete=true" "${root}/index.stdout" >/dev/null
grep -F "gitops_production_proof_index_operator_proof=${operator_proof}" "${root}/index.stdout" >/dev/null
grep -F "gitops_production_proof_index_operator_proof_sha256=${operator_proof_sha256}" "${root}/index.stdout" >/dev/null
grep -F "gitops_production_proof_index_served_proof=${served_proof}" "${root}/index.stdout" >/dev/null
grep -F "gitops_production_proof_index_served_link=matches_latest_operator_proof" "${root}/index.stdout" >/dev/null
grep -F "remote_deploy_performed=false" "${root}/index.stdout" >/dev/null
pass "complete proof index"

operator_only_dir="${root}/operator-only"
mkdir -p "$operator_only_dir"
cp "$operator_proof" "${operator_only_dir}/production-operator-proof.fixture.json"
bash scripts/recipes/gitops-production-proof-index.sh "$operator_only_dir" 86400 false >"${root}/operator-only-index.stdout"
grep -F "gitops_production_proof_index_status=missing_served_proof" "${root}/operator-only-index.stdout" >/dev/null
grep -F "gitops_production_proof_index_complete=false" "${root}/operator-only-index.stdout" >/dev/null
pass "operator-only index reports missing served proof"

if bash scripts/recipes/gitops-production-proof-index.sh "$operator_only_dir" 86400 true >"${root}/operator-only-strict.stdout" 2>"${root}/operator-only-strict.stderr"; then
  printf '[gitops-production-proof-index-test] expected failure: strict index rejects missing served proof\n' >&2
  exit 1
fi
grep -F "gitops_production_proof_index_status=missing_served_proof" "${root}/operator-only-strict.stdout" >/dev/null
pass "strict index rejects missing served proof"

newer_operator="${proof_dir}/production-operator-proof.newer.json"
cp "$operator_proof" "$newer_operator"
bash scripts/recipes/gitops-production-proof-index.sh "$proof_dir" 86400 false >"${root}/stale-link-index.stdout"
grep -F "gitops_production_proof_index_status=served_proof_not_linked_to_latest_operator" "${root}/stale-link-index.stdout" >/dev/null
grep -F "gitops_production_proof_index_served_link=stale_or_mismatched_operator_proof" "${root}/stale-link-index.stdout" >/dev/null
grep -F "gitops_production_proof_index_complete=false" "${root}/stale-link-index.stdout" >/dev/null
pass "index detects served proof not linked to latest operator proof"

bash scripts/recipes/gitops-production-proof-index.sh "${root}/missing-proof-dir" 86400 false >"${root}/missing-dir-index.stdout"
grep -F "gitops_production_proof_index_status=missing_proof_dir" "${root}/missing-dir-index.stdout" >/dev/null
grep -F "gitops_production_proof_index_complete=false" "${root}/missing-dir-index.stdout" >/dev/null
pass "missing proof dir is reported"

printf '[gitops-production-proof-index-test] %s checks passed\n' "$pass_count"
