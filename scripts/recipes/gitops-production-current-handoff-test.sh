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

write_fake_mgmt_apply() {
  local path="$1"
  local marker="$2"
  cat >"$path" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail

marker="${FISHYSTUFF_FAKE_MGMT_MARKER:?}"
if [[ "${FISHYSTUFF_GITOPS_ENABLE_LOCAL_APPLY:-}" != "1" ]]; then
  echo "FISHYSTUFF_GITOPS_ENABLE_LOCAL_APPLY was not set for fake apply" >&2
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
expected=(run --tmp-prefix --no-pgp lang --no-watch --converged-timeout 45 main.mcl)
if [[ "$*" != "${expected[*]}" ]]; then
  echo "unexpected fake mgmt apply args: $*" >&2
  exit 2
fi
printf '%s\n' "$FISHYSTUFF_GITOPS_STATE_FILE" > "$marker"
EOF
  chmod +x "$path"
  export FISHYSTUFF_FAKE_MGMT_MARKER="$marker"
}

make_cdn_current_root() {
  local root="$1"
  local name="$2"
  local dir="$root/${name}"

  mkdir -p "$dir/map"
  printf 'fixture module\n' >"$dir/map/fishystuff_ui_bevy.fixture.js"
  printf 'fixture wasm\n' >"$dir/map/fishystuff_ui_bevy_bg.fixture.wasm"
  jq -n \
    '{
      module: "fishystuff_ui_bevy.fixture.js",
      wasm: "fishystuff_ui_bevy_bg.fixture.wasm"
    }' >"$dir/map/runtime-manifest.json"

  nix-store --add "$dir"
}

make_store_fixture() {
  local root="$1"
  local name="$2"
  local dir="$root/${name}"

  mkdir -p "$dir"
  printf '%s\n' "$name" >"$dir/fixture.txt"
  nix-store --add "$dir"
}

make_cdn_serving_root() {
  local root="$1"
  local name="$2"
  local current_root="$3"
  local retained_roots_json="$4"
  local dir="$root/${name}"

  mkdir -p "$dir"
  cp -R "${current_root}/." "$dir/"
  jq -n \
    --arg current_root "$current_root" \
    --argjson retained_roots "$retained_roots_json" \
    '{
      schema_version: 1,
      current_root: $current_root,
      retained_roots: $retained_roots,
      retained_root_count: ($retained_roots | length),
      assets: []
    }' >"$dir/cdn-serving-manifest.json"

  nix-store --add "$dir"
}

release_identity_from_state() {
  local state_file="$1"
  local release_id="$2"

  jq -er \
    --arg release_id "$release_id" \
    '(.releases[$release_id] // error("release is missing")) as $release
    | "release=\($release_id);generation=\($release.generation);git_rev=\($release.git_rev);dolt_commit=\($release.dolt_commit);dolt_repository=\($release.dolt.repository);dolt_branch_context=\($release.dolt.branch_context);dolt_mode=\($release.dolt.mode);api=\($release.closures.api.store_path);site=\($release.closures.site.store_path);cdn_runtime=\($release.closures.cdn_runtime.store_path);dolt_service=\($release.closures.dolt_service.store_path)"' \
    "$state_file"
}

write_admission_observations() {
  local summary="$1"
  local state_file="$2"
  local api_meta_path="$3"
  local db_probe_path="$4"
  local site_cdn_probe_path="$5"
  local release_id=""
  local release_identity=""
  local dolt_commit=""

  release_id="$(jq -er '.environment.active_release' "$summary")"
  release_identity="$(release_identity_from_state "$state_file" "$release_id")"
  dolt_commit="$(jq -er '.active_release.dolt_commit' "$summary")"

  jq -n \
    --arg release_id "$release_id" \
    --arg release_identity "$release_identity" \
    --arg dolt_commit "$dolt_commit" \
    '{
      release_id: $release_id,
      release_identity: $release_identity,
      dolt_commit: $dolt_commit
    }' >"$api_meta_path"

  jq -n \
    '{
      name: "representative-db-backed-route",
      passed: true
    }' >"$db_probe_path"

  jq -n \
    '{
      name: "site-selected-cdn-runtime",
      passed: true
    }' >"$site_cdn_probe_path"
}

write_retained_json() {
  local path="$1"
  local api_closure="$2"
  local site_closure="$3"
  local cdn_runtime_closure="$4"
  local dolt_service_closure="$5"

  jq -n \
    --arg api_closure "$api_closure" \
    --arg site_closure "$site_closure" \
    --arg cdn_runtime_closure "$cdn_runtime_closure" \
    --arg dolt_service_closure "$dolt_service_closure" \
    '[
      {
        release_id: "previous-production-release",
        generation: 1,
        git_rev: "previous-git",
        dolt_commit: "previous-dolt",
        api_closure: $api_closure,
        site_closure: $site_closure,
        cdn_runtime_closure: $cdn_runtime_closure,
        dolt_service_closure: $dolt_service_closure,
        dolt_materialization: "fetch_pin",
        dolt_cache_dir: "/var/lib/fishystuff/gitops/dolt-cache/fishystuff",
        dolt_release_ref: "fishystuff/gitops/previous-production-release"
      }
    ]' >"$path"
}

