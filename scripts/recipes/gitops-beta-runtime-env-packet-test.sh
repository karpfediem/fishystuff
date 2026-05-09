#!/usr/bin/env bash
set -euo pipefail

SCRIPT_PATH="$(readlink -f "${BASH_SOURCE[0]}")"
SCRIPT_DIR="$(cd "$(dirname "$SCRIPT_PATH")" && pwd)"
source "${SCRIPT_DIR}/lib/common.sh"

cd "$RECIPE_REPO_ROOT"

pass_count=0

pass() {
  printf '[gitops-beta-runtime-env-packet-test] pass: %s\n' "$1"
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
    printf '[gitops-beta-runtime-env-packet-test] expected failure: %s\n' "$name" >&2
    exit 1
  fi
  if ! grep -F "$expected" "$stderr" >/dev/null; then
    printf '[gitops-beta-runtime-env-packet-test] expected %s stderr to contain %q\n' "$name" "$expected" >&2
    cat "$stderr" >&2
    exit 1
  fi
  pass "$name"
}

root="$(mktemp -d)"
api_env="${root}/api/runtime.env"
dolt_env="${root}/dolt/beta.env"

bash scripts/recipes/gitops-beta-runtime-env-packet.sh \
  "$api_env" \
  "$dolt_env" \
  auto \
  auto \
  "${root}/summary.json" >"${root}/missing.stdout"
grep -F "runtime_env_packet_status=pending_runtime_env" "${root}/missing.stdout" >/dev/null
grep -F "runtime_env_packet_api_status=missing" "${root}/missing.stdout" >/dev/null
grep -F "runtime_env_packet_dolt_status=missing" "${root}/missing.stdout" >/dev/null
grep -F "runtime_env_packet_next_command_01=FISHYSTUFF_GITOPS_ENABLE_BETA_DOLT_RUNTIME_ENV_WRITE=1 just gitops-beta-write-runtime-env service=dolt output=${dolt_env}" "${root}/missing.stdout" >/dev/null
grep -F "runtime_env_packet_next_command_02=FISHYSTUFF_GITOPS_ENABLE_BETA_API_RUNTIME_ENV_WRITE=1 just gitops-beta-write-runtime-env-secretspec service=api output=${api_env} profile=beta-runtime" "${root}/missing.stdout" >/dev/null
grep -F "runtime_env_packet_after_success_command=just gitops-beta-service-start-plan api_bundle=auto dolt_bundle=auto api_env_file=${api_env} dolt_env_file=${dolt_env} summary_file=${root}/summary.json" "${root}/missing.stdout" >/dev/null
grep -F "remote_deploy_performed=false" "${root}/missing.stdout" >/dev/null
grep -F "infrastructure_mutation_performed=false" "${root}/missing.stdout" >/dev/null
grep -F "local_host_mutation_performed=false" "${root}/missing.stdout" >/dev/null
pass "missing runtime env packet"

env \
  FISHYSTUFF_GITOPS_ENABLE_BETA_API_RUNTIME_ENV_WRITE=1 \
  FISHYSTUFF_GITOPS_BETA_API_DATABASE_URL="mysql://fishy:secret@127.0.0.1:3316/fishystuff" \
  bash scripts/recipes/gitops-beta-write-runtime-env.sh api "$api_env" >/dev/null
env \
  FISHYSTUFF_GITOPS_ENABLE_BETA_DOLT_RUNTIME_ENV_WRITE=1 \
  bash scripts/recipes/gitops-beta-write-runtime-env.sh dolt "$dolt_env" >/dev/null

bash scripts/recipes/gitops-beta-runtime-env-packet.sh \
  "$api_env" \
  "$dolt_env" \
  /tmp/api-bundle \
  /tmp/dolt-bundle \
  "${root}/summary.json" >"${root}/ready.stdout"
grep -F "runtime_env_packet_status=ready" "${root}/ready.stdout" >/dev/null
grep -F "runtime_env_packet_api_status=ready" "${root}/ready.stdout" >/dev/null
grep -F "runtime_env_packet_dolt_status=ready" "${root}/ready.stdout" >/dev/null
grep -F "runtime_env_packet_api_database=loopback-dolt-beta" "${root}/ready.stdout" >/dev/null
grep -F "runtime_env_packet_next_command_01=just gitops-beta-service-start-plan api_bundle=/tmp/api-bundle dolt_bundle=/tmp/dolt-bundle api_env_file=${api_env} dolt_env_file=${dolt_env} summary_file=${root}/summary.json" "${root}/ready.stdout" >/dev/null
pass "ready runtime env packet"

bad_api_env="${root}/bad-api.env"
cat >"$bad_api_env" <<'EOF'
FISHYSTUFF_DATABASE_URL='mysql://fishy:secret@127.0.0.1:3316/fishystuff'
FISHYSTUFF_CORS_ALLOWED_ORIGINS='https://fishystuff.fish'
FISHYSTUFF_PUBLIC_SITE_BASE_URL='https://beta.fishystuff.fish'
FISHYSTUFF_PUBLIC_CDN_BASE_URL='https://cdn.beta.fishystuff.fish'
FISHYSTUFF_RUNTIME_CDN_BASE_URL='https://cdn.beta.fishystuff.fish'
EOF
expect_fail_contains \
  "reject invalid existing API runtime env" \
  "production or shared deployment material" \
  bash scripts/recipes/gitops-beta-runtime-env-packet.sh \
    "$bad_api_env" \
    "$dolt_env" \
    auto \
    auto \
    "${root}/summary.json"

printf '[gitops-beta-runtime-env-packet-test] %s checks passed\n' "$pass_count"
