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
  printf '[gitops-beta-apply-activation-draft-test] pass: %s\n' "$1"
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
    printf '[gitops-beta-apply-activation-draft-test] expected failure: %s\n' "$name" >&2
    exit 1
  fi
  if ! grep -F "$expected" "$stderr" >/dev/null; then
    printf '[gitops-beta-apply-activation-draft-test] expected %s stderr to contain %q\n' "$name" "$expected" >&2
    cat "$stderr" >&2
    exit 1
  fi
  pass "$name"
}

write_fake_mgmt_apply() {
  local path="$1"
  local marker="$2"

  cat >"$path" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail

marker="${FISHYSTUFF_FAKE_MGMT_MARKER:?}"
if [[ "${FISHYSTUFF_GITOPS_ENABLE_LOCAL_APPLY:-}" != "1" ]]; then
  echo "FISHYSTUFF_GITOPS_ENABLE_LOCAL_APPLY was not set for fake beta apply" >&2
  exit 2
fi
if [[ "${FISHYSTUFF_GITOPS_STATE_FILE:-}" != /* ]]; then
  echo "FISHYSTUFF_GITOPS_STATE_FILE must be absolute" >&2
  exit 2
fi
if [[ ! -f "$FISHYSTUFF_GITOPS_STATE_FILE" ]]; then
  echo "FISHYSTUFF_GITOPS_STATE_FILE does not exist: $FISHYSTUFF_GITOPS_STATE_FILE" >&2
  exit 2
fi
if ! jq -e '.mode == "local-apply" and .environments.beta.enabled == true and (.environments.production? == null)' "$FISHYSTUFF_GITOPS_STATE_FILE" >/dev/null; then
  echo "fake beta mgmt apply received a non-beta activation draft" >&2
  exit 2
fi
expected=(run --tmp-prefix --no-pgp lang --no-watch --converged-timeout 45 main.mcl)
if [[ "$*" != "${expected[*]}" ]]; then
  echo "unexpected fake beta mgmt apply args: $*" >&2
  exit 2
fi
printf '%s\n' "$FISHYSTUFF_GITOPS_STATE_FILE" >"$marker"
EOF
  chmod +x "$path"
}

write_fake_hostname() {
  local path="$1"

  cat >"$path" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail

printf '%s\n' "${FISHYSTUFF_FAKE_HOSTNAME:-site-nbg1-beta}"
EOF
  chmod +x "$path"
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
apply_fake_mgmt="${root}/mgmt-apply"
apply_fake_mgmt_marker="${root}/fake-mgmt-apply-state"
fake_bin="${root}/fake-bin"
mkdir -p "$fake_bin"
write_fake_mgmt_apply "$apply_fake_mgmt" "$apply_fake_mgmt_marker"
write_fake_hostname "${fake_bin}/hostname"
PATH="${fake_bin}:$PATH"

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

expect_fail_contains \
  "beta apply refuses without beta opt-in" \
  "gitops-apply-activation-draft requires FISHYSTUFF_GITOPS_ENABLE_BETA_APPLY=1" \
  bash scripts/recipes/gitops-beta-apply-activation-draft.sh \
    "$draft" \
    "$summary" \
    "$admission" \
    "$apply_fake_mgmt" \
    "${root}/fishystuff_deploy"

expect_fail_contains \
  "beta apply requires beta operator proof file" \
  "gitops-apply-activation-draft requires proof_file or FISHYSTUFF_GITOPS_BETA_OPERATOR_PROOF_FILE" \
  env FISHYSTUFF_GITOPS_ENABLE_BETA_APPLY=1 FISHYSTUFF_GITOPS_ENABLE_LOCAL_APPLY=1 \
    bash scripts/recipes/gitops-beta-apply-activation-draft.sh \
      "$draft" \
      "$summary" \
      "$admission" \
      "$apply_fake_mgmt" \
      "${root}/fishystuff_deploy"

expect_fail_contains \
  "beta apply requires reviewed operator proof hash" \
  "gitops-apply-activation-draft requires FISHYSTUFF_GITOPS_BETA_APPLY_OPERATOR_PROOF_SHA256" \
  env FISHYSTUFF_GITOPS_ENABLE_BETA_APPLY=1 FISHYSTUFF_GITOPS_ENABLE_LOCAL_APPLY=1 \
    bash scripts/recipes/gitops-beta-apply-activation-draft.sh \
      "$draft" \
      "$summary" \
      "$admission" \
      "$apply_fake_mgmt" \
      "${root}/fishystuff_deploy" \
      45 \
      "$operator_proof"

expect_fail_contains \
  "beta apply rejects stale operator proof hash" \
  "FISHYSTUFF_GITOPS_BETA_APPLY_OPERATOR_PROOF_SHA256 does not match operator proof" \
  env FISHYSTUFF_GITOPS_ENABLE_BETA_APPLY=1 FISHYSTUFF_GITOPS_ENABLE_LOCAL_APPLY=1 FISHYSTUFF_GITOPS_BETA_APPLY_OPERATOR_PROOF_SHA256=0000000000000000000000000000000000000000000000000000000000000000 \
    bash scripts/recipes/gitops-beta-apply-activation-draft.sh \
      "$draft" \
      "$summary" \
      "$admission" \
      "$apply_fake_mgmt" \
      "${root}/fishystuff_deploy" \
      45 \
      "$operator_proof"

expect_fail_contains \
  "beta apply rejects wrong host" \
  "gitops-beta-apply-activation-draft requires current hostname to match beta resident hostname" \
  env FISHYSTUFF_FAKE_HOSTNAME=operator-dev FISHYSTUFF_GITOPS_ENABLE_BETA_APPLY=1 FISHYSTUFF_GITOPS_ENABLE_LOCAL_APPLY=1 FISHYSTUFF_GITOPS_BETA_APPLY_OPERATOR_PROOF_SHA256="$operator_proof_sha256" \
    bash scripts/recipes/gitops-beta-apply-activation-draft.sh \
      "$draft" \
      "$summary" \
      "$admission" \
      "$apply_fake_mgmt" \
      "${root}/fishystuff_deploy" \
      45 \
      "$operator_proof"

production_summary="${root}/production-summary.json"
jq '.environment.name = "production"' "$summary" >"$production_summary"
expect_fail_contains \
  "beta apply rejects production summary" \
  "gitops-beta-apply-activation-draft requires a beta handoff summary" \
  env FISHYSTUFF_GITOPS_ENABLE_BETA_APPLY=1 FISHYSTUFF_GITOPS_ENABLE_LOCAL_APPLY=1 FISHYSTUFF_GITOPS_BETA_APPLY_OPERATOR_PROOF_SHA256="$operator_proof_sha256" \
    bash scripts/recipes/gitops-beta-apply-activation-draft.sh \
      "$draft" \
      "$production_summary" \
      "$admission" \
      "$apply_fake_mgmt" \
      "${root}/fishystuff_deploy" \
      45 \
      "$operator_proof"

production_operator_proof="${root}/production-operator-proof.json"
jq '.environment = "production"' "$operator_proof" >"$production_operator_proof"
read -r production_operator_proof_sha256 _ < <(sha256sum "$production_operator_proof")
expect_fail_contains \
  "beta apply rejects production operator proof" \
  "activation apply environment does not match operator proof" \
  env FISHYSTUFF_GITOPS_ENABLE_BETA_APPLY=1 FISHYSTUFF_GITOPS_ENABLE_LOCAL_APPLY=1 FISHYSTUFF_GITOPS_BETA_APPLY_OPERATOR_PROOF_SHA256="$production_operator_proof_sha256" \
    bash scripts/recipes/gitops-beta-apply-activation-draft.sh \
      "$draft" \
      "$summary" \
      "$admission" \
      "$apply_fake_mgmt" \
      "${root}/fishystuff_deploy" \
      45 \
      "$production_operator_proof"

env \
  FISHYSTUFF_FAKE_MGMT_MARKER="$apply_fake_mgmt_marker" \
  FISHYSTUFF_GITOPS_ENABLE_BETA_APPLY=1 \
  FISHYSTUFF_GITOPS_ENABLE_LOCAL_APPLY=1 \
  FISHYSTUFF_GITOPS_BETA_APPLY_OPERATOR_PROOF_SHA256="$operator_proof_sha256" \
  bash scripts/recipes/gitops-beta-apply-activation-draft.sh \
    "$draft" \
    "$summary" \
    "$admission" \
    "$apply_fake_mgmt" \
    "${root}/fishystuff_deploy" \
    45 \
    "$operator_proof" \
    >"${root}/apply.stdout" \
    2>"${root}/apply.stderr"

grep -F "gitops_activation_apply_ok=$draft" "${root}/apply.stdout" >/dev/null
grep -F "gitops_activation_apply_environment=beta" "${root}/apply.stdout" >/dev/null
grep -F "gitops_beta_activation_apply_ok=$draft" "${root}/apply.stdout" >/dev/null
grep -F "gitops_beta_activation_apply_operator_proof=$operator_proof" "${root}/apply.stdout" >/dev/null
grep -F "gitops_beta_activation_apply_operator_proof_sha256=$operator_proof_sha256" "${root}/apply.stdout" >/dev/null
grep -F "remote_deploy_performed=false" "${root}/apply.stdout" >/dev/null
grep -F "infrastructure_mutation_performed=false" "${root}/apply.stdout" >/dev/null
if [[ "$(cat "$apply_fake_mgmt_marker")" != "$draft" ]]; then
  printf '[gitops-beta-apply-activation-draft-test] fake mgmt apply saw wrong activation draft state file\n' >&2
  exit 1
fi
pass "valid beta apply gate"

if grep -F "production" "${root}/apply.stdout" >/dev/null; then
  printf '[gitops-beta-apply-activation-draft-test] beta apply stdout unexpectedly mentions production\n' >&2
  cat "${root}/apply.stdout" >&2
  exit 1
fi
pass "no production strings in beta apply stdout"

printf '[gitops-beta-apply-activation-draft-test] %s checks passed\n' "$pass_count"