write_served_rollback_set_state() {
  local state_dir="$1"
  local api_closure="$2"
  local site_closure="$3"
  local cdn_runtime_closure="$4"
  local dolt_service_closure="$5"
  local member="$state_dir/rollback-set/production/previous-production-release.json"
  local index="$state_dir/rollback-set/production.json"
  local identity="release=previous-production-release;generation=1;git_rev=previous-git;dolt_commit=previous-dolt;dolt_repository=fishystuff/fishystuff;dolt_branch_context=main;dolt_mode=read_only;api=${api_closure};site=${site_closure};cdn_runtime=${cdn_runtime_closure};dolt_service=${dolt_service_closure}"

  mkdir -p "$(dirname "$member")"
  jq -n \
    --arg identity "$identity" \
    --arg api_closure "$api_closure" \
    --arg site_closure "$site_closure" \
    --arg cdn_runtime_closure "$cdn_runtime_closure" \
    --arg dolt_service_closure "$dolt_service_closure" \
    '{
      desired_generation: 42,
      environment: "production",
      host: "production-single-host",
      current_release_id: "currently-served-release",
      release_id: "previous-production-release",
      release_identity: $identity,
      api_bundle: $api_closure,
      dolt_service_bundle: $dolt_service_closure,
      site_content: $site_closure,
      cdn_runtime_content: $cdn_runtime_closure,
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

write_activation_served_state() {
  local state_dir="$1"
  local run_dir="$2"
  local draft_file="$3"
  local release_id="$4"
  local retained_release_id="previous-production-release"
  local generation=""
  local host=""
  local api_upstream=""
  local admission_url=""
  local release_identity=""
  local retained_release_identity=""
  local active_api_closure=""
  local active_site_closure=""
  local active_cdn_closure=""
  local active_dolt_service_closure=""
  local retained_api_closure=""
  local retained_site_closure=""
  local retained_cdn_closure=""
  local retained_dolt_service_closure=""
  local rollback_member="$state_dir/rollback-set/production/${retained_release_id}.json"

  generation="$(jq -er '.generation' "$draft_file")"
  host="$(jq -er '.environments.production.host' "$draft_file")"
  api_upstream="$(jq -er '.environments.production.api_upstream' "$draft_file")"
  admission_url="$(jq -er '.environments.production.admission_probe.url' "$draft_file")"
  release_identity="$(release_identity_from_state "$draft_file" "$release_id")"
  retained_release_identity="$(release_identity_from_state "$draft_file" "$retained_release_id")"
  active_api_closure="$(jq -er --arg release_id "$release_id" '.releases[$release_id].closures.api.store_path' "$draft_file")"
  active_site_closure="$(jq -er --arg release_id "$release_id" '.releases[$release_id].closures.site.store_path' "$draft_file")"
  active_cdn_closure="$(jq -er --arg release_id "$release_id" '.releases[$release_id].closures.cdn_runtime.store_path' "$draft_file")"
  active_dolt_service_closure="$(jq -er --arg release_id "$release_id" '.releases[$release_id].closures.dolt_service.store_path' "$draft_file")"
  retained_api_closure="$(jq -er --arg release_id "$retained_release_id" '.releases[$release_id].closures.api.store_path' "$draft_file")"
  retained_site_closure="$(jq -er --arg release_id "$retained_release_id" '.releases[$release_id].closures.site.store_path' "$draft_file")"
  retained_cdn_closure="$(jq -er --arg release_id "$retained_release_id" '.releases[$release_id].closures.cdn_runtime.store_path' "$draft_file")"
  retained_dolt_service_closure="$(jq -er --arg release_id "$retained_release_id" '.releases[$release_id].closures.dolt_service.store_path' "$draft_file")"

  mkdir -p \
    "$state_dir/status" \
    "$state_dir/active" \
    "$state_dir/rollback" \
    "$state_dir/rollback-set/production" \
    "$run_dir/admission" \
    "$run_dir/routes" \
    "$run_dir/roots"

  jq -n \
    --argjson generation "$generation" \
    --arg host "$host" \
    --arg release_id "$release_id" \
    --arg release_identity "$release_identity" \
    '{
      desired_generation: $generation,
      release_id: $release_id,
      release_identity: $release_identity,
      environment: "production",
      host: $host,
      phase: "served",
      transition_kind: "activate",
      rollback_from_release: "",
      rollback_to_release: "",
      rollback_reason: "verified production handoff admission",
      admission_state: "passed_fixture",
      retained_release_ids: ["previous-production-release"],
      retained_dolt_status_paths: [],
      rollback_available: true,
      rollback_primary_release_id: "previous-production-release",
      rollback_retained_count: 1,
      served: true,
      failure_reason: ""
    }' >"$state_dir/status/production.json"

  jq -n \
    --argjson generation "$generation" \
    --arg host "$host" \
    --arg release_id "$release_id" \
    --arg release_identity "$release_identity" \
    --arg active_site_closure "$active_site_closure" \
    --arg active_cdn_closure "$active_cdn_closure" \
    --arg api_upstream "$api_upstream" \
    --arg state_dir "$state_dir" \
    '{
      desired_generation: $generation,
      environment: "production",
      host: $host,
      release_id: $release_id,
      release_identity: $release_identity,
      instance_name: ("production-" + $release_id),
      site_content: $active_site_closure,
      cdn_runtime_content: $active_cdn_closure,
      api_upstream: $api_upstream,
      site_link: ($state_dir + "/served/production/site"),
      cdn_link: ($state_dir + "/served/production/cdn"),
      retained_release_ids: ["previous-production-release"],
      retained_dolt_status_paths: [],
      transition_kind: "activate",
      rollback_from_release: "",
      rollback_to_release: "",
      rollback_reason: "verified production handoff admission",
      admission_state: "passed_fixture",
      served: true,
      route_state: "selected_local_symlinks"
    }' >"$state_dir/active/production.json"

  jq -n \
    --argjson generation "$generation" \
    --arg host "$host" \
    --arg release_id "$release_id" \
    --arg release_identity "$release_identity" \
    --arg rollback_member "$rollback_member" \
    '{
      desired_generation: $generation,
      environment: "production",
      host: $host,
      current_release_id: $release_id,
      current_release_identity: $release_identity,
      retained_release_count: 1,
      retained_release_ids: ["previous-production-release"],
      retained_release_document_paths: [$rollback_member],
      rollback_set_available: true,
      rollback_set_state: "retained_hot_release_set"
    }' >"$state_dir/rollback-set/production.json"

  jq -n \
    --argjson generation "$generation" \
    --arg host "$host" \
    --arg release_id "$release_id" \
    --arg release_identity "$release_identity" \
    --arg retained_release_identity "$retained_release_identity" \
    --arg retained_api_closure "$retained_api_closure" \
    --arg retained_site_closure "$retained_site_closure" \
    --arg retained_cdn_closure "$retained_cdn_closure" \
    --arg retained_dolt_service_closure "$retained_dolt_service_closure" \
    '{
      desired_generation: $generation,
      environment: "production",
      host: $host,
      current_release_id: $release_id,
      current_release_identity: $release_identity,
      rollback_release_id: "previous-production-release",
      rollback_release_identity: $retained_release_identity,
      rollback_api_bundle: $retained_api_closure,
      rollback_dolt_service_bundle: $retained_dolt_service_closure,
      rollback_site_content: $retained_site_closure,
      rollback_cdn_runtime_content: $retained_cdn_closure,
      rollback_dolt_commit: "previous-dolt",
      rollback_dolt_materialization: "fetch_pin",
      rollback_dolt_cache_dir: "/var/lib/fishystuff/gitops/dolt-cache/fishystuff",
      rollback_dolt_release_ref: "fishystuff/gitops/previous-production-release",
      rollback_available: true,
      rollback_state: "retained_hot_release"
    }' >"$state_dir/rollback/production.json"

  jq -n \
    --argjson generation "$generation" \
    --arg host "$host" \
    --arg release_id "$release_id" \
    --arg release_identity "$release_identity" \
    --arg retained_release_identity "$retained_release_identity" \
    --arg retained_api_closure "$retained_api_closure" \
    --arg retained_site_closure "$retained_site_closure" \
    --arg retained_cdn_closure "$retained_cdn_closure" \
    --arg retained_dolt_service_closure "$retained_dolt_service_closure" \
    '{
      desired_generation: $generation,
      environment: "production",
      host: $host,
      current_release_id: $release_id,
      release_id: "previous-production-release",
      release_identity: $retained_release_identity,
      api_bundle: $retained_api_closure,
      dolt_service_bundle: $retained_dolt_service_closure,
      site_content: $retained_site_closure,
      cdn_runtime_content: $retained_cdn_closure,
      dolt_commit: "previous-dolt",
      dolt_materialization: "fetch_pin",
      dolt_cache_dir: "/var/lib/fishystuff/gitops/dolt-cache/fishystuff",
      dolt_release_ref: "fishystuff/gitops/previous-production-release",
      dolt_status_path: "/run/fishystuff/gitops/dolt/production-previous-production-release.json",
      rollback_member_state: "retained_hot_release"
    }' >"$rollback_member"

  jq -n \
    --arg host "$host" \
    --arg release_id "$release_id" \
    --arg release_identity "$release_identity" \
    --arg active_site_closure "$active_site_closure" \
    --arg active_cdn_closure "$active_cdn_closure" \
    --arg admission_url "$admission_url" \
    '{
      environment: "production",
      host: $host,
      release_id: $release_id,
      release_identity: $release_identity,
      site_content: $active_site_closure,
      cdn_runtime_content: $active_cdn_closure,
      admission_state: "passed_fixture",
      probe: "http-json-scalars",
      probe_name: "api-meta",
      url: $admission_url
    }' >"$run_dir/admission/production.json"

  jq -n \
    --argjson generation "$generation" \
    --arg host "$host" \
    --arg release_id "$release_id" \
    --arg release_identity "$release_identity" \
    --arg api_upstream "$api_upstream" \
    --arg state_dir "$state_dir" \
    '{
      desired_generation: $generation,
      environment: "production",
      host: $host,
      release_id: $release_id,
      release_identity: $release_identity,
      active_path: ($state_dir + "/active/production.json"),
      site_root: ($state_dir + "/served/production/site"),
      cdn_root: ($state_dir + "/served/production/cdn"),
      api_upstream: $api_upstream,
      served: true,
      state: "selected_local_route"
    }' >"$run_dir/routes/production.json"

  write_roots_status_for_activation "$run_dir/roots/production-${release_id}.json" "$generation" "$host" "$release_id" "$release_identity" "$active_api_closure"
  write_roots_status_for_activation "$run_dir/roots/production-${retained_release_id}.json" "$generation" "$host" "$retained_release_id" "$retained_release_identity" "$retained_api_closure"
}

