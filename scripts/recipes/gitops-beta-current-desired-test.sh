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
cdn_runtime_closure="$(make_store_fixture "$root" cdn-runtime)"
dolt_service_closure="$(make_store_fixture "$root" dolt-service)"
output="$root/beta-current.desired.json"

FISHYSTUFF_GITOPS_GIT_REV="beta-test-git" \
  FISHYSTUFF_GITOPS_DOLT_COMMIT="beta-test-dolt" \
  FISHYSTUFF_GITOPS_DOLT_REMOTE_URL="file://${root}/dolt-origin" \
  FISHYSTUFF_GITOPS_API_CLOSURE="$api_closure" \
  FISHYSTUFF_GITOPS_SITE_CLOSURE="$site_closure" \
  FISHYSTUFF_GITOPS_CDN_RUNTIME_CLOSURE="$cdn_runtime_closure" \
  FISHYSTUFF_GITOPS_DOLT_SERVICE_CLOSURE="$dolt_service_closure" \
  bash scripts/recipes/gitops-beta-current-desired.sh "$output" beta

jq -e \
  --arg api "$api_closure" \
  --arg site "$site_closure" \
  --arg cdn_runtime "$cdn_runtime_closure" \
  --arg dolt_service "$dolt_service_closure" \
  '
    .cluster == "beta"
    and .generation == 1
    and .mode == "validate"
    and .hosts["beta-single-host"].enabled == true
    and .hosts["beta-single-host"].hostname == "beta-single-host"
    and .environments.beta.enabled == true
    and .environments.beta.host == "beta-single-host"
    and .environments.beta.serve == false
    and (.environments.beta.retained_releases | length) == 0
    and (.releases | to_entries | length) == 1
    and (
      .releases | to_entries[0] as $entry
      | .[$entry.key].git_rev == "beta-test-git"
      and .[$entry.key].dolt_commit == "beta-test-dolt"
      and .[$entry.key].closures.api.store_path == $api
      and .[$entry.key].closures.site.store_path == $site
      and .[$entry.key].closures.cdn_runtime.store_path == $cdn_runtime
      and .[$entry.key].closures.dolt_service.store_path == $dolt_service
      and (.[$entry.key].closures.api.gcroot_path | startswith("/nix/var/nix/gcroots/fishystuff/gitops-beta/"))
      and .[$entry.key].dolt.branch_context == "beta"
      and .[$entry.key].dolt.cache_dir == "/var/lib/fishystuff/gitops-beta/dolt-cache/fishystuff"
      and (.[$entry.key].dolt.release_ref | startswith("fishystuff/gitops-beta/"))
    )
  ' "$output" >/dev/null

if jq -e '.. | strings | select(. == "production" or . == "main" or test("/var/lib/fishystuff/gitops/") or test("/nix/var/nix/gcroots/fishystuff/gitops/") or test("^fishystuff/gitops/"))' "$output" >/dev/null; then
  echo "beta desired-state fixture leaked production-oriented names" >&2
  jq . "$output" >&2
  exit 1
fi

fake_mgmt="$root/fake-mgmt"
mgmt_marker="$root/fake-mgmt-state-file"
write_fake_mgmt "$fake_mgmt" "$mgmt_marker"
bash scripts/recipes/gitops-unify.sh "$fake_mgmt" "$output"
grep -Fx "$output" "$mgmt_marker" >/dev/null

printf '[gitops-beta-current-desired-test] checks passed\n'
