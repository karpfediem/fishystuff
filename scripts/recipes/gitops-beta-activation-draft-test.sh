#!/usr/bin/env bash
set -euo pipefail

SCRIPT_PATH="$(readlink -f "${BASH_SOURCE[0]}")"
SCRIPT_DIR="$(cd "$(dirname "$SCRIPT_PATH")" && pwd)"
source "${SCRIPT_DIR}/lib/common.sh"

cd "$RECIPE_REPO_ROOT"

pass_count=0

pass() {
  printf '[gitops-beta-activation-draft-test] pass: %s\n' "$1"
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
    printf '[gitops-beta-activation-draft-test] expected failure: %s\n' "$name" >&2
    exit 1
  fi
  if ! grep -F "$expected" "$stderr" >/dev/null; then
    printf '[gitops-beta-activation-draft-test] expected %s stderr to contain %q\n' "$name" "$expected" >&2
    cat "$stderr" >&2
    exit 1
  fi
  pass "$name"
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

make_fake_mgmt() {
  local path="$1"
  cat >"$path" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
if [[ "$*" != "run --tmp-prefix --no-network --no-pgp lang --only-unify main.mcl" ]]; then
  echo "unexpected fake mgmt args: $*" >&2
  exit 2
fi
printf '%s\n' "${FISHYSTUFF_GITOPS_STATE_FILE:-}" >"${FISHYSTUFF_FAKE_MGMT_MARKER:?}"
EOF
  chmod +x "$path"
}

make_fake_deploy() {
  local path="$1"
  cat >"$path" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
if [[ "$*" != gitops\ check-desired-serving\ --state\ *\ --environment\ beta ]]; then
  echo "unexpected fake fishystuff_deploy args: $*" >&2
  exit 2
fi
printf 'fake_beta_desired_serving_ok\n'
EOF
  chmod +x "$path"
}

make_fixture() {
  local root="$1"
  local active_id="beta-release"
  local retained_id="previous-beta-release"
  local active_api="${root}/active-api"
  local active_site="${root}/active-site"
  local active_cdn="${root}/active-cdn"
  local active_cdn_current="${root}/active-cdn-current"
  local active_dolt="${root}/active-dolt-service"
  local retained_api="${root}/retained-api"
  local retained_site="${root}/retained-site"
  local retained_cdn="${root}/retained-cdn"
  local retained_cdn_current="${root}/retained-cdn-current"
  local retained_dolt="${root}/retained-dolt-service"
  local state_file="${root}/beta-current.desired.json"
  local summary_file="${root}/beta-current.handoff-summary.json"
  local api_meta_observation="${root}/api-meta.json"
  local db_probe_observation="${root}/db-probe.json"
  local site_cdn_probe_observation="${root}/site-cdn-probe.json"
  local state_sha=""
  local identity=""

  mkdir -p "$active_api" "$active_site" "$active_cdn" "$active_cdn_current" "$active_dolt"
  mkdir -p "$retained_api" "$retained_site" "$retained_cdn" "$retained_cdn_current" "$retained_dolt"

  jq -n \
    --arg current_root "$retained_cdn_current" \
    '{
      current_root: $current_root,
      retained_roots: [],
      retained_root_count: 0
    }' >"${retained_cdn}/cdn-serving-manifest.json"
  jq -n \
    --arg current_root "$active_cdn_current" \
    --arg retained_root "$retained_cdn_current" \
    '{
      current_root: $current_root,
      retained_roots: [$retained_root],
      retained_root_count: 1
    }' >"${active_cdn}/cdn-serving-manifest.json"

  jq -n \
    --arg active_api "$active_api" \
    --arg active_site "$active_site" \
    --arg active_cdn "$active_cdn" \
    --arg active_dolt "$active_dolt" \
    --arg retained_api "$retained_api" \
    --arg retained_site "$retained_site" \
    --arg retained_cdn "$retained_cdn" \
    --arg retained_dolt "$retained_dolt" \
    --arg active_id "$active_id" \
    --arg retained_id "$retained_id" \
    '{
      cluster: "beta",
      generation: 7,
      mode: "validate",
      hosts: {
        "beta-single-host": {
          enabled: true,
          role: "single-site",
          hostname: "beta-single-host"
        }
      },
      releases: {
        ($active_id): {
          generation: 5,
          git_rev: "beta-active-git",
          dolt_commit: "beta-active-dolt",
          closures: {
            api: { enabled: true, store_path: $active_api, gcroot_path: "/nix/var/nix/gcroots/fishystuff/gitops-beta/beta-release/api" },
            site: { enabled: true, store_path: $active_site, gcroot_path: "/nix/var/nix/gcroots/fishystuff/gitops-beta/beta-release/site" },
            cdn_runtime: { enabled: true, store_path: $active_cdn, gcroot_path: "/nix/var/nix/gcroots/fishystuff/gitops-beta/beta-release/cdn-runtime" },
            dolt_service: { enabled: true, store_path: $active_dolt, gcroot_path: "/nix/var/nix/gcroots/fishystuff/gitops-beta/beta-release/dolt-service" }
          },
          dolt: {
            repository: "fishystuff/fishystuff",
            commit: "beta-active-dolt",
            branch_context: "beta",
            mode: "read_only",
            materialization: "fetch_pin",
            remote_url: "file:///tmp/fishystuff-beta-dolt-remote",
            cache_dir: "/var/lib/fishystuff/gitops-beta/dolt-cache/fishystuff",
            release_ref: "fishystuff/gitops-beta/beta-release"
          }
        },
        ($retained_id): {
          generation: 4,
          git_rev: "beta-retained-git",
          dolt_commit: "beta-retained-dolt",
          closures: {
            api: { enabled: true, store_path: $retained_api, gcroot_path: "/nix/var/nix/gcroots/fishystuff/gitops-beta/previous-beta-release/api" },
            site: { enabled: true, store_path: $retained_site, gcroot_path: "/nix/var/nix/gcroots/fishystuff/gitops-beta/previous-beta-release/site" },
            cdn_runtime: { enabled: true, store_path: $retained_cdn, gcroot_path: "/nix/var/nix/gcroots/fishystuff/gitops-beta/previous-beta-release/cdn-runtime" },
            dolt_service: { enabled: true, store_path: $retained_dolt, gcroot_path: "/nix/var/nix/gcroots/fishystuff/gitops-beta/previous-beta-release/dolt-service" }
          },
          dolt: {
            repository: "fishystuff/fishystuff",
            commit: "beta-retained-dolt",
            branch_context: "beta",
            mode: "read_only",
            materialization: "fetch_pin",
            remote_url: "file:///tmp/fishystuff-beta-dolt-remote",
            cache_dir: "/var/lib/fishystuff/gitops-beta/dolt-cache/fishystuff",
            release_ref: "fishystuff/gitops-beta/previous-beta-release"
          }
        }
      },
      environments: {
        beta: {
          enabled: true,
          strategy: "single_active",
          host: "beta-single-host",
          active_release: $active_id,
          retained_releases: [$retained_id],
          serve: false
        }
      }
    }' >"$state_file"

  read -r state_sha _ < <(sha256sum "$state_file")
  jq -n \
    --arg state_file "$state_file" \
    --arg state_sha "$state_sha" \
    --arg active_id "$active_id" \
    --arg retained_id "$retained_id" \
    --arg active_api "$active_api" \
    --arg active_site "$active_site" \
    --arg active_cdn "$active_cdn" \
    --arg active_dolt "$active_dolt" \
    --arg retained_api "$retained_api" \
    --arg retained_site "$retained_site" \
    --arg retained_cdn "$retained_cdn" \
    --arg retained_dolt "$retained_dolt" \
    --arg active_cdn_manifest "${active_cdn}/cdn-serving-manifest.json" \
    --arg active_cdn_current "$active_cdn_current" \
    --arg retained_cdn_current "$retained_cdn_current" \
    '{
      schema: "fishystuff.gitops.current-handoff.v1",
      desired_state_path: $state_file,
      desired_state_sha256: $state_sha,
      cluster: "beta",
      mode: "validate",
      desired_generation: 7,
      environment: {
        name: "beta",
        host: "beta-single-host",
        serve_requested: false,
        active_release: $active_id,
        retained_releases: [$retained_id]
      },
      active_release: {
        release_id: $active_id,
        generation: 5,
        git_rev: "beta-active-git",
        dolt_commit: "beta-active-dolt",
        closures: {
          api: $active_api,
          site: $active_site,
          cdn_runtime: $active_cdn,
          dolt_service: $active_dolt
        },
        dolt: {
          materialization: "fetch_pin",
          branch_context: "beta",
          cache_dir: "/var/lib/fishystuff/gitops-beta/dolt-cache/fishystuff",
          release_ref: "fishystuff/gitops-beta/beta-release"
        }
      },
      retained_release_count: 1,
      retained_releases: [
        {
          release_id: $retained_id,
          generation: 4,
          git_rev: "beta-retained-git",
          dolt_commit: "beta-retained-dolt",
          closures: {
            api: $retained_api,
            site: $retained_site,
            cdn_runtime: $retained_cdn,
            dolt_service: $retained_dolt
          },
          dolt: {
            materialization: "fetch_pin",
            branch_context: "beta",
            cache_dir: "/var/lib/fishystuff/gitops-beta/dolt-cache/fishystuff",
            release_ref: "fishystuff/gitops-beta/previous-beta-release"
          }
        }
      ],
      cdn_retention: {
        active_cdn_runtime: $active_cdn,
        active_manifest: $active_cdn_manifest,
        active_current_root: $active_cdn_current,
        active_retained_roots: [$retained_cdn_current],
        retained_releases: [
          {
            release_id: $retained_id,
            cdn_runtime: $retained_cdn,
            retained_cdn_runtime_is_serving_root: true,
            expected_retained_cdn_root: $retained_cdn_current,
            retained_by_active_cdn_serving_root: true
          }
        ]
      },
      checks: {
        current_desired_generated: true,
        desired_serving_preflight_passed: true,
        desired_serving_preflight_skipped: false,
        closure_paths_verified: true,
        cdn_manifest_verified: true,
        cdn_retained_roots_verified: true,
        gitops_unify_passed: true,
        remote_deploy_performed: false,
        infrastructure_mutation_performed: false
      }
    }' >"$summary_file"

  identity="$(release_identity_from_state "$state_file" "$active_id")"
  jq -n \
    --arg active_id "$active_id" \
    --arg identity "$identity" \
    '{
      release_id: $active_id,
      release_identity: $identity,
      dolt_commit: "beta-active-dolt"
    }' >"$api_meta_observation"
  jq -n '{ name: "beta-db-fixture", passed: true }' >"$db_probe_observation"
  jq -n '{ name: "beta-site-cdn-fixture", passed: true }' >"$site_cdn_probe_observation"

  printf '%s\n' "$state_file" >"${root}/state.path"
  printf '%s\n' "$summary_file" >"${root}/summary.path"
  printf '%s\n' "$api_meta_observation" >"${root}/api-meta.path"
  printf '%s\n' "$db_probe_observation" >"${root}/db-probe.path"
  printf '%s\n' "$site_cdn_probe_observation" >"${root}/site-cdn-probe.path"
}