write_roots_status_for_activation() {
  local path="$1"
  local generation="$2"
  local host="$3"
  local release_id="$4"
  local release_identity="$5"
  local api_closure="$6"

  jq -n \
    --argjson generation "$generation" \
    --arg host "$host" \
    --arg release_id "$release_id" \
    --arg release_identity "$release_identity" \
    --arg api_closure "$api_closure" \
    '{
      desired_generation: $generation,
      environment: "production",
      host: $host,
      release_id: $release_id,
      release_identity: $release_identity,
      root_count: 1,
      require_nix_gcroot: true,
      roots_ready: true,
      state: "roots_ready",
      roots: [
        {
          name: "api",
          root_path: ("/nix/var/nix/gcroots/fishystuff/gitops/" + $release_id + "/api"),
          store_path: $api_closure,
          observed_target: $api_closure,
          symlink_ready: true,
          nix_gcroot_ready: true
        }
      ]
    }' >"$path"
}

run_fixture_handoff() {
  local deploy_bin="$1"
  local root="$2"
  local output="$root/production-current.desired.json"
  local retained="$root/retained.json"
  local fake_mgmt="$root/fake-mgmt"
  local fake_mgmt_marker="$root/fake-mgmt-state-file"
  local summary="$root/production-current.handoff-summary.json"
  local previous_cdn_current=""
  local previous_cdn_serving=""
  local previous_api=""
  local previous_site=""
  local previous_dolt_service=""
  local active_api=""
  local active_site=""
  local active_cdn_current=""
  local active_cdn_serving=""
  local active_dolt_service=""
  local retained_roots_json=""
  local output_sha256=""
  local stale_cdn_summary=""
  local tampered_state=""
  local admission_evidence="$root/admission-evidence.json"
  local bad_admission_evidence="$root/bad-admission-evidence.json"
  local api_meta_observation="$root/api-meta.json"
  local bad_api_meta_observation="$root/bad-api-meta.json"
  local db_probe_observation="$root/db-probe.json"
  local site_cdn_probe_observation="$root/site-cdn-probe.json"
  local activation_draft="$root/production-activation.draft.desired.json"
  local stale_activation_draft="$root/stale-production-activation.draft.desired.json"
  local activation_review="$root/activation-review.txt"
  local apply_fake_mgmt="$root/fake-mgmt-apply"
  local apply_fake_mgmt_marker="$root/fake-mgmt-apply-state-file"
  local applied_state_dir="$root/applied-state"
  local applied_run_dir="$root/applied-run"
  local activation_api_upstream="http://127.0.0.1:19090"
  local activation_release_id=""
  local activation_draft_sha256=""

  previous_cdn_current="$(make_cdn_current_root "$root" "previous-cdn-current")"
  previous_cdn_serving="$(make_cdn_serving_root "$root" "previous-cdn-serving" "$previous_cdn_current" '[]')"
  previous_api="$(make_store_fixture "$root" "previous-api")"
  previous_site="$(make_store_fixture "$root" "previous-site")"
  previous_dolt_service="$(make_store_fixture "$root" "previous-dolt-service")"
  active_api="$(make_store_fixture "$root" "active-api")"
  active_site="$(make_store_fixture "$root" "active-site")"
  active_cdn_current="$(make_cdn_current_root "$root" "active-cdn-current")"
  retained_roots_json="$(jq -cn --arg root "$previous_cdn_current" '[$root]')"
  active_cdn_serving="$(make_cdn_serving_root "$root" "active-cdn-serving" "$active_cdn_current" "$retained_roots_json")"
  active_dolt_service="$(make_store_fixture "$root" "active-dolt-service")"

  write_retained_json "$retained" "$previous_api" "$previous_site" "$previous_cdn_serving" "$previous_dolt_service"
  write_fake_mgmt "$fake_mgmt" "$fake_mgmt_marker"

  FISHYSTUFF_GITOPS_RETAINED_RELEASES_FILE="$retained" \
    FISHYSTUFF_GITOPS_GENERATION=23 \
    FISHYSTUFF_GITOPS_RELEASE_GENERATION=5 \
    FISHYSTUFF_GITOPS_GIT_REV="active-git" \
    FISHYSTUFF_GITOPS_DOLT_COMMIT="active-dolt" \
    FISHYSTUFF_GITOPS_DOLT_REMOTE_URL="https://doltremoteapi.dolthub.com/fishystuff/fishystuff" \
    FISHYSTUFF_GITOPS_API_CLOSURE="$active_api" \
    FISHYSTUFF_GITOPS_SITE_CLOSURE="$active_site" \
    FISHYSTUFF_GITOPS_CDN_RUNTIME_CLOSURE="$active_cdn_serving" \
    FISHYSTUFF_GITOPS_DOLT_SERVICE_CLOSURE="$active_dolt_service" \
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

  read -r output_sha256 _ < <(sha256sum "$output")
  jq -e \
    --arg output "$output" \
    --arg output_sha256 "$output_sha256" \
    --arg active_cdn_serving "$active_cdn_serving" \
    --arg previous_cdn_serving "$previous_cdn_serving" \
    --arg previous_cdn_current "$previous_cdn_current" \
    '
    .schema == "fishystuff.gitops.production-current-handoff.v1"
    and .desired_state_path == $output
    and .desired_state_sha256 == $output_sha256
    and .cluster == "production"
    and .mode == "validate"
    and .desired_generation == 23
    and .environment.name == "production"
    and .environment.serve_requested == false
    and (.active_release.release_id | startswith("release-"))
    and .active_release.release_id == .environment.active_release
    and .active_release.dolt_commit == "active-dolt"
    and .active_release.closures.cdn_runtime == $active_cdn_serving
    and .retained_release_count == 1
    and .retained_releases[0].release_id == "previous-production-release"
    and .retained_releases[0].dolt_commit == "previous-dolt"
    and .retained_releases[0].closures.cdn_runtime == $previous_cdn_serving
    and .cdn_retention.active_cdn_runtime == $active_cdn_serving
    and .cdn_retention.retained_releases[0].release_id == "previous-production-release"
    and .cdn_retention.retained_releases[0].cdn_runtime == $previous_cdn_serving
    and .cdn_retention.retained_releases[0].expected_retained_cdn_root == $previous_cdn_current
    and .cdn_retention.retained_releases[0].retained_cdn_runtime_is_serving_root == true
    and .cdn_retention.retained_releases[0].retained_by_active_cdn_serving_root == true
    and .checks.production_current_desired_generated == true
    and .checks.desired_serving_preflight_passed == true
    and .checks.closure_paths_verified == true
    and .checks.cdn_retained_roots_verified == true
    and .checks.gitops_unify_passed == true
    and .checks.remote_deploy_performed == false
    and .checks.infrastructure_mutation_performed == false
  ' "$summary" >/dev/null

  bash scripts/recipes/gitops-check-handoff-summary.sh "$summary" "$output" >"$root/check-summary.stdout" 2>"$root/check-summary.stderr"

  expect_fail_contains \
    "activation draft requires admission evidence" \
    "gitops-production-activation-draft requires admission_file" \
    bash scripts/recipes/gitops-production-activation-draft.sh \
      "$root/no-admission.draft.desired.json" \
      "$summary" \
      "" \
      "$fake_mgmt" \
      "$deploy_bin"

  write_admission_observations "$summary" "$output" "$api_meta_observation" "$db_probe_observation" "$site_cdn_probe_observation"
  jq '.dolt_commit = "wrong-dolt-commit"' "$api_meta_observation" >"$bad_api_meta_observation"
  expect_fail_contains \
    "admission evidence writer rejects mismatched API meta" \
    "API meta observation does not match verified production handoff" \
    bash scripts/recipes/gitops-write-activation-admission-evidence.sh \
      "$root/bad-api-meta-admission.evidence.json" \
      "$summary" \
      "$activation_api_upstream" \
      "$bad_api_meta_observation" \
      "$db_probe_observation" \
      "$site_cdn_probe_observation"

  bash scripts/recipes/gitops-write-activation-admission-evidence.sh \
    "$admission_evidence" \
    "$summary" \
    "$activation_api_upstream" \
    "$api_meta_observation" \
    "$db_probe_observation" \
    "$site_cdn_probe_observation" \
    >"$root/write-admission.stdout" \
    2>"$root/write-admission.stderr"
  jq '.dolt_commit = "wrong-dolt-commit"' "$admission_evidence" >"$bad_admission_evidence"
  expect_fail_contains \
    "activation draft rejects mismatched admission evidence" \
    "admission evidence does not match verified production handoff" \
    bash scripts/recipes/gitops-production-activation-draft.sh \
      "$root/bad-admission.draft.desired.json" \
      "$summary" \
      "$bad_admission_evidence" \
      "$fake_mgmt" \
      "$deploy_bin"

  bash scripts/recipes/gitops-production-activation-draft.sh \
    "$activation_draft" \
    "$summary" \
    "$admission_evidence" \
    "$fake_mgmt" \
    "$deploy_bin" \
    >"$root/activation.stdout" \
    2>"$root/activation.stderr"
  activation_release_id="$(jq -er '.environment.active_release' "$summary")"
  jq -e \
    --arg activation_api_upstream "$activation_api_upstream" \
    --arg activation_release_id "$activation_release_id" \
    '
    .cluster == "production"
    and .mode == "local-apply"
    and .generation == 24
    and .environments.production.serve == true
    and .environments.production.active_release == $activation_release_id
    and .environments.production.api_upstream == $activation_api_upstream
    and .environments.production.admission_probe.kind == "api_meta"
    and .environments.production.admission_probe.probe_name == "api-meta"
    and .environments.production.admission_probe.url == ($activation_api_upstream + "/api/v1/meta")
    and .environments.production.admission_probe.expected_status == 200
    and .environments.production.transition.kind == "activate"
    and .environments.production.transition.from_release == ""
    and .environments.production.retained_releases == ["previous-production-release"]
  ' "$activation_draft" >/dev/null

  if [[ "$(cat "$fake_mgmt_marker")" != "$activation_draft" ]]; then
    printf '[gitops-production-current-handoff-test] fake mgmt saw wrong activation draft state file\n' >&2
    exit 1
  fi
  bash scripts/recipes/gitops-check-activation-draft.sh \
    "$activation_draft" \
    "$summary" \
    "$admission_evidence" \
    "$deploy_bin" \
    >"$root/check-activation.stdout" \
    2>"$root/check-activation.stderr"

  bash scripts/recipes/gitops-review-activation-draft.sh \
    "$activation_draft" \
    "$summary" \
    "$admission_evidence" \
    "$deploy_bin" \
    >"$activation_review" \
    2>"$root/review-activation.stderr"
  grep -F "gitops_activation_review_ok=$activation_draft" "$activation_review" >/dev/null
  grep -F "environment=production" "$activation_review" >/dev/null
  grep -F "mode=local-apply" "$activation_review" >/dev/null
  grep -F "serve=true" "$activation_review" >/dev/null
  grep -F "transition_kind=activate" "$activation_review" >/dev/null
  grep -F "release_id=$activation_release_id" "$activation_review" >/dev/null
  grep -F "dolt_commit=active-dolt" "$activation_review" >/dev/null
  grep -F "api_upstream=$activation_api_upstream" "$activation_review" >/dev/null
  grep -F "api_meta_url=$activation_api_upstream/api/v1/meta" "$activation_review" >/dev/null
  grep -F "remote_deploy_performed=false" "$activation_review" >/dev/null
  grep -F "infrastructure_mutation_performed=false" "$activation_review" >/dev/null
  activation_draft_sha256="$(awk -F= '$1 == "activation_draft_sha256" { print $2 }' "$activation_review")"
  if [[ -z "$activation_draft_sha256" ]]; then
    printf '[gitops-production-current-handoff-test] activation review did not print draft sha256\n' >&2
    exit 1
  fi

  expect_fail_contains \
    "activation apply refuses without opt-in" \
    "gitops-apply-activation-draft requires FISHYSTUFF_GITOPS_ENABLE_PRODUCTION_APPLY=1" \
    bash scripts/recipes/gitops-apply-activation-draft.sh \
      "$activation_draft" \
      "$summary" \
      "$admission_evidence" \
      auto \
      "$deploy_bin"

  expect_fail_contains \
    "activation apply requires reviewed draft hash" \
    "gitops-apply-activation-draft requires FISHYSTUFF_GITOPS_APPLY_DRAFT_SHA256" \
    env FISHYSTUFF_GITOPS_ENABLE_PRODUCTION_APPLY=1 FISHYSTUFF_GITOPS_ENABLE_LOCAL_APPLY=1 \
      bash scripts/recipes/gitops-apply-activation-draft.sh \
        "$activation_draft" \
        "$summary" \
        "$admission_evidence" \
        auto \
        "$deploy_bin"

  expect_fail_contains \
    "activation apply rejects stale reviewed draft hash" \
    "FISHYSTUFF_GITOPS_APPLY_DRAFT_SHA256 does not match activation draft" \
    env FISHYSTUFF_GITOPS_ENABLE_PRODUCTION_APPLY=1 FISHYSTUFF_GITOPS_ENABLE_LOCAL_APPLY=1 FISHYSTUFF_GITOPS_APPLY_DRAFT_SHA256=0000000000000000000000000000000000000000000000000000000000000000 \
      bash scripts/recipes/gitops-apply-activation-draft.sh \
        "$activation_draft" \
        "$summary" \
        "$admission_evidence" \
        auto \
        "$deploy_bin"

  write_fake_mgmt_apply "$apply_fake_mgmt" "$apply_fake_mgmt_marker"
  env \
    FISHYSTUFF_FAKE_MGMT_MARKER="$apply_fake_mgmt_marker" \
    FISHYSTUFF_GITOPS_ENABLE_PRODUCTION_APPLY=1 \
    FISHYSTUFF_GITOPS_ENABLE_LOCAL_APPLY=1 \
    FISHYSTUFF_GITOPS_APPLY_DRAFT_SHA256="$activation_draft_sha256" \
    bash scripts/recipes/gitops-apply-activation-draft.sh \
      "$activation_draft" \
      "$summary" \
      "$admission_evidence" \
      "$apply_fake_mgmt" \
      "$deploy_bin" \
      >"$root/apply-activation.stdout" \
      2>"$root/apply-activation.stderr"
  grep -F "gitops_activation_apply_ok=$activation_draft" "$root/apply-activation.stdout" >/dev/null
  grep -F "gitops_activation_apply_draft_sha256=$activation_draft_sha256" "$root/apply-activation.stdout" >/dev/null
  grep -F "remote_deploy_performed=false" "$root/apply-activation.stdout" >/dev/null
  grep -F "infrastructure_mutation_performed=false" "$root/apply-activation.stdout" >/dev/null
  if [[ "$(cat "$apply_fake_mgmt_marker")" != "$activation_draft" ]]; then
    printf '[gitops-production-current-handoff-test] fake mgmt apply saw wrong activation draft state file\n' >&2
    exit 1
  fi

  write_activation_served_state "$applied_state_dir" "$applied_run_dir" "$activation_draft" "$activation_release_id"
  bash scripts/recipes/gitops-verify-activation-served.sh \
    "$activation_draft" \
    "$summary" \
    "$admission_evidence" \
    "$deploy_bin" \
    "$applied_state_dir" \
    "$applied_run_dir" \
    >"$root/verify-served.stdout" \
    2>"$root/verify-served.stderr"
  grep -F "gitops_activation_served_ok=$activation_release_id" "$root/verify-served.stdout" >/dev/null
  grep -F "gitops_activation_served_state_dir=$applied_state_dir" "$root/verify-served.stdout" >/dev/null
  grep -F "remote_deploy_performed=false" "$root/verify-served.stdout" >/dev/null
  grep -F "infrastructure_mutation_performed=false" "$root/verify-served.stdout" >/dev/null
  pass "activation served verifier accepts fixture state"

  jq '.environments.production.api_upstream = "http://127.0.0.1:19999"' "$activation_draft" >"$stale_activation_draft"
  expect_fail_contains \
    "activation draft verifier rejects stale API upstream" \
    "activation draft does not match verified handoff and admission evidence" \
    bash scripts/recipes/gitops-check-activation-draft.sh \
      "$stale_activation_draft" \
      "$summary" \
      "$admission_evidence" \
      "$deploy_bin"

  stale_cdn_summary="$root/stale-cdn-retention.handoff-summary.json"
  jq '.cdn_retention.active_retained_roots = []' "$summary" >"$stale_cdn_summary"
  expect_fail_contains \
    "handoff summary rejects stale CDN retention data" \
    "handoff summary CDN retention data does not match the active CDN manifest" \
    bash scripts/recipes/gitops-check-handoff-summary.sh "$stale_cdn_summary" "$output"

  tampered_state="$root/tampered.desired.json"
  jq '.generation = 999' "$output" >"$tampered_state"
  mv "$tampered_state" "$output"
  expect_fail_contains \
    "handoff summary rejects desired hash mismatch" \
    "handoff summary desired_state_sha256 does not match checked state file" \
    bash scripts/recipes/gitops-check-handoff-summary.sh "$summary" "$output"
}

