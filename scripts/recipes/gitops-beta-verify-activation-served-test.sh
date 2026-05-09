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
  printf '[gitops-beta-verify-activation-served-test] pass: %s\n' "$1"
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
    printf '[gitops-beta-verify-activation-served-test] expected failure: %s\n' "$name" >&2
    exit 1
  fi
  if ! grep -F "$expected" "$stderr" >/dev/null; then
    printf '[gitops-beta-verify-activation-served-test] expected %s stderr to contain %q\n' "$name" "$expected" >&2
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

make_fake_served_deploy() {
  local path="$1"
  local real_deploy_bin="$2"
  cat >"$path" <<EOF
#!/usr/bin/env bash
set -euo pipefail
case "\$*" in
  gitops\ check-desired-serving\ --state\ *\ --environment\ beta)
    printf 'fake_beta_desired_serving_ok\n'
    ;;
  gitops\ inspect-served\ *)
    exec "$real_deploy_bin" "\$@"
    ;;
  *)
    echo "unexpected fake beta served fishystuff_deploy args: \$*" >&2
    exit 2
    ;;
esac
EOF
  chmod +x "$path"
}

write_roots_status() {
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
      environment: "beta",
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
          root_path: ("/nix/var/nix/gcroots/fishystuff/gitops-beta/" + $release_id + "/api"),
          store_path: $api_closure,
          observed_target: $api_closure,
          symlink_ready: true,
          nix_gcroot_ready: true
        }
      ]
    }' >"$path"
}

write_beta_served_state() {
  local state_dir="$1"
  local run_dir="$2"
  local draft_file="$3"
  local release_id="$4"
  local retained_release_id="previous-beta-release"
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
  local rollback_member="$state_dir/rollback-set/beta/${retained_release_id}.json"

  generation="$(jq -er '.generation' "$draft_file")"
  host="$(jq -er '.environments.beta.host' "$draft_file")"
  api_upstream="$(jq -er '.environments.beta.api_upstream' "$draft_file")"
  admission_url="$(jq -er '.environments.beta.admission_probe.url' "$draft_file")"
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
    "$state_dir/rollback-set/beta" \
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
      environment: "beta",
      host: $host,
      phase: "served",
      transition_kind: "activate",
      rollback_from_release: "",
      rollback_to_release: "",
      rollback_reason: "verified beta handoff admission",
      admission_state: "passed_fixture",
      retained_release_ids: ["previous-beta-release"],
      retained_dolt_status_paths: [],
      rollback_available: true,
      rollback_primary_release_id: "previous-beta-release",
      rollback_retained_count: 1,
      served: true,
      failure_reason: ""
    }' >"$state_dir/status/beta.json"

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
      environment: "beta",
      host: $host,
      release_id: $release_id,
      release_identity: $release_identity,
      instance_name: ("beta-" + $release_id),
      site_content: $active_site_closure,
      cdn_runtime_content: $active_cdn_closure,
      api_upstream: $api_upstream,
      site_link: ($state_dir + "/served/beta/site"),
      cdn_link: ($state_dir + "/served/beta/cdn"),
      retained_release_ids: ["previous-beta-release"],
      retained_dolt_status_paths: [],
      transition_kind: "activate",
      rollback_from_release: "",
      rollback_to_release: "",
      rollback_reason: "verified beta handoff admission",
      admission_state: "passed_fixture",
      served: true,
      route_state: "selected_local_symlinks"
    }' >"$state_dir/active/beta.json"

  jq -n \
    --argjson generation "$generation" \
    --arg host "$host" \
    --arg release_id "$release_id" \
    --arg release_identity "$release_identity" \
    --arg rollback_member "$rollback_member" \
    '{
      desired_generation: $generation,
      environment: "beta",
      host: $host,
      current_release_id: $release_id,
      current_release_identity: $release_identity,
      retained_release_count: 1,
      retained_release_ids: ["previous-beta-release"],
      retained_release_document_paths: [$rollback_member],
      rollback_set_available: true,
      rollback_set_state: "retained_hot_release_set"
    }' >"$state_dir/rollback-set/beta.json"

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
      environment: "beta",
      host: $host,
      current_release_id: $release_id,
      current_release_identity: $release_identity,
      rollback_release_id: "previous-beta-release",
      rollback_release_identity: $retained_release_identity,
      rollback_api_bundle: $retained_api_closure,
      rollback_dolt_service_bundle: $retained_dolt_service_closure,
      rollback_site_content: $retained_site_closure,
      rollback_cdn_runtime_content: $retained_cdn_closure,
      rollback_dolt_commit: "beta-retained-dolt",
      rollback_dolt_materialization: "fetch_pin",
      rollback_dolt_cache_dir: "/var/lib/fishystuff/gitops-beta/dolt-cache/fishystuff",
      rollback_dolt_release_ref: "fishystuff/gitops-beta/previous-beta-release",
      rollback_available: true,
      rollback_state: "retained_hot_release"
    }' >"$state_dir/rollback/beta.json"

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
      environment: "beta",
      host: $host,
      current_release_id: $release_id,
      release_id: "previous-beta-release",
      release_identity: $retained_release_identity,
      api_bundle: $retained_api_closure,
      dolt_service_bundle: $retained_dolt_service_closure,
      site_content: $retained_site_closure,
      cdn_runtime_content: $retained_cdn_closure,
      dolt_commit: "beta-retained-dolt",
      dolt_materialization: "fetch_pin",
      dolt_cache_dir: "/var/lib/fishystuff/gitops-beta/dolt-cache/fishystuff",
      dolt_release_ref: "fishystuff/gitops-beta/previous-beta-release",
      dolt_status_path: "/run/fishystuff/gitops-beta/dolt/beta-previous-beta-release.json",
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
      environment: "beta",
      host: $host,
      release_id: $release_id,
      release_identity: $release_identity,
      site_content: $active_site_closure,
      cdn_runtime_content: $active_cdn_closure,
      admission_state: "passed_fixture",
      probe: "http-json-scalars",
      probe_name: "api-meta",
      url: $admission_url
    }' >"$run_dir/admission/beta.json"

  jq -n \
    --argjson generation "$generation" \
    --arg host "$host" \
    --arg release_id "$release_id" \
    --arg release_identity "$release_identity" \
    --arg api_upstream "$api_upstream" \
    --arg state_dir "$state_dir" \
    '{
      desired_generation: $generation,
      environment: "beta",
      host: $host,
      release_id: $release_id,
      release_identity: $release_identity,
      active_path: ($state_dir + "/active/beta.json"),
      site_root: ($state_dir + "/served/beta/site"),
      cdn_root: ($state_dir + "/served/beta/cdn"),
      api_upstream: $api_upstream,
      served: true,
      state: "selected_local_route"
    }' >"$run_dir/routes/beta.json"

  write_roots_status "$run_dir/roots/beta-${release_id}.json" "$generation" "$host" "$release_id" "$release_identity" "$active_api_closure"
  write_roots_status "$run_dir/roots/beta-${retained_release_id}.json" "$generation" "$host" "$retained_release_id" "$retained_release_identity" "$retained_api_closure"
}

