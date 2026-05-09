#!/usr/bin/env bash
set -euo pipefail

SCRIPT_PATH="$(readlink -f "${BASH_SOURCE[0]}")"
SCRIPT_DIR="$(cd "$(dirname "$SCRIPT_PATH")" && pwd)"
source "${SCRIPT_DIR}/lib/common.sh"

cd "$RECIPE_REPO_ROOT"

pass_count=0

pass() {
  printf '[gitops-beta-host-selection-packet-test] pass: %s\n' "$1"
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
    printf '[gitops-beta-host-selection-packet-test] expected failure: %s\n' "$name" >&2
    exit 1
  fi
  if ! grep -F "$expected" "$stderr" >/dev/null; then
    printf '[gitops-beta-host-selection-packet-test] expected %s stderr to contain %q\n' "$name" "$expected" >&2
    cat "$stderr" >&2
    exit 1
  fi
  pass "$name"
}

root="$(mktemp -d)"

bash scripts/recipes/gitops-beta-host-selection-packet.sh >"${root}/pending.stdout"
grep -F "gitops_beta_host_selection_packet_ok=true" "${root}/pending.stdout" >/dev/null
grep -F "selection_status=pending_public_ipv4" "${root}/pending.stdout" >/dev/null
grep -F "host_name=site-nbg1-beta" "${root}/pending.stdout" >/dev/null
grep -F "host_expected_hostname=site-nbg1-beta" "${root}/pending.stdout" >/dev/null
grep -F "host_name_matches_expected_hostname=true" "${root}/pending.stdout" >/dev/null
grep -F "host_public_ipv4=<required>" "${root}/pending.stdout" >/dev/null
grep -F "resident_target=root@<new-beta-public-ip>" "${root}/pending.stdout" >/dev/null
grep -F "ssh_probe_performed=false" "${root}/pending.stdout" >/dev/null
grep -F "remote_deploy_performed=false" "${root}/pending.stdout" >/dev/null
grep -F "infrastructure_mutation_performed=false" "${root}/pending.stdout" >/dev/null
grep -F "local_host_mutation_performed=false" "${root}/pending.stdout" >/dev/null
pass "pending beta host selection packet"

bash scripts/recipes/gitops-beta-host-selection-packet.sh 203.0.113.10 >"${root}/ready.stdout"
grep -F "selection_status=ready" "${root}/ready.stdout" >/dev/null
grep -F "host_public_ipv4=203.0.113.10" "${root}/ready.stdout" >/dev/null
grep -F "resident_target=root@203.0.113.10" "${root}/ready.stdout" >/dev/null
grep -F "operator_env_01=FISHYSTUFF_BETA_RESIDENT_TARGET=root@203.0.113.10" "${root}/ready.stdout" >/dev/null
grep -F "read_only_next_command_01=FISHYSTUFF_OPERATOR_SECRETSPEC_PROFILE=beta-deploy secretspec run --profile beta-deploy -- just gitops-beta-remote-host-preflight target=root@203.0.113.10" "${root}/ready.stdout" >/dev/null
grep -F "guarded_followup_command_01=FISHYSTUFF_OPERATOR_SECRETSPEC_PROFILE=beta-deploy FISHYSTUFF_GITOPS_ENABLE_BETA_REMOTE_BOOTSTRAP=1" "${root}/ready.stdout" >/dev/null
grep -F "just gitops-beta-remote-host-bootstrap target=root@203.0.113.10" "${root}/ready.stdout" >/dev/null
pass "ready beta host selection packet"

bash scripts/recipes/gitops-beta-host-selection-packet.sh public_ipv4=203.0.113.20 host_name=site-nbg1-beta-v2 deployer >"${root}/custom.stdout"
grep -F "selection_status=ready" "${root}/custom.stdout" >/dev/null
grep -F "host_name=site-nbg1-beta-v2" "${root}/custom.stdout" >/dev/null
grep -F "host_name_matches_expected_hostname=false" "${root}/custom.stdout" >/dev/null
grep -F "resident_target=deployer@203.0.113.20" "${root}/custom.stdout" >/dev/null
pass "custom beta host selection packet"

expect_fail_contains \
  "reject production profile" \
  "must not run with production SecretSpec profile active" \
  env FISHYSTUFF_OPERATOR_SECRETSPEC_PROFILE=production-deploy \
    bash scripts/recipes/gitops-beta-host-selection-packet.sh 203.0.113.10

expect_fail_contains \
  "reject production-looking host" \
  "beta host_name must not look like production" \
  bash scripts/recipes/gitops-beta-host-selection-packet.sh 203.0.113.10 site-nbg1-prod

expect_fail_contains \
  "reject invalid IPv4" \
  "public_ipv4 must be an IPv4 address" \
  bash scripts/recipes/gitops-beta-host-selection-packet.sh beta.fishystuff.fish

expect_fail_contains \
  "reject ssh_user with host" \
  "ssh_user must be a bare SSH username" \
  bash scripts/recipes/gitops-beta-host-selection-packet.sh 203.0.113.10 site-nbg1-beta root@203.0.113.10

printf '[gitops-beta-host-selection-packet-test] %s checks passed\n' "$pass_count"
