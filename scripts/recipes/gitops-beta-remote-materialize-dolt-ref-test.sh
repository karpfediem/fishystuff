#!/usr/bin/env bash
set -euo pipefail

SCRIPT_PATH="$(readlink -f "${BASH_SOURCE[0]}")"
SCRIPT_DIR="$(cd "$(dirname "$SCRIPT_PATH")" && pwd)"
source "${SCRIPT_DIR}/lib/common.sh"

cd "$RECIPE_REPO_ROOT"

pass_count=0

pass() {
  printf '[gitops-beta-remote-materialize-dolt-ref-test] pass: %s\n' "$1"
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
    printf '[gitops-beta-remote-materialize-dolt-ref-test] expected failure: %s\n' "$name" >&2
    exit 1
  fi
  if ! grep -F "$expected" "$stderr" >/dev/null; then
    printf '[gitops-beta-remote-materialize-dolt-ref-test] expected %s stderr to contain %q\n' "$name" "$expected" >&2
    cat "$stderr" >&2
    exit 1
  fi
  pass "$name"
}

root="$(mktemp -d)"
summary="${root}/beta-current.handoff-summary.json"
dolt_bundle="${root}/dolt-bundle"
fake_ssh="${root}/ssh"
dolt_bin="$(command -v dolt)"