if [[ "${FISHYSTUFF_GITOPS_BETA_ACTIVATION_DRAFT_TEST_SOURCE_ONLY:-}" == "1" ]]; then
  return 0 2>/dev/null || exit 0
fi

root="$(mktemp -d)"
make_fixture "$root"
make_fake_mgmt "${root}/mgmt"
make_fake_deploy "${root}/fishystuff_deploy"

state="$(cat "${root}/state.path")"
summary="$(cat "${root}/summary.path")"
api_meta="$(cat "${root}/api-meta.path")"
db_probe="$(cat "${root}/db-probe.path")"
site_cdn_probe="$(cat "${root}/site-cdn-probe.path")"
admission="${root}/beta-admission.evidence.json"
draft="${root}/beta-activation.draft.desired.json"
review="${root}/beta-activation.review"
fake_mgmt_marker="${root}/fake-mgmt-state"
activation_api_upstream="http://127.0.0.1:18192"
export FISHYSTUFF_FAKE_MGMT_MARKER="$fake_mgmt_marker"

bash scripts/recipes/gitops-check-handoff-summary.sh "$summary" "$state" >"${root}/check-summary.stdout" 2>"${root}/check-summary.stderr"
pass "generic beta handoff summary accepted"

bash scripts/recipes/gitops-beta-write-activation-admission-evidence.sh \
  "$admission" \
  "$summary" \
  "$activation_api_upstream" \
  "$api_meta" \
  "$db_probe" \
  "$site_cdn_probe" \
  >"${root}/write-admission.stdout" \
  2>"${root}/write-admission.stderr"
