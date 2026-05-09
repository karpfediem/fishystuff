#!/usr/bin/env bash
set -euo pipefail

SCRIPT_PATH="$(readlink -f "${BASH_SOURCE[0]}")"
SCRIPT_DIR="$(cd "$(dirname "$SCRIPT_PATH")" && pwd)"
source "${SCRIPT_DIR}/lib/common.sh"

cd "$RECIPE_REPO_ROOT"

FISHYSTUFF_GITOPS_BETA_ACTIVATION_DRAFT_TEST_SOURCE_ONLY=1
source scripts/recipes/gitops-beta-activation-draft-test.sh
unset FISHYSTUFF_GITOPS_BETA_ACTIVATION_DRAFT_TEST_SOURCE_ONLY

pass_count=0

pass() {
  printf '[gitops-beta-observe-admission-test] pass: %s\n' "$1"
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
    printf '[gitops-beta-observe-admission-test] expected failure: %s\n' "$name" >&2
    exit 1
  fi
  if ! grep -F "$expected" "$stderr" >/dev/null; then
    printf '[gitops-beta-observe-admission-test] expected %s stderr to contain %q\n' "$name" "$expected" >&2
    cat "$stderr" >&2
    exit 1
  fi
  pass "$name"
}

write_fake_curl() {
  local path="$1"

  cat >"$path" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail

if [[ "$#" -ne 2 || "$1" != "-fsS" ]]; then
  echo "unexpected fake curl args: $*" >&2
  exit 2
fi
case "$2" in
  http://127.0.0.1:18192/api/v1/meta)
    cat "${FISHYSTUFF_FAKE_API_META_JSON:?}"
    ;;
  http://127.0.0.1:18192/api/v1/fish?lang=en)
    cat "${FISHYSTUFF_FAKE_FISH_JSON:?}"
    ;;
  *)
    echo "unexpected fake curl URL: $2" >&2
    exit 2
    ;;
esac
EOF
  chmod +x "$path"
}

prepare_runtime_files() {
  local root="$1"
  local summary="$2"
  local site_root=""
  local cdn_root=""

  site_root="$(jq -er '.active_release.closures.site' "$summary")"
  cdn_root="$(jq -er '.active_release.closures.cdn_runtime' "$summary")"
  mkdir -p "${cdn_root}/map"
  cat >"${site_root}/runtime-config.js" <<'EOF'
window.__fishystuffRuntimeConfig = {
  cdnBaseUrl: "https://cdn.beta.fishystuff.fish/"
};
EOF
  printf 'console.log("runtime");\n' >"${cdn_root}/map/fishystuff_ui_bevy.test.js"
  printf 'wasm' >"${cdn_root}/map/fishystuff_ui_bevy_bg.test.wasm"
  jq -n \
    '{
      module: "./fishystuff_ui_bevy.test.js",
      wasm: "./fishystuff_ui_bevy_bg.test.wasm"
    }' >"${cdn_root}/map/runtime-manifest.json"
}

root="$(mktemp -d)"
make_fixture "$root"
summary="$(cat "${root}/summary.path")"
api_meta="$(cat "${root}/api-meta.path")"
fake_curl="${root}/curl"
fish_json="${root}/fish.json"
output="${root}/beta-admission.evidence.json"
observations="${root}/observations"
write_fake_curl "$fake_curl"
prepare_runtime_files "$root" "$summary"
jq -n '{ revision: "fixture-fish-revision", count: 1, fish: [{ item_id: 8474, name: "Mudskipper" }] }' >"$fish_json"

env \
  FISHYSTUFF_FAKE_API_META_JSON="$api_meta" \
  FISHYSTUFF_FAKE_FISH_JSON="$fish_json" \
  bash scripts/recipes/gitops-beta-observe-admission.sh \
    "$output" \
    "$summary" \
    "http://127.0.0.1:18192" \
    "$observations" \
    "$fake_curl" \
    "https://cdn.beta.fishystuff.fish/" \
  >"${root}/observe.stdout"

grep -F "gitops_beta_admission_observation_ok=${output}" "${root}/observe.stdout" >/dev/null
grep -F "gitops_beta_admission_api_upstream=http://127.0.0.1:18192" "${root}/observe.stdout" >/dev/null
grep -F "local_host_mutation_performed=false" "${root}/observe.stdout" >/dev/null
jq -e \
  '.schema == "fishystuff.gitops.activation-admission.v1"
  and .environment == "beta"
  and .db_backed_probe.name == "beta-api-fish-list-en"
  and .db_backed_probe.response.count == 1
  and .site_cdn_probe.name == "beta-site-cdn-runtime-manifest"
  and (.site_cdn_probe.runtime_wasm | endswith(".wasm"))' \
  "$output" >/dev/null
pass "observe beta admission evidence"

expect_fail_contains \
  "reject public API upstream" \
  "api_upstream must be a loopback HTTP URL" \
  env \
    FISHYSTUFF_FAKE_API_META_JSON="$api_meta" \
    FISHYSTUFF_FAKE_FISH_JSON="$fish_json" \
    bash scripts/recipes/gitops-beta-observe-admission.sh \
      "${root}/public-upstream.json" \
      "$summary" \
      "https://api.beta.fishystuff.fish" \
      "${root}/public-observations" \
      "$fake_curl" \
      "https://cdn.beta.fishystuff.fish/"

bad_fish_json="${root}/bad-fish.json"
jq -n '{ revision: "empty", count: 0, fish: [] }' >"$bad_fish_json"
expect_fail_contains \
  "reject empty DB-backed fish probe" \
  "DB-backed fish probe must return a non-empty fish array" \
  env \
    FISHYSTUFF_FAKE_API_META_JSON="$api_meta" \
    FISHYSTUFF_FAKE_FISH_JSON="$bad_fish_json" \
    bash scripts/recipes/gitops-beta-observe-admission.sh \
      "${root}/bad-fish-admission.json" \
      "$summary" \
      "http://127.0.0.1:18192" \
      "${root}/bad-fish-observations" \
      "$fake_curl" \
      "https://cdn.beta.fishystuff.fish/"

missing_manifest_root="$(mktemp -d)"
make_fixture "$missing_manifest_root"
missing_manifest_summary="$(cat "${missing_manifest_root}/summary.path")"
missing_manifest_api_meta="$(cat "${missing_manifest_root}/api-meta.path")"
prepare_runtime_files "$missing_manifest_root" "$missing_manifest_summary"
rm -f "$(jq -er '.active_release.closures.cdn_runtime' "$missing_manifest_summary")/map/runtime-manifest.json"
expect_fail_contains \
  "reject missing CDN runtime manifest" \
  "does not contain map runtime manifest" \
  env \
    FISHYSTUFF_FAKE_API_META_JSON="$missing_manifest_api_meta" \
    FISHYSTUFF_FAKE_FISH_JSON="$fish_json" \
    bash scripts/recipes/gitops-beta-observe-admission.sh \
      "${missing_manifest_root}/admission.json" \
      "$missing_manifest_summary" \
      "http://127.0.0.1:18192" \
      "${missing_manifest_root}/observations" \
      "$fake_curl" \
      "https://cdn.beta.fishystuff.fish/"

printf '[gitops-beta-observe-admission-test] %s checks passed\n' "$pass_count"
