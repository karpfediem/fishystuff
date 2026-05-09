#!/usr/bin/env bash
set -euo pipefail

SCRIPT_PATH="$(readlink -f "${BASH_SOURCE[0]}")"
SCRIPT_DIR="$(cd "$(dirname "$SCRIPT_PATH")" && pwd)"
source "${SCRIPT_DIR}/lib/common.sh"

cd "$RECIPE_REPO_ROOT"

pass_count=0

pass() {
  printf '[gitops-beta-runtime-env-host-preflight-test] pass: %s\n' "$1"
  pass_count="$((pass_count + 1))"
}

expect_fail_contains() {
  local name="$1"
  local expected="$2"
  shift 2
  local test_root=""
  local stderr=""

  test_root="$(mktemp -d)"
  stderr="${test_root}/stderr"
  if "$@" >"${test_root}/stdout" 2>"$stderr"; then
    printf '[gitops-beta-runtime-env-host-preflight-test] expected failure: %s\n' "$name" >&2
    exit 1
  fi
  if ! grep -F "$expected" "$stderr" >/dev/null; then
    printf '[gitops-beta-runtime-env-host-preflight-test] expected %s stderr to contain %q\n' "$name" "$expected" >&2
    cat "$stderr" >&2
    exit 1
  fi
  pass "$name"
}

root="$(mktemp -d)"
api_env="${root}/api/runtime.env"
dolt_env="${root}/dolt/beta.env"

bash scripts/recipes/gitops-beta-runtime-env-host-preflight.sh \
  "$api_env" \
  "$dolt_env" >"${root}/blocked.stdout"
grep -F "gitops_beta_runtime_env_host_preflight_ok=true" "${root}/blocked.stdout" >/dev/null
grep -F "runtime_env_host_preflight_status=blocked" "${root}/blocked.stdout" >/dev/null
grep -F "runtime_env_host_preflight_api_parent_exists=false" "${root}/blocked.stdout" >/dev/null
grep -F "runtime_env_host_preflight_dolt_parent_exists=false" "${root}/blocked.stdout" >/dev/null
grep -F "runtime_env_host_preflight_path_ready=false" "${root}/blocked.stdout" >/dev/null
grep -F "runtime_env_host_preflight_ready=false" "${root}/blocked.stdout" >/dev/null
grep -F "remote_deploy_performed=false" "${root}/blocked.stdout" >/dev/null
grep -F "infrastructure_mutation_performed=false" "${root}/blocked.stdout" >/dev/null
grep -F "local_host_mutation_performed=false" "${root}/blocked.stdout" >/dev/null
pass "blocked host preflight"

mkdir -p "${root}/api" "${root}/dolt"
bash scripts/recipes/gitops-beta-runtime-env-host-preflight.sh \
  "$api_env" \
  "$dolt_env" >"${root}/ready.stdout"
grep -F "runtime_env_host_preflight_api_parent_exists=true" "${root}/ready.stdout" >/dev/null
grep -F "runtime_env_host_preflight_api_parent_writable=true" "${root}/ready.stdout" >/dev/null
grep -F "runtime_env_host_preflight_dolt_parent_exists=true" "${root}/ready.stdout" >/dev/null
grep -F "runtime_env_host_preflight_dolt_parent_writable=true" "${root}/ready.stdout" >/dev/null
grep -F "runtime_env_host_preflight_path_ready=true" "${root}/ready.stdout" >/dev/null
pass "writable path host preflight"

expect_fail_contains \
  "reject production runtime env path" \
  "refusing beta api runtime env preflight outside the beta runtime path or /tmp" \
  bash scripts/recipes/gitops-beta-runtime-env-host-preflight.sh \
    "/var/lib/fishystuff/gitops/api/runtime.env" \
    "$dolt_env"

expect_fail_contains \
  "reject production operator profile" \
  "unsafe beta secret scope" \
  env FISHYSTUFF_OPERATOR_SECRETSPEC_PROFILE=production-deploy \
    bash scripts/recipes/gitops-beta-runtime-env-host-preflight.sh \
      "$api_env" \
      "$dolt_env"

printf '[gitops-beta-runtime-env-host-preflight-test] %s checks passed\n' "$pass_count"