run_missing_cdn_retention_refusal() {
  local deploy_bin="$1"
  local root="$2"
  local output="$root/missing-cdn-retention.desired.json"
  local retained="$root/missing-cdn-retention.retained.json"
  local summary="$root/missing-cdn-retention.handoff-summary.json"
  local previous_cdn_current=""
  local previous_cdn_serving=""
  local previous_api=""
  local previous_site=""
  local previous_dolt_service=""
  local active_api=""
  local active_site=""
  local active_cdn_current=""
  local active_cdn_serving=""
  local active_dolt_service=""

  previous_cdn_current="$(make_cdn_current_root "$root" "previous-missing-retention-cdn-current")"
  previous_cdn_serving="$(make_cdn_serving_root "$root" "previous-missing-retention-cdn-serving" "$previous_cdn_current" '[]')"
  previous_api="$(make_store_fixture "$root" "previous-missing-retention-api")"
  previous_site="$(make_store_fixture "$root" "previous-missing-retention-site")"
  previous_dolt_service="$(make_store_fixture "$root" "previous-missing-retention-dolt-service")"
  active_api="$(make_store_fixture "$root" "active-missing-retention-api")"
  active_site="$(make_store_fixture "$root" "active-missing-retention-site")"
  active_cdn_current="$(make_cdn_current_root "$root" "active-missing-retention-cdn-current")"
  active_cdn_serving="$(make_cdn_serving_root "$root" "active-missing-retention-cdn-serving" "$active_cdn_current" '[]')"
  active_dolt_service="$(make_store_fixture "$root" "active-missing-retention-dolt-service")"
  write_retained_json "$retained" "$previous_api" "$previous_site" "$previous_cdn_serving" "$previous_dolt_service"

  expect_fail_contains \
    "missing active CDN retained root is refused" \
    "active CDN serving root does not retain previous-production-release CDN root" \
    env \
      FISHYSTUFF_GITOPS_RETAINED_RELEASES_FILE="$retained" \
      FISHYSTUFF_GITOPS_GENERATION=25 \
      FISHYSTUFF_GITOPS_RELEASE_GENERATION=7 \
      FISHYSTUFF_GITOPS_GIT_REV="active-git-missing-retention" \
      FISHYSTUFF_GITOPS_DOLT_COMMIT="active-dolt-missing-retention" \
      FISHYSTUFF_GITOPS_DOLT_REMOTE_URL="https://doltremoteapi.dolthub.com/fishystuff/fishystuff" \
      FISHYSTUFF_GITOPS_API_CLOSURE="$active_api" \
      FISHYSTUFF_GITOPS_SITE_CLOSURE="$active_site" \
      FISHYSTUFF_GITOPS_CDN_RUNTIME_CLOSURE="$active_cdn_serving" \
      FISHYSTUFF_GITOPS_DOLT_SERVICE_CLOSURE="$active_dolt_service" \
      bash scripts/recipes/gitops-production-current-handoff.sh \
        "$output" \
        main \
        /run/current-system/sw/bin/true \
        "$deploy_bin" \
        "$summary"
}

