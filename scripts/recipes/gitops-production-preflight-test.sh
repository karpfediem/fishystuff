#!/usr/bin/env bash
set -euo pipefail

SCRIPT_PATH="$(readlink -f "${BASH_SOURCE[0]}")"
SCRIPT_DIR="$(cd "$(dirname "$SCRIPT_PATH")" && pwd)"
source "${SCRIPT_DIR}/lib/common.sh"

cd "$RECIPE_REPO_ROOT"

export FISHYSTUFF_GITOPS_HOST_HANDOFF_PLAN_TEST_SOURCE_ONLY=1
# Reuse the host handoff fixture builders so this wrapper test stays tied to the
# same local-only activation, admission, and edge-bundle contract.
# shellcheck source=scripts/recipes/gitops-production-host-handoff-plan-test.sh
source scripts/recipes/gitops-production-host-handoff-plan-test.sh
unset FISHYSTUFF_GITOPS_HOST_HANDOFF_PLAN_TEST_SOURCE_ONLY

pass_count=0

pass() {
  printf '[gitops-production-preflight-test] pass: %s\n' "$1"
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
    printf '[gitops-production-preflight-test] expected failure: %s\n' "$name" >&2
    exit 1
  fi
  if ! grep -F "$expected" "$stderr" >/dev/null; then
    printf '[gitops-production-preflight-test] expected %s stderr to contain %q\n' "$name" "$expected" >&2
    cat "$stderr" >&2
    exit 1
  fi
  pass "$name"
}

write_retained_output_from_summary() {
  local summary="$1"
  local output="$2"

  jq '
    [.retained_releases[]
      | {
          release_id,
          generation,
          git_rev,
          dolt_commit,
          api_closure: .closures.api,
          site_closure: .closures.site,
          cdn_runtime_closure: .closures.cdn_runtime,
          dolt_service_closure: .closures.dolt_service,
          dolt_materialization: .dolt.materialization,
          dolt_cache_dir: .dolt.cache_dir,
          dolt_release_ref: .dolt.release_ref
        }
    ]' "$summary" >"$output"
}

make_fake_preflight_deploy() {
  local path="$1"
  cat >"$path" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
case "$*" in
  gitops\ check-desired-serving\ --state\ *\ --environment\ production)
    printf 'fake_desired_serving_ok\n'
    ;;
  gitops\ retained-releases-json\ --rollback-set\ *)
    cat "${FISHYSTUFF_FAKE_RETAINED_RELEASES_JSON:?}"
    ;;
  *)
    echo "unexpected fake fishystuff_deploy args: $*" >&2
    exit 2
    ;;
esac
EOF
  chmod +x "$path"
}

root="$(mktemp -d)"
make_fixture "$root"
make_edge_bundle "${root}/edge-bundle"
make_fake_deploy "${root}/fishystuff_deploy"

draft="$(cat "${root}/draft.path")"
summary="$(cat "${root}/summary.path")"
admission="$(cat "${root}/admission.path")"

bash scripts/recipes/gitops-production-preflight.sh \
  "$draft" \
  "$summary" \
  "$admission" \
  "${root}/edge-bundle" \
  "${root}/fishystuff_deploy" \
  false >"${root}/preflight.stdout"

grep -F "gitops_production_preflight_ok=${draft}" "${root}/preflight.stdout" >/dev/null
grep -F "handoff_summary=${summary}" "${root}/preflight.stdout" >/dev/null
grep -F "admission_evidence=${admission}" "${root}/preflight.stdout" >/dev/null
grep -F "edge_bundle=${root}/edge-bundle" "${root}/preflight.stdout" >/dev/null
grep -F "helper_regressions_run=false" "${root}/preflight.stdout" >/dev/null
grep -F "host_handoff_plan_begin" "${root}/preflight.stdout" >/dev/null
grep -F "planned_host_step_05=systemctl restart fishystuff-edge.service" "${root}/preflight.stdout" >/dev/null
grep -F "remote_deploy_performed=false" "${root}/preflight.stdout" >/dev/null
pass "valid production preflight"

served_root="${root}/served"
served_retained="${root}/served-retained.json"
mkdir -p "${served_root}/rollback-set"
printf '{}\n' >"${served_root}/rollback-set/production.json"
write_retained_output_from_summary "$summary" "$served_retained"
make_fake_preflight_deploy "${root}/fishystuff_deploy_with_retained"

FISHYSTUFF_FAKE_RETAINED_RELEASES_JSON="$served_retained" \
  bash scripts/recipes/gitops-production-preflight.sh \
    "$draft" \
    "$summary" \
    "$admission" \
    "${root}/edge-bundle" \
    "${root}/fishystuff_deploy_with_retained" \
    false \
    "$served_root" >"${root}/preflight-served.stdout"

grep -F "gitops_production_preflight_ok=${draft}" "${root}/preflight-served.stdout" >/dev/null
grep -F "served_rollback_set_checked=true" "${root}/preflight-served.stdout" >/dev/null
grep -F "served_state_dir=${served_root}" "${root}/preflight-served.stdout" >/dev/null
grep -F "served_rollback_set=${served_root}/rollback-set/production.json" "${root}/preflight-served.stdout" >/dev/null
pass "valid production preflight checks served rollback-set"

wrong_retained="${root}/served-retained-wrong.json"
jq '.[0].release_id = "wrong-release"' "$served_retained" >"$wrong_retained"
expect_fail_contains \
  "reject served rollback-set mismatch" \
  "served rollback-set retained releases do not match the handoff summary" \
  env FISHYSTUFF_FAKE_RETAINED_RELEASES_JSON="$wrong_retained" \
    bash scripts/recipes/gitops-production-preflight.sh \
    "$draft" \
    "$summary" \
    "$admission" \
    "${root}/edge-bundle" \
    "${root}/fishystuff_deploy_with_retained" \
    false \
    "$served_root"

expect_fail_contains \
  "missing admission evidence" \
  "gitops-production-preflight requires admission_file or FISHYSTUFF_GITOPS_ADMISSION_EVIDENCE_FILE" \
  bash scripts/recipes/gitops-production-preflight.sh \
    "$draft" \
    "$summary" \
    "" \
    "${root}/edge-bundle" \
    "${root}/fishystuff_deploy" \
    false

expect_fail_contains \
  "invalid helper test flag" \
  "run_helper_tests must be true or false" \
  bash scripts/recipes/gitops-production-preflight.sh \
    "$draft" \
    "$summary" \
    "$admission" \
    "${root}/edge-bundle" \
    "${root}/fishystuff_deploy" \
    maybe

printf '[gitops-production-preflight-test] %s checks passed\n' "$pass_count"
