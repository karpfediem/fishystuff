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
  printf '[gitops-beta-admission-packet-test] pass: %s\n' "$1"
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
    printf '[gitops-beta-admission-packet-test] expected failure: %s\n' "$name" >&2
    exit 1
  fi
  if ! grep -F "$expected" "$stderr" >/dev/null; then
    printf '[gitops-beta-admission-packet-test] expected %s stderr to contain %q\n' "$name" "$expected" >&2
    cat "$stderr" >&2
    exit 1
  fi
  pass "$name"
}

root="$(mktemp -d)"
make_fixture "$root"
summary="$(cat "${root}/summary.path")"
api_meta="$(cat "${root}/api-meta.path")"
db_probe="$(cat "${root}/db-probe.path")"
site_cdn_probe="$(cat "${root}/site-cdn-probe.path")"
admission="${root}/beta-admission.evidence.json"
draft="${root}/beta-activation.draft.desired.json"
api_upstream="http://127.0.0.1:18192"

bash scripts/recipes/gitops-beta-admission-packet.sh \
  "$admission" \
  "$summary" \
  "$api_upstream" \
  "${root}/observations" \
  "$draft" >"${root}/missing.stdout"

grep -F "gitops_beta_admission_packet_ok=true" "${root}/missing.stdout" >/dev/null
grep -F "admission_packet_status=missing" "${root}/missing.stdout" >/dev/null
grep -F "admission_packet_summary_file=${summary}" "${root}/missing.stdout" >/dev/null
grep -F "admission_packet_admission_file=${admission}" "${root}/missing.stdout" >/dev/null
grep -F "admission_packet_api_upstream=${api_upstream}" "${root}/missing.stdout" >/dev/null
grep -F "admission_packet_next_command_01=just gitops-beta-observe-admission output=${admission} summary_file=${summary} api_upstream=${api_upstream} observation_dir=${root}/observations" "${root}/missing.stdout" >/dev/null
grep -F "admission_packet_after_success_command=just gitops-beta-activation-draft output=${draft} summary_file=${summary} admission_file=${admission}" "${root}/missing.stdout" >/dev/null
grep -F "remote_deploy_performed=false" "${root}/missing.stdout" >/dev/null
grep -F "infrastructure_mutation_performed=false" "${root}/missing.stdout" >/dev/null
grep -F "local_host_mutation_performed=false" "${root}/missing.stdout" >/dev/null
pass "missing admission packet"

bash scripts/recipes/gitops-beta-write-activation-admission-evidence.sh \
  "$admission" \
  "$summary" \
  "$api_upstream" \
  "$api_meta" \
  "$db_probe" \
  "$site_cdn_probe" >/dev/null 2>"${root}/write-admission.stderr"

bash scripts/recipes/gitops-beta-admission-packet.sh \
  "$admission" \
  "$summary" \
  "$api_upstream" \
  "${root}/observations" \
  "$draft" >"${root}/ready.stdout"

grep -F "admission_packet_status=ready" "${root}/ready.stdout" >/dev/null
grep -F "admission_packet_release_id=beta-release" "${root}/ready.stdout" >/dev/null
grep -F "admission_packet_db_probe=beta-db-fixture" "${root}/ready.stdout" >/dev/null
grep -F "admission_packet_site_cdn_probe=beta-site-cdn-fixture" "${root}/ready.stdout" >/dev/null
grep -F "admission_packet_next_command_01=just gitops-beta-activation-draft output=${draft} summary_file=${summary} admission_file=${admission}" "${root}/ready.stdout" >/dev/null
pass "ready admission packet"

bad_admission="${root}/bad-admission.evidence.json"
jq '.api_upstream = "http://127.0.0.1:9999"' "$admission" >"$bad_admission"
expect_fail_contains \
  "reject stale admission evidence" \
  "beta admission evidence does not match" \
  bash scripts/recipes/gitops-beta-admission-packet.sh \
    "$bad_admission" \
    "$summary" \
    "$api_upstream" \
    "${root}/observations" \
    "$draft"

expect_fail_contains \
  "reject public API upstream" \
  "api_upstream must be a loopback HTTP URL" \
  bash scripts/recipes/gitops-beta-admission-packet.sh \
    "$admission" \
    "$summary" \
    "https://api.beta.fishystuff.fish" \
    "${root}/observations" \
    "$draft"

printf '[gitops-beta-admission-packet-test] %s checks passed\n' "$pass_count"
