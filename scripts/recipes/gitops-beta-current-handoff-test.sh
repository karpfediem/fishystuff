#!/usr/bin/env bash
set -euo pipefail

SCRIPT_PATH="$(readlink -f "${BASH_SOURCE[0]}")"
SCRIPT_DIR="$(cd "$(dirname "$SCRIPT_PATH")" && pwd)"
source "${SCRIPT_DIR}/lib/common.sh"

cd "$RECIPE_REPO_ROOT"

require_command() {
  local name="$1"
  if ! command -v "$name" >/dev/null 2>&1; then
    echo "$name is required" >&2
    exit 127
  fi
}

make_store_fixture() {
  local root="$1"
  local name="$2"
  local dir="$root/$name"

  mkdir -p "$dir"
  printf '%s\n' "$name" >"$dir/fixture.txt"
  nix-store --add "$dir"
}

make_cdn_serving_root() {
  local root="$1"
  local name="$2"
  local current_root="$3"
  local dir="$root/$name"

  mkdir -p "$dir"
  printf 'runtime module\n' >"$dir/fishystuff_ui_bevy.fixture.js"
  printf 'runtime wasm\n' >"$dir/fishystuff_ui_bevy_bg.fixture.wasm"
  jq -n \
    --arg current_root "$current_root" \
    '{
      schema_version: 1,
      current_root: $current_root,
      retained_roots: [],
      retained_root_count: 0,
      assets: [
        "fishystuff_ui_bevy.fixture.js",
        "fishystuff_ui_bevy_bg.fixture.wasm"
      ]
    }' >"$dir/cdn-serving-manifest.json"
  nix-store --add "$dir"
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
printf '%s\n' "$FISHYSTUFF_GITOPS_STATE_FILE" >"$marker"
EOF
  chmod +x "$path"
  export FISHYSTUFF_FAKE_MGMT_MARKER="$marker"
}

require_command jq
require_command nix-store

root="$(mktemp -d)"
trap 'rm -rf "$root"' EXIT

api_closure="$(make_store_fixture "$root" api)"
site_closure="$(make_store_fixture "$root" site)"
current_cdn_root="$(make_store_fixture "$root" current-cdn-root)"
cdn_runtime_closure="$(make_cdn_serving_root "$root" cdn-serving-root "$current_cdn_root")"
dolt_service_closure="$(make_store_fixture "$root" dolt-service)"
output="$root/beta-current.desired.json"
summary="$root/beta-current.handoff-summary.json"
fake_mgmt="$root/fake-mgmt"
mgmt_marker="$root/fake-mgmt-state-file"

write_fake_mgmt "$fake_mgmt" "$mgmt_marker"

FISHYSTUFF_GITOPS_GIT_REV="beta-handoff-test-git" \
  FISHYSTUFF_GITOPS_DOLT_COMMIT="beta-handoff-test-dolt" \
  FISHYSTUFF_GITOPS_DOLT_REMOTE_URL="file://${root}/dolt-origin" \
  FISHYSTUFF_GITOPS_API_CLOSURE="$api_closure" \
  FISHYSTUFF_GITOPS_SITE_CLOSURE="$site_closure" \
  FISHYSTUFF_GITOPS_CDN_RUNTIME_CLOSURE="$cdn_runtime_closure" \
  FISHYSTUFF_GITOPS_DOLT_SERVICE_CLOSURE="$dolt_service_closure" \
  bash scripts/recipes/gitops-beta-current-handoff.sh "$output" beta "$fake_mgmt" auto "$summary"

grep -Fx "$output" "$mgmt_marker" >/dev/null
bash scripts/recipes/gitops-check-handoff-summary.sh "$summary" "$output" >/dev/null

jq -e \
  --arg output "$output" \
  --arg api "$api_closure" \
  --arg site "$site_closure" \
  --arg cdn_runtime "$cdn_runtime_closure" \
  --arg dolt_service "$dolt_service_closure" \
  '
    .schema == "fishystuff.gitops.current-handoff.v1"
    and .desired_state_path == $output
    and .cluster == "beta"
    and .mode == "validate"
    and .environment.name == "beta"
    and .environment.host == "beta-single-host"
    and .environment.serve_requested == false
    and (.environment.retained_releases | length) == 0
    and .active_release.git_rev == "beta-handoff-test-git"
    and .active_release.dolt_commit == "beta-handoff-test-dolt"
    and .active_release.closures.api == $api
    and .active_release.closures.site == $site
    and .active_release.closures.cdn_runtime == $cdn_runtime
    and .active_release.closures.dolt_service == $dolt_service
    and .active_release.dolt.branch_context == "beta"
    and (.active_release.dolt.release_ref | startswith("fishystuff/gitops-beta/"))
    and .retained_release_count == 0
    and (.retained_releases | length) == 0
    and .cdn_retention.active_cdn_runtime == $cdn_runtime
    and (.cdn_retention.active_retained_roots | length) == 0
    and (.cdn_retention.retained_releases | length) == 0
    and .checks.current_desired_generated == true
    and .checks.desired_serving_preflight_passed == false
    and .checks.desired_serving_preflight_skipped == true
    and .checks.closure_paths_verified == true
    and .checks.cdn_manifest_verified == true
    and .checks.cdn_retained_roots_verified == true
    and .checks.gitops_unify_passed == true
    and .checks.remote_deploy_performed == false
    and .checks.infrastructure_mutation_performed == false
  ' "$summary" >/dev/null

if jq -e '.. | strings | select(. == "production" or . == "main" or test("/var/lib/fishystuff/gitops/") or test("/nix/var/nix/gcroots/fishystuff/gitops/") or test("^fishystuff/gitops/"))' "$output" "$summary" >/dev/null; then
  echo "beta current handoff leaked production-oriented names" >&2
  jq . "$summary" >&2
  exit 1
fi

printf '[gitops-beta-current-handoff-test] checks passed\n'
