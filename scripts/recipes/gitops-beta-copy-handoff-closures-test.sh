#!/usr/bin/env bash
set -euo pipefail

SCRIPT_PATH="$(readlink -f "${BASH_SOURCE[0]}")"
SCRIPT_DIR="$(cd "$(dirname "$SCRIPT_PATH")" && pwd)"
source "${SCRIPT_DIR}/lib/common.sh"

cd "$RECIPE_REPO_ROOT"

pass_count=0

pass() {
  printf '[gitops-beta-copy-handoff-closures-test] pass: %s\n' "$1"
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
    printf '[gitops-beta-copy-handoff-closures-test] expected failure: %s\n' "$name" >&2
    exit 1
  fi
  if ! grep -F "$expected" "$stderr" >/dev/null; then
    printf '[gitops-beta-copy-handoff-closures-test] expected %s stderr to contain %q\n' "$name" "$expected" >&2
    cat "$stderr" >&2
    exit 1
  fi
  pass "$name"
}

root="$(mktemp -d)"
summary="${root}/beta-current.handoff-summary.json"
push_bin="${root}/push-closure"
store_fixture="$(dirname "$(dirname "$(readlink -f "$(command -v bash)")")")"
if [[ "$store_fixture" != /nix/store/* || ! -e "$store_fixture" ]]; then
  printf '[gitops-beta-copy-handoff-closures-test] could not resolve a local /nix/store fixture from bash\n' >&2
  exit 1
fi
api="$store_fixture"
site="$store_fixture"
cdn="$store_fixture"
dolt="$store_fixture"

jq -n \
  --arg api "$api" \
  --arg site "$site" \
  --arg cdn "$cdn" \
  --arg dolt "$dolt" \
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
      closures: {
        api: $api,
        site: $site,
        cdn_runtime: $cdn,
        dolt_service: $dolt
      }
    },
    checks: {
      closure_paths_verified: true,
      gitops_unify_passed: true,
      remote_deploy_performed: false,
      infrastructure_mutation_performed: false
    }
  }' >"$summary"

cat >"$push_bin" <<'PUSH'
#!/usr/bin/env bash
set -euo pipefail
printf '%s\n' "$*" >"${FISHYSTUFF_FAKE_PUSH_LOG:?}"
PUSH
chmod +x "$push_bin"

env \
  FISHYSTUFF_OPERATOR_SECRETSPEC_PROFILE=beta-deploy \
  FISHYSTUFF_GITOPS_ENABLE_BETA_REMOTE_CLOSURE_COPY=1 \
  FISHYSTUFF_GITOPS_BETA_REMOTE_CLOSURE_TARGET=root@203.0.113.20 \
  FISHYSTUFF_FAKE_PUSH_LOG="${root}/push.log" \
  bash scripts/recipes/gitops-beta-copy-handoff-closures.sh root@203.0.113.20 "$summary" "$push_bin" >"${root}/copy.out"
grep -F "gitops_beta_copy_handoff_closures_ok=true" "${root}/copy.out" >/dev/null
grep -F "resident_target=root@203.0.113.20" "${root}/copy.out" >/dev/null
grep -F "release_id=release-test" "${root}/copy.out" >/dev/null
grep -F "remote_store_mutation_performed=true" "${root}/copy.out" >/dev/null
grep -F "root@203.0.113.20 ${api} ${site} ${cdn} ${dolt}" "${root}/push.log" >/dev/null
pass "copies exact beta handoff closures through guarded wrapper"

expect_fail_contains \
  "requires opt-in" \
  "gitops-beta-copy-handoff-closures requires FISHYSTUFF_GITOPS_ENABLE_BETA_REMOTE_CLOSURE_COPY=1" \
  env FISHYSTUFF_OPERATOR_SECRETSPEC_PROFILE=beta-deploy \
    bash scripts/recipes/gitops-beta-copy-handoff-closures.sh root@203.0.113.20 "$summary" "$push_bin"

expect_fail_contains \
  "requires target acknowledgement" \
  "gitops-beta-copy-handoff-closures requires FISHYSTUFF_GITOPS_BETA_REMOTE_CLOSURE_TARGET=root@203.0.113.20" \
  env \
    FISHYSTUFF_OPERATOR_SECRETSPEC_PROFILE=beta-deploy \
    FISHYSTUFF_GITOPS_ENABLE_BETA_REMOTE_CLOSURE_COPY=1 \
    FISHYSTUFF_GITOPS_BETA_REMOTE_CLOSURE_TARGET=root@203.0.113.21 \
    bash scripts/recipes/gitops-beta-copy-handoff-closures.sh root@203.0.113.20 "$summary" "$push_bin"

expect_fail_contains \
  "rejects production profile" \
  "must not run with production SecretSpec profile active" \
  env \
    FISHYSTUFF_OPERATOR_SECRETSPEC_PROFILE=production-deploy \
    FISHYSTUFF_GITOPS_ENABLE_BETA_REMOTE_CLOSURE_COPY=1 \
    FISHYSTUFF_GITOPS_BETA_REMOTE_CLOSURE_TARGET=root@203.0.113.20 \
    bash scripts/recipes/gitops-beta-copy-handoff-closures.sh root@203.0.113.20 "$summary" "$push_bin"

expect_fail_contains \
  "rejects dns target" \
  "target host must be an IPv4 address" \
  env \
    FISHYSTUFF_OPERATOR_SECRETSPEC_PROFILE=beta-deploy \
    FISHYSTUFF_GITOPS_ENABLE_BETA_REMOTE_CLOSURE_COPY=1 \
    FISHYSTUFF_GITOPS_BETA_REMOTE_CLOSURE_TARGET=root@beta.fishystuff.fish \
    bash scripts/recipes/gitops-beta-copy-handoff-closures.sh root@beta.fishystuff.fish "$summary" "$push_bin"

expect_fail_contains \
  "rejects previous beta host" \
  "target points at the previous beta host" \
  env \
    FISHYSTUFF_OPERATOR_SECRETSPEC_PROFILE=beta-deploy \
    FISHYSTUFF_GITOPS_ENABLE_BETA_REMOTE_CLOSURE_COPY=1 \
    FISHYSTUFF_GITOPS_BETA_REMOTE_CLOSURE_TARGET=root@178.104.230.121 \
    bash scripts/recipes/gitops-beta-copy-handoff-closures.sh root@178.104.230.121 "$summary" "$push_bin"

jq '.cluster = "production"' "$summary" >"${root}/prod-summary.json"
expect_fail_contains \
  "rejects non-beta summary" \
  "handoff summary cluster must be beta" \
  env \
    FISHYSTUFF_OPERATOR_SECRETSPEC_PROFILE=beta-deploy \
    FISHYSTUFF_GITOPS_ENABLE_BETA_REMOTE_CLOSURE_COPY=1 \
    FISHYSTUFF_GITOPS_BETA_REMOTE_CLOSURE_TARGET=root@203.0.113.20 \
    bash scripts/recipes/gitops-beta-copy-handoff-closures.sh root@203.0.113.20 "${root}/prod-summary.json" "$push_bin"

printf '[gitops-beta-copy-handoff-closures-test] %s checks passed\n' "$pass_count"