run_missing_closure_path_refusal() {
  local deploy_bin="$1"
  local root="$2"
  local output="$root/missing-closure.desired.json"
  local retained="$root/missing-closure.retained.json"
  local summary="$root/missing-closure.handoff-summary.json"
  local previous_cdn_current=""
  local previous_cdn_serving=""
  local previous_api=""
  local previous_site=""
  local previous_dolt_service=""
  local active_site=""
  local active_cdn_current=""
  local active_cdn_serving=""
  local active_dolt_service=""
  local retained_roots_json=""

  previous_cdn_current="$(make_cdn_current_root "$root" "previous-missing-closure-cdn-current")"
  previous_cdn_serving="$(make_cdn_serving_root "$root" "previous-missing-closure-cdn-serving" "$previous_cdn_current" '[]')"
  previous_api="$(make_store_fixture "$root" "previous-missing-closure-api")"
  previous_site="$(make_store_fixture "$root" "previous-missing-closure-site")"
  previous_dolt_service="$(make_store_fixture "$root" "previous-missing-closure-dolt-service")"
  active_site="$(make_store_fixture "$root" "active-missing-closure-site")"
  active_cdn_current="$(make_cdn_current_root "$root" "active-missing-closure-cdn-current")"
  retained_roots_json="$(jq -cn --arg root "$previous_cdn_current" '[$root]')"
  active_cdn_serving="$(make_cdn_serving_root "$root" "active-missing-closure-cdn-serving" "$active_cdn_current" "$retained_roots_json")"
  active_dolt_service="$(make_store_fixture "$root" "active-missing-closure-dolt-service")"
  write_retained_json "$retained" "$previous_api" "$previous_site" "$previous_cdn_serving" "$previous_dolt_service"

  expect_fail_contains \
    "missing active closure path is refused" \
    "handoff closure path does not exist" \
    env \
      FISHYSTUFF_GITOPS_RETAINED_RELEASES_FILE="$retained" \
      FISHYSTUFF_GITOPS_GENERATION=26 \
      FISHYSTUFF_GITOPS_RELEASE_GENERATION=8 \
      FISHYSTUFF_GITOPS_GIT_REV="active-git-missing-closure" \
      FISHYSTUFF_GITOPS_DOLT_COMMIT="active-dolt-missing-closure" \
      FISHYSTUFF_GITOPS_DOLT_REMOTE_URL="https://doltremoteapi.dolthub.com/fishystuff/fishystuff" \
      FISHYSTUFF_GITOPS_API_CLOSURE="/nix/store/example-missing-active-api" \
      FISHYSTUFF_GITOPS_SITE_CLOSURE="$active_site" \
      FISHYSTUFF_GITOPS_CDN_RUNTIME_CLOSURE="$active_cdn_serving" \
      FISHYSTUFF_GITOPS_DOLT_SERVICE_CLOSURE="$active_dolt_service" \
      bash scripts/recipes/gitops-production-current-handoff.sh \
        "$output" \
        main \
        /run/current-system/sw/bin/true \
        "$deploy_bin" \
        "$summary"
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
  local previous_cdn_current=""
  local previous_cdn_serving=""
  local previous_api=""
  local previous_site=""
  local previous_dolt_service=""
  local active_api=""
  local active_site=""
  local active_cdn_current=""
  local active_cdn_serving=""
  local active_dolt_service=""
  local retained_roots_json=""
  local output_sha256=""

  previous_cdn_current="$(make_cdn_current_root "$root" "previous-from-served-cdn-current")"
  previous_cdn_serving="$(make_cdn_serving_root "$root" "previous-from-served-cdn-serving" "$previous_cdn_current" '[]')"
  previous_api="$(make_store_fixture "$root" "previous-from-served-api")"
  previous_site="$(make_store_fixture "$root" "previous-from-served-site")"
  previous_dolt_service="$(make_store_fixture "$root" "previous-from-served-dolt-service")"
  active_api="$(make_store_fixture "$root" "active-from-served-api")"
  active_site="$(make_store_fixture "$root" "active-from-served-site")"
  active_cdn_current="$(make_cdn_current_root "$root" "active-from-served-cdn-current")"
  retained_roots_json="$(jq -cn --arg root "$previous_cdn_current" '[$root]')"
  active_cdn_serving="$(make_cdn_serving_root "$root" "active-from-served-cdn-serving" "$active_cdn_current" "$retained_roots_json")"
  active_dolt_service="$(make_store_fixture "$root" "active-from-served-dolt-service")"

  write_served_rollback_set_state "$state_dir" "$previous_api" "$previous_site" "$previous_cdn_serving" "$previous_dolt_service"
  write_fake_mgmt "$fake_mgmt" "$fake_mgmt_marker"

  FISHYSTUFF_GITOPS_GENERATION=24 \
    FISHYSTUFF_GITOPS_RELEASE_GENERATION=6 \
    FISHYSTUFF_GITOPS_GIT_REV="active-git-from-served" \
    FISHYSTUFF_GITOPS_DOLT_COMMIT="active-dolt-from-served" \
    FISHYSTUFF_GITOPS_DOLT_REMOTE_URL="https://doltremoteapi.dolthub.com/fishystuff/fishystuff" \
    FISHYSTUFF_GITOPS_API_CLOSURE="$active_api" \
    FISHYSTUFF_GITOPS_SITE_CLOSURE="$active_site" \
    FISHYSTUFF_GITOPS_CDN_RUNTIME_CLOSURE="$active_cdn_serving" \
    FISHYSTUFF_GITOPS_DOLT_SERVICE_CLOSURE="$active_dolt_service" \
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

  jq -e \
    --arg previous_api "$previous_api" \
    --arg previous_cdn_serving "$previous_cdn_serving" \
    '
    .[0].release_id == "previous-production-release"
    and .[0].dolt_commit == "previous-dolt"
    and .[0].api_closure == $previous_api
    and .[0].cdn_runtime_closure == $previous_cdn_serving
  ' "$retained" >/dev/null

  jq -e '
    .cluster == "production"
    and .generation == 24
    and .environments.production.retained_releases == ["previous-production-release"]
    and .releases[.environments.production.active_release].generation == 6
    and .releases[.environments.production.active_release].dolt_commit == "active-dolt-from-served"
  ' "$output" >/dev/null

  read -r output_sha256 _ < <(sha256sum "$output")
  jq -e \
    --arg output "$output" \
    --arg output_sha256 "$output_sha256" \
    --arg active_cdn_serving "$active_cdn_serving" \
    --arg previous_cdn_current "$previous_cdn_current" \
    '
    .desired_state_path == $output
    and .desired_state_sha256 == $output_sha256
    and .retained_release_count == 1
    and .retained_releases[0].release_id == "previous-production-release"
    and .active_release.dolt_commit == "active-dolt-from-served"
    and .active_release.closures.cdn_runtime == $active_cdn_serving
    and .cdn_retention.retained_releases[0].expected_retained_cdn_root == $previous_cdn_current
    and .checks.desired_serving_preflight_passed == true
    and .checks.closure_paths_verified == true
    and .checks.cdn_retained_roots_verified == true
    and .checks.gitops_unify_passed == true
  ' "$summary" >/dev/null

  bash scripts/recipes/gitops-check-handoff-summary.sh "$summary" "$output" >"$root/from-served-check-summary.stdout" 2>"$root/from-served-check-summary.stderr"

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

missing_cdn_retention_root="$(mktemp -d)"
run_missing_cdn_retention_refusal "$deploy_bin" "$missing_cdn_retention_root"

missing_closure_root="$(mktemp -d)"
run_missing_closure_path_refusal "$deploy_bin" "$missing_closure_root"

from_served_root="$(mktemp -d)"
run_fixture_from_served "$deploy_bin" "$from_served_root"
pass "served rollback-set feeds retained JSON and checked handoff"

printf '[gitops-production-current-handoff-test] %s checks passed\n' "$pass_count"
