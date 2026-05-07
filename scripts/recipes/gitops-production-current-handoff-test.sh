#!/usr/bin/env bash
set -euo pipefail

SCRIPT_PATH="$(readlink -f "${BASH_SOURCE[0]}")"
SCRIPT_DIR="$(cd "$(dirname "$SCRIPT_PATH")" && pwd)"
source "${SCRIPT_DIR}/lib/common.sh"

cd "$RECIPE_REPO_ROOT"

pass_count=0

pass() {
  printf '[gitops-production-current-handoff-test] pass: %s\n' "$1"
  pass_count="$((pass_count + 1))"
}

expect_fail_contains() {
  local name="$1"
  local expected="$2"
  shift 2
  local root=""
  local stderr=""

  root="$(mktemp -d)"
  stderr="$root/stderr"
  if "$@" >"$root/stdout" 2>"$stderr"; then
    printf '[gitops-production-current-handoff-test] expected failure: %s\n' "$name" >&2
    exit 1
  fi
  if ! grep -F "$expected" "$stderr" >/dev/null; then
    printf '[gitops-production-current-handoff-test] expected %s stderr to contain %q\n' "$name" "$expected" >&2
    cat "$stderr" >&2
    exit 1
  fi
  pass "$name"
}

require_deploy_bin() {
  if [[ -x ./target/debug/fishystuff_deploy ]]; then
    printf '%s\n' "./target/debug/fishystuff_deploy"
    return
  fi
  cargo build -p fishystuff_deploy >/dev/null
  printf '%s\n' "./target/debug/fishystuff_deploy"
}

write_fake_mgmt() {
  local path="$1"
  local marker="$2"
  cat >"$path" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail

marker="${FISHYSTUFF_FAKE_MGMT_MARKER:?}"
if [[ "${FISHYSTUFF_GITOPS_STATE_FILE:-}" != /* ]]; then
  echo "FISHYSTUFF_GITOPS_STATE_FILE must be absolute" >&2
  exit 2
fi
if [[ ! -f "$FISHYSTUFF_GITOPS_STATE_FILE" ]]; then
  echo "FISHYSTUFF_GITOPS_STATE_FILE does not exist: $FISHYSTUFF_GITOPS_STATE_FILE" >&2
  exit 2
fi
expected=(run --tmp-prefix --no-network --no-pgp lang --only-unify main.mcl)
if [[ "$*" != "${expected[*]}" ]]; then
  echo "unexpected fake mgmt args: $*" >&2
  exit 2
fi
printf '%s\n' "$FISHYSTUFF_GITOPS_STATE_FILE" > "$marker"
EOF
  chmod +x "$path"
  export FISHYSTUFF_FAKE_MGMT_MARKER="$marker"
}

write_retained_json() {
  local path="$1"
  cat >"$path" <<'EOF'
[
  {
    "release_id": "previous-production-release",
    "generation": 1,
    "git_rev": "previous-git",
    "dolt_commit": "previous-dolt",
    "api_closure": "/nix/store/example-previous-api",
    "site_closure": "/nix/store/example-previous-site",
    "cdn_runtime_closure": "/nix/store/example-previous-cdn",
    "dolt_service_closure": "/nix/store/example-previous-dolt-service",
    "dolt_materialization": "fetch_pin",
    "dolt_cache_dir": "/var/lib/fishystuff/gitops/dolt-cache/fishystuff",
    "dolt_release_ref": "fishystuff/gitops/previous-production-release"
  }
]
EOF
}

run_fixture_handoff() {
  local deploy_bin="$1"
  local root="$2"
  local output="$root/production-current.desired.json"
  local retained="$root/retained.json"
  local fake_mgmt="$root/fake-mgmt"
  local fake_mgmt_marker="$root/fake-mgmt-state-file"

  write_retained_json "$retained"
  write_fake_mgmt "$fake_mgmt" "$fake_mgmt_marker"

  FISHYSTUFF_GITOPS_RETAINED_RELEASES_FILE="$retained" \
    FISHYSTUFF_GITOPS_GENERATION=23 \
    FISHYSTUFF_GITOPS_RELEASE_GENERATION=5 \
    FISHYSTUFF_GITOPS_GIT_REV="active-git" \
    FISHYSTUFF_GITOPS_DOLT_COMMIT="active-dolt" \
    FISHYSTUFF_GITOPS_DOLT_REMOTE_URL="https://doltremoteapi.dolthub.com/fishystuff/fishystuff" \
    FISHYSTUFF_GITOPS_API_CLOSURE="/nix/store/example-active-api" \
    FISHYSTUFF_GITOPS_SITE_CLOSURE="/nix/store/example-active-site" \
    FISHYSTUFF_GITOPS_CDN_RUNTIME_CLOSURE="/nix/store/example-active-cdn" \
    FISHYSTUFF_GITOPS_DOLT_SERVICE_CLOSURE="/nix/store/example-active-dolt-service" \
    bash scripts/recipes/gitops-production-current-handoff.sh \
      "$output" \
      main \
      "$fake_mgmt" \
      "$deploy_bin" \
      >"$root/stdout" \
      2>"$root/stderr"

  jq -e '
    .cluster == "production"
    and .generation == 23
    and .mode == "validate"
    and .environments.production.serve == false
    and .environments.production.retained_releases == ["previous-production-release"]
    and .releases[.environments.production.active_release].generation == 5
    and .releases[.environments.production.active_release].dolt.materialization == "fetch_pin"
    and .releases."previous-production-release".dolt.materialization == "fetch_pin"
  ' "$output" >/dev/null

  if [[ "$(cat "$fake_mgmt_marker")" != "$output" ]]; then
    printf '[gitops-production-current-handoff-test] fake mgmt saw wrong state file\n' >&2
    exit 1
  fi
}

deploy_bin="$(require_deploy_bin)"

expect_fail_contains \
  "missing retained rollback input is refused" \
  "requires FISHYSTUFF_GITOPS_RETAINED_RELEASES_FILE" \
  bash scripts/recipes/gitops-production-current-handoff.sh \
    "$(mktemp -u /tmp/fishystuff-gitops-handoff-missing.XXXXXX.json)" \
    main \
    /run/current-system/sw/bin/true \
    "$deploy_bin"

fixture_root="$(mktemp -d)"
run_fixture_handoff "$deploy_bin" "$fixture_root"
pass "fixture handoff runs generator preflight and fake mgmt unify"

printf '[gitops-production-current-handoff-test] %s checks passed\n' "$pass_count"