if [[ "$dolt_bin" != /nix/store/*/bin/dolt ]]; then
  printf '[gitops-beta-remote-materialize-dolt-ref-test] dolt fixture is not a Nix store binary: %s\n' "$dolt_bin" >&2
  exit 1
fi

mkdir -p "${dolt_bundle}/artifacts/exe"
cat >"${dolt_bundle}/artifacts/exe/main" <<EOF
#!/usr/bin/env bash
export PATH="${dolt_bin%/dolt}:/nix/store/example-coreutils/bin:\$PATH"
exec dolt sql-server
EOF
chmod +x "${dolt_bundle}/artifacts/exe/main"

jq -n \
  --arg dolt_bundle "$dolt_bundle" \
  '{
    schema: "fishystuff.gitops.current-handoff.v1",
    cluster: "beta",
    mode: "validate",
    environment: {
      name: "beta"
    },
    active_release: {
      release_id: "release-test",
      git_rev: "git-test",
      dolt_commit: "dolt-test",
      dolt: {
        branch_context: "beta",
        materialization: "fetch_pin",
        remote_url: "https://doltremoteapi.dolthub.com/fishystuff/fishystuff",
        release_ref: "fishystuff/gitops-beta/release-test"
      },
      closures: {
        dolt_service: $dolt_bundle
      }
    },
    checks: {
      closure_paths_verified: true,
      gitops_unify_passed: true,
      remote_deploy_performed: false,
      infrastructure_mutation_performed: false
    }
  }' >"$summary"

cat >"$fake_ssh" <<'SSH'
#!/usr/bin/env bash
set -euo pipefail
printf '%s\n' "$*" >>"${FISHYSTUFF_FAKE_REMOTE_LOG:?}"
cat >"${FISHYSTUFF_FAKE_REMOTE_STDIN:?}"
printf 'remote_dolt_branch_commit=dolt-test\n'
printf 'remote_dolt_release_ref=fishystuff/gitops-beta/release-test\n'
printf 'remote_dolt_release_ref_commit=dolt-test\n'
printf 'remote_hostname=site-nbg1-beta\n'
printf 'remote_host_mutation_performed=true\n'
printf 'remote_deploy_performed=false\n'
printf 'infrastructure_mutation_performed=false\n'
printf 'local_host_mutation_performed=false\n'
SSH
chmod +x "$fake_ssh"

env \
  FISHYSTUFF_OPERATOR_SECRETSPEC_PROFILE=beta-deploy \
  FISHYSTUFF_GITOPS_ENABLE_BETA_REMOTE_DOLT_REF_MATERIALIZE=1 \
  FISHYSTUFF_GITOPS_ENABLE_BETA_REMOTE_DOLT_STOP_SERVICES=1 \
  FISHYSTUFF_GITOPS_BETA_REMOTE_DOLT_REF_TARGET=root@203.0.113.20 \
  FISHYSTUFF_GITOPS_BETA_REMOTE_DOLT_REF_ALLOW_BUNDLE_FIXTURE=1 \
  HETZNER_SSH_PRIVATE_KEY='fixture-private-key' \
  FISHYSTUFF_FAKE_REMOTE_LOG="${root}/remote.log" \
  FISHYSTUFF_FAKE_REMOTE_STDIN="${root}/remote.sh" \
  bash scripts/recipes/gitops-beta-remote-materialize-dolt-ref.sh root@203.0.113.20 site-nbg1-beta "$summary" "$fake_ssh" >"${root}/materialize.out"
grep -F "gitops_beta_remote_materialize_dolt_ref_checked=true" "${root}/materialize.out" >/dev/null
grep -F "gitops_beta_remote_materialize_dolt_ref_ok=true" "${root}/materialize.out" >/dev/null
grep -F "dolt_release_ref=fishystuff/gitops-beta/release-test" "${root}/materialize.out" >/dev/null
grep -F "dolt_materialization=fetch_pin" "${root}/materialize.out" >/dev/null
grep -F "dolt_remote_url=https://doltremoteapi.dolthub.com/fishystuff/fishystuff" "${root}/materialize.out" >/dev/null
grep -F "root@203.0.113.20" "${root}/remote.log" >/dev/null
grep -F "systemctl stop fishystuff-beta-api.service" "${root}/remote.sh" >/dev/null
grep -F '"$DOLT_BIN" clone --branch "$BRANCH_CONTEXT" --single-branch "$REMOTE_URL" "$repo_name"' "${root}/remote.sh" >/dev/null
grep -F '"$DOLT_BIN" fetch origin "$BRANCH_CONTEXT"' "${root}/remote.sh" >/dev/null
grep -F '"$DOLT_BIN" branch -f "$RELEASE_REF" "$COMMIT"' "${root}/remote.sh" >/dev/null
pass "remote Dolt release ref materialization is explicit and beta-targeted"

base_env=(
  FISHYSTUFF_OPERATOR_SECRETSPEC_PROFILE=beta-deploy
  FISHYSTUFF_GITOPS_ENABLE_BETA_REMOTE_DOLT_REF_MATERIALIZE=1
  FISHYSTUFF_GITOPS_ENABLE_BETA_REMOTE_DOLT_STOP_SERVICES=1
  FISHYSTUFF_GITOPS_BETA_REMOTE_DOLT_REF_TARGET=root@203.0.113.20
  FISHYSTUFF_GITOPS_BETA_REMOTE_DOLT_REF_ALLOW_BUNDLE_FIXTURE=1
  HETZNER_SSH_PRIVATE_KEY=fixture-private-key
  FISHYSTUFF_FAKE_REMOTE_LOG="${root}/remote-fail.log"
  FISHYSTUFF_FAKE_REMOTE_STDIN="${root}/remote-fail.sh"
)

expect_fail_contains \
  "requires materialize opt-in" \
  "gitops-beta-remote-materialize-dolt-ref requires FISHYSTUFF_GITOPS_ENABLE_BETA_REMOTE_DOLT_REF_MATERIALIZE=1" \
  env \
    FISHYSTUFF_OPERATOR_SECRETSPEC_PROFILE=beta-deploy \
    HETZNER_SSH_PRIVATE_KEY='fixture-private-key' \
    bash scripts/recipes/gitops-beta-remote-materialize-dolt-ref.sh root@203.0.113.20 site-nbg1-beta "$summary" "$fake_ssh"

expect_fail_contains \
  "requires stop-services opt-in" \
  "gitops-beta-remote-materialize-dolt-ref requires FISHYSTUFF_GITOPS_ENABLE_BETA_REMOTE_DOLT_STOP_SERVICES=1" \
  env \
    FISHYSTUFF_OPERATOR_SECRETSPEC_PROFILE=beta-deploy \
    FISHYSTUFF_GITOPS_ENABLE_BETA_REMOTE_DOLT_REF_MATERIALIZE=1 \
    HETZNER_SSH_PRIVATE_KEY='fixture-private-key' \
    bash scripts/recipes/gitops-beta-remote-materialize-dolt-ref.sh root@203.0.113.20 site-nbg1-beta "$summary" "$fake_ssh"

expect_fail_contains \
  "requires target acknowledgement" \
  "gitops-beta-remote-materialize-dolt-ref requires FISHYSTUFF_GITOPS_BETA_REMOTE_DOLT_REF_TARGET=root@203.0.113.20" \
  env \
    "${base_env[@]}" \
    FISHYSTUFF_GITOPS_BETA_REMOTE_DOLT_REF_TARGET=root@203.0.113.21 \
    bash scripts/recipes/gitops-beta-remote-materialize-dolt-ref.sh root@203.0.113.20 site-nbg1-beta "$summary" "$fake_ssh"

expect_fail_contains \
  "rejects production profile" \
  "must not run with production SecretSpec profile active" \
  env \
    "${base_env[@]}" \
    FISHYSTUFF_OPERATOR_SECRETSPEC_PROFILE=production-deploy \
    bash scripts/recipes/gitops-beta-remote-materialize-dolt-ref.sh root@203.0.113.20 site-nbg1-beta "$summary" "$fake_ssh"

expect_fail_contains \
  "rejects dns target" \
  "target host must be an IPv4 address" \
  env \
    "${base_env[@]}" \
    FISHYSTUFF_GITOPS_BETA_REMOTE_DOLT_REF_TARGET=root@beta.fishystuff.fish \
    bash scripts/recipes/gitops-beta-remote-materialize-dolt-ref.sh root@beta.fishystuff.fish site-nbg1-beta "$summary" "$fake_ssh"

printf '[gitops-beta-remote-materialize-dolt-ref-test] %s checks passed\n' "$pass_count"