if [[ "${FISHYSTUFF_GITOPS_BETA_VERIFY_ACTIVATION_SERVED_TEST_SOURCE_ONLY:-}" == "1" ]]; then
  return 0 2>/dev/null || exit 0
fi

root="$(mktemp -d)"
deploy_bin="$(require_deploy_bin)"
make_fixture "$root"
make_fake_mgmt "${root}/mgmt"
make_fake_deploy "${root}/fishystuff_deploy"
make_fake_served_deploy "${root}/fishystuff_deploy_served" "$deploy_bin"

state="$(cat "${root}/state.path")"
summary="$(cat "${root}/summary.path")"
api_meta="$(cat "${root}/api-meta.path")"
db_probe="$(cat "${root}/db-probe.path")"
site_cdn_probe="$(cat "${root}/site-cdn-probe.path")"
admission="${root}/beta-admission.evidence.json"
draft="${root}/beta-activation.draft.desired.json"
fake_mgmt_marker="${root}/fake-mgmt-state"
export FISHYSTUFF_FAKE_MGMT_MARKER="$fake_mgmt_marker"

bash scripts/recipes/gitops-beta-write-activation-admission-evidence.sh \
  "$admission" \
  "$summary" \
  "http://127.0.0.1:18192" \
  "$api_meta" \
  "$db_probe" \
  "$site_cdn_probe" \
  >"${root}/write-admission.stdout" \
  2>"${root}/write-admission.stderr"

bash scripts/recipes/gitops-beta-activation-draft.sh \
  "$draft" \
  "$summary" \
  "$admission" \
  "${root}/mgmt" \
  "${root}/fishystuff_deploy" \
  >"${root}/activation.stdout" \
  2>"${root}/activation.stderr"

release_id="$(jq -er '.environments.beta.active_release' "$draft")"
write_beta_served_state "${root}/state-dir" "${root}/run-dir" "$draft" "$release_id"

bash scripts/recipes/gitops-beta-verify-activation-served.sh \
  "$draft" \
  "$summary" \
  "$admission" \
  "${root}/fishystuff_deploy_served" \
  "${root}/state-dir" \
  "${root}/run-dir" >"${root}/verify.stdout"

grep -F "gitops_activation_served_ok=${release_id}" "${root}/verify.stdout" >/dev/null
grep -F "gitops_activation_served_environment=beta" "${root}/verify.stdout" >/dev/null
grep -F "gitops_activation_served_state_dir=${root}/state-dir" "${root}/verify.stdout" >/dev/null
grep -F "remote_deploy_performed=false" "${root}/verify.stdout" >/dev/null
pass "valid beta served verification"

if grep -F "production" "${root}/verify.stdout" >/dev/null; then
  printf '[gitops-beta-verify-activation-served-test] beta served verification unexpectedly mentions production\n' >&2
  cat "${root}/verify.stdout" >&2
  exit 1
fi
pass "no production strings in beta served verification"

production_summary="${root}/production-summary.json"
jq '.environment.name = "production"' "$summary" >"$production_summary"
expect_fail_contains \
  "reject production handoff summary" \
  "requires a beta handoff summary" \
  bash scripts/recipes/gitops-beta-verify-activation-served.sh \
    "$draft" \
    "$production_summary" \
    "$admission" \
    "${root}/fishystuff_deploy_served" \
    "${root}/state-dir" \
    "${root}/run-dir"

jq '.release_id = "wrong-release"' "${root}/state-dir/status/beta.json" >"${root}/state-dir/status/beta.json.tmp"
mv "${root}/state-dir/status/beta.json.tmp" "${root}/state-dir/status/beta.json"
expect_fail_contains \
  "reject stale beta served status" \
  "status release_id was wrong-release" \
  bash scripts/recipes/gitops-beta-verify-activation-served.sh \
    "$draft" \
    "$summary" \
    "$admission" \
    "${root}/fishystuff_deploy_served" \
    "${root}/state-dir" \
    "${root}/run-dir"

printf '[gitops-beta-verify-activation-served-test] %s checks passed\n' "$pass_count"