jq -e \
  --arg activation_api_upstream "$activation_api_upstream" \
  '.schema == "fishystuff.gitops.activation-admission.v1"
  and .environment == "beta"
  and .api_upstream == $activation_api_upstream
  and .api_meta.url == ($activation_api_upstream + "/api/v1/meta")
  and .db_backed_probe.name == "beta-db-fixture"
  and .site_cdn_probe.name == "beta-site-cdn-fixture"' \
  "$admission" >/dev/null
pass "beta admission evidence"

production_summary="${root}/production-summary.json"
jq '.environment.name = "production"' "$summary" >"$production_summary"
expect_fail_contains \
  "beta admission wrapper rejects production summary" \
  "requires a beta handoff summary" \
  bash scripts/recipes/gitops-beta-write-activation-admission-evidence.sh \
    "${root}/wrong-env-admission.json" \
    "$production_summary" \
    "$activation_api_upstream" \
    "$api_meta" \
    "$db_probe" \
    "$site_cdn_probe"

expect_fail_contains \
  "beta activation wrapper rejects production summary" \
  "requires a beta handoff summary" \
  bash scripts/recipes/gitops-beta-activation-draft.sh \
    "${root}/wrong-env-draft.json" \
    "$production_summary" \
    "$admission" \
    "${root}/mgmt" \
    "${root}/fishystuff_deploy"

