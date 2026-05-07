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

write_served_rollback_set_state() {
  local state_dir="$1"
  local member="$state_dir/rollback-set/production/previous-production-release.json"
  local index="$state_dir/rollback-set/production.json"
  local identity="release=previous-production-release;generation=1;git_rev=previous-git;dolt_commit=previous-dolt;dolt_repository=fishystuff/fishystuff;dolt_branch_context=main;dolt_mode=read_only;api=/nix/store/example-previous-api;site=/nix/store/example-previous-site;cdn_runtime=/nix/store/example-previous-cdn;dolt_service=/nix/store/example-previous-dolt-service"

  mkdir -p "$(dirname "$member")"
  jq -n \
    --arg identity "$identity" \
    '{
      desired_generation: 42,
      environment: "production",
      host: "production-single-host",
      current_release_id: "currently-served-release",
      release_id: "previous-production-release",
      release_identity: $identity,
      api_bundle: "/nix/store/example-previous-api",
      dolt_service_bundle: "/nix/store/example-previous-dolt-service",
      site_content: "/nix/store/example-previous-site",
      cdn_runtime_content: "/nix/store/example-previous-cdn",
      dolt_commit: "previous-dolt",
      dolt_materialization: "fetch_pin",
      dolt_cache_dir: "/var/lib/fishystuff/gitops/dolt-cache/fishystuff",
      dolt_release_ref: "fishystuff/gitops/previous-production-release",
      dolt_status_path: "/run/fishystuff/gitops/dolt/previous-production-release.json",
      rollback_member_state: "retained_hot_release"
    }' >"$member"

  jq -n \
    --arg member "$member" \
    '{
      desired_generation: 42,
      environment: "production",
      host: "production-single-host",
      current_release_id: "currently-served-release",
      current_release_identity: "release=currently-served-release;api=example",
      retained_release_count: 1,
      retained_release_ids: ["previous-production-release"],
      retained_release_document_paths: [$member],
      rollback_set_available: true,
      rollback_set_state: "retained_hot_release_set"
    }' >"$index"
}

run_fixture_handoff() {
  local deploy_bin="$1"
  local root="$2"
  local output="$root/production-current.desired.json"
  local retained="$root/retained.json"
  local fake_mgmt="$root/fake-mgmt"
  local fake_mgmt_marker="$root/fake-mgmt-state-file"
  local summary="$root/production-current.handoff-summary.json"

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
      "$summary" \
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

  jq -e --arg output "$output" '
    .schema == "fishystuff.gitops.production-current-handoff.v1"
    and .desired_state_path == $output
    and .cluster == "production"
    and .mode == "validate"
    and .desired_generation == 23
    and .environment.name == "production"
    and .environment.serve_requested == false
    and (.active_release.release_id | startswith("release-"))
    and .active_release.release_id == .environment.active_release
    and .active_release.dolt_commit == "active-dolt"
    and .active_release.closures.cdn_runtime == "/nix/store/example-active-cdn"
    and .retained_release_count == 1
    and .retained_releases[0].release_id == "previous-production-release"
    and .retained_releases[0].dolt_commit == "previous-dolt"
    and .checks.production_current_desired_generated == true
    and .checks.desired_serving_preflight_passed == true
    and .checks.gitops_unify_passed == true
    and .checks.remote_deploy_performed == false
    and .checks.infrastructure_mutation_performed == false
  ' "$summary" >/dev/null
}

run_fixture_from_served() {
  local deploy_bin="$1"
  local root="$2"
  local output="$root/from-served.desired.json"
  local state_dir="$root/gitops-state"
  local retained="$root/from-served.retained-releases.json"
  local fake_mgmt="$root/fake-mgmt-from-served"
  local fake_mgmt_marker="$root/fake-mgmt-from-served-state-file"
  local summary="$root/from-served.handoff-summary.json"

  write_served_rollback_set_state "$state_dir"
  write_fake_mgmt "$fake_mgmt" "$fake_mgmt_marker"

  FISHYSTUFF_GITOPS_GENERATION=24 \
    FISHYSTUFF_GITOPS_RELEASE_GENERATION=6 \
    FISHYSTUFF_GITOPS_GIT_REV="active-git-from-served" \
    FISHYSTUFF_GITOPS_DOLT_COMMIT="active-dolt-from-served" \
    FISHYSTUFF_GITOPS_DOLT_REMOTE_URL="https://doltremoteapi.dolthub.com/fishystuff/fishystuff" \
    FISHYSTUFF_GITOPS_API_CLOSURE="/nix/store/example-active-from-served-api" \
    FISHYSTUFF_GITOPS_SITE_CLOSURE="/nix/store/example-active-from-served-site" \
    FISHYSTUFF_GITOPS_CDN_RUNTIME_CLOSURE="/nix/store/example-active-from-served-cdn" \
    FISHYSTUFF_GITOPS_DOLT_SERVICE_CLOSURE="/nix/store/example-active-from-served-dolt-service" \
    bash scripts/recipes/gitops-production-current-from-served.sh \
      "$output" \
      "$state_dir" \
      production \
      "$retained" \
      main \
      "$fake_mgmt" \
      "$deploy_bin" \
      "$summary" \
      >"$root/from-served.stdout" \
      2>"$root/from-served.stderr"

  jq -e '
    .[0].release_id == "previous-production-release"
    and .[0].dolt_commit == "previous-dolt"
    and .[0].api_closure == "/nix/store/example-previous-api"
  ' "$retained" >/dev/null

  jq -e '
    .cluster == "production"
    and .generation == 24
    and .environments.production.retained_releases == ["previous-production-release"]
    and .releases[.environments.production.active_release].generation == 6
    and .releases[.environments.production.active_release].dolt_commit == "active-dolt-from-served"
  ' "$output" >/dev/null

  jq -e --arg output "$output" '
    .desired_state_path == $output
    and .retained_release_count == 1
    and .retained_releases[0].release_id == "previous-production-release"
    and .active_release.dolt_commit == "active-dolt-from-served"
    and .checks.desired_serving_preflight_passed == true
    and .checks.gitops_unify_passed == true
  ' "$summary" >/dev/null

  if [[ "$(cat "$fake_mgmt_marker")" != "$output" ]]; then
    printf '[gitops-production-current-handoff-test] fake mgmt from-served saw wrong state file\n' >&2
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

from_served_root="$(mktemp -d)"
run_fixture_from_served "$deploy_bin" "$from_served_root"
pass "served rollback-set feeds retained JSON and checked handoff"

printf '[gitops-production-current-handoff-test] %s checks passed\n' "$pass_count"