bash scripts/recipes/gitops-beta-activation-draft.sh \
  "$draft" \
  "$summary" \
  "$admission" \
  "${root}/mgmt" \
  "${root}/fishystuff_deploy" \
  >"${root}/activation.stdout" \
  2>"${root}/activation.stderr"
jq -e \
  --arg activation_api_upstream "$activation_api_upstream" \
  '.cluster == "beta"
  and .mode == "local-apply"
  and .generation == 8
  and .environments.beta.serve == true
  and .environments.beta.active_release == "beta-release"
  and .environments.beta.retained_releases == ["previous-beta-release"]
  and .environments.beta.api_upstream == $activation_api_upstream
  and .environments.beta.admission_probe.kind == "api_meta"
  and .environments.beta.admission_probe.url == ($activation_api_upstream + "/api/v1/meta")
  and .environments.beta.transition.kind == "activate"
  and .environments.beta.transition.from_release == ""
  and .environments.beta.transition.reason == "verified beta handoff admission"' \
  "$draft" >/dev/null
if [[ "$(cat "$fake_mgmt_marker")" != "$draft" ]]; then
  printf '[gitops-beta-activation-draft-test] fake mgmt saw wrong state file\n' >&2
  exit 1
fi
pass "beta activation draft"

bash scripts/recipes/gitops-check-activation-draft.sh \
  "$draft" \
  "$summary" \
  "$admission" \
  "${root}/fishystuff_deploy" \
  >"${root}/check-activation.stdout" \
  2>"${root}/check-activation.stderr"
pass "generic beta activation check"

bash scripts/recipes/gitops-review-activation-draft.sh \
  "$draft" \
  "$summary" \
  "$admission" \
  "${root}/fishystuff_deploy" \
  >"$review" \
  2>"${root}/review-activation.stderr"
grep -F "gitops_activation_review_ok=$draft" "$review" >/dev/null
grep -F "environment=beta" "$review" >/dev/null
grep -F "mode=local-apply" "$review" >/dev/null
grep -F "serve=true" "$review" >/dev/null
grep -F "transition_kind=activate" "$review" >/dev/null
grep -F "release_id=beta-release" "$review" >/dev/null
grep -F "dolt_commit=beta-active-dolt" "$review" >/dev/null
grep -F "api_upstream=$activation_api_upstream" "$review" >/dev/null
grep -F "remote_deploy_performed=false" "$review" >/dev/null
grep -F "infrastructure_mutation_performed=false" "$review" >/dev/null
pass "generic beta activation review"

if grep -F "production" "$admission" "$draft" "$review" >/dev/null; then
  printf '[gitops-beta-activation-draft-test] beta activation artifacts unexpectedly mention production\n' >&2
  exit 1
fi
pass "no production strings in beta activation artifacts"

printf '[gitops-beta-activation-draft-test] %s checks passed\n' "$pass_count"
