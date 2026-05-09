#!/usr/bin/env bash
set -euo pipefail

SCRIPT_PATH="$(readlink -f "${BASH_SOURCE[0]}")"
SCRIPT_DIR="$(cd "$(dirname "$SCRIPT_PATH")" && pwd)"
source "${SCRIPT_DIR}/lib/common.sh"

cd "$RECIPE_REPO_ROOT"

require_command() {
  local command_name="$1"
  if ! command -v "$command_name" >/dev/null 2>&1; then
    echo "missing required command: ${command_name}" >&2
    exit 127
  fi
}

pass_count=0

pass() {
  printf '[gitops-beta-current-handoff-plan-test] pass: %s\n' "$1"
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
    printf '[gitops-beta-current-handoff-plan-test] expected failure: %s\n' "$name" >&2
    exit 1
  fi
  if ! grep -F "$expected" "$stderr" >/dev/null; then
    printf '[gitops-beta-current-handoff-plan-test] expected %s stderr to contain %q\n' "$name" "$expected" >&2
    cat "$stderr" >&2
    exit 1
  fi
  pass "$name"
}

write_fake_mgmt() {
  local path="$1"

  cat >"$path" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
exit 0
EOF
  chmod +x "$path"
}

root="$(mktemp -d)"
trap 'rm -rf "$root"' EXIT

env \
  -u FISHYSTUFF_OPERATOR_ROOT \
  -u FISHYSTUFF_GITOPS_CDN_RUNTIME_CLOSURE \
  FISHYSTUFF_GITOPS_GIT_REV=beta-plan-git \
  FISHYSTUFF_GITOPS_DOLT_COMMIT=beta-plan-dolt \
  FISHYSTUFF_GITOPS_DOLT_REMOTE_URL="file://${root}/dolt-origin" \
  bash scripts/recipes/gitops-beta-current-handoff-plan.sh \
    "${root}/beta-current.desired.json" \
    beta \
    auto \
    auto \
    "${root}/beta-current.handoff-summary.json" \
  >"${root}/blocked.stdout"

grep -F "gitops_beta_current_handoff_plan_ok=true" "${root}/blocked.stdout" >/dev/null
grep -F "cdn_runtime_closure_status=blocked_missing_operator_root" "${root}/blocked.stdout" >/dev/null
grep -F "handoff_plan_status=blocked" "${root}/blocked.stdout" >/dev/null
grep -F "handoff_can_run=false" "${root}/blocked.stdout" >/dev/null
grep -F "remote_deploy_performed=false" "${root}/blocked.stdout" >/dev/null
grep -F "infrastructure_mutation_performed=false" "${root}/blocked.stdout" >/dev/null
grep -F "local_host_mutation_performed=false" "${root}/blocked.stdout" >/dev/null
pass "report missing CDN operator input"

api_closure="$(readlink -f /run/current-system)"
site_closure="$(readlink -f /run/current-system/sw)"
cdn_runtime_closure="$(readlink -f /run/current-system/sw/bin/bash)"
dolt_service_closure="$api_closure"
fake_mgmt="${root}/mgmt"
write_fake_mgmt "$fake_mgmt"

env \
  -u FISHYSTUFF_OPERATOR_ROOT \
  FISHYSTUFF_GITOPS_GIT_REV=beta-plan-git \
  FISHYSTUFF_GITOPS_DOLT_COMMIT=beta-plan-dolt \
  FISHYSTUFF_GITOPS_DOLT_REMOTE_URL="file://${root}/dolt-origin" \
  FISHYSTUFF_GITOPS_API_CLOSURE="$api_closure" \
  FISHYSTUFF_GITOPS_SITE_CLOSURE="$site_closure" \
  FISHYSTUFF_GITOPS_CDN_RUNTIME_CLOSURE="$cdn_runtime_closure" \
  FISHYSTUFF_GITOPS_DOLT_SERVICE_CLOSURE="$dolt_service_closure" \
  bash scripts/recipes/gitops-beta-current-handoff-plan.sh \
    "${root}/ready.desired.json" \
    beta \
    "$fake_mgmt" \
    auto \
    "${root}/ready.handoff-summary.json" \
  >"${root}/ready.stdout"

grep -F "handoff_plan_status=ready" "${root}/ready.stdout" >/dev/null
grep -F "handoff_can_run=true" "${root}/ready.stdout" >/dev/null
grep -F "closure_build_required=false" "${root}/ready.stdout" >/dev/null
grep -F "mgmt_bin_status=provided_executable" "${root}/ready.stdout" >/dev/null
grep -F "api_closure_status=provided_existing" "${root}/ready.stdout" >/dev/null
grep -F "site_closure_status=provided_existing" "${root}/ready.stdout" >/dev/null
grep -F "cdn_runtime_closure_status=provided_existing" "${root}/ready.stdout" >/dev/null
grep -F "dolt_service_closure_status=provided_existing" "${root}/ready.stdout" >/dev/null
pass "accept exact local closure inputs"

missing_closure="/nix/store/00000000000000000000000000000000-missing-fishystuff"
env \
  -u FISHYSTUFF_OPERATOR_ROOT \
  FISHYSTUFF_GITOPS_GIT_REV=beta-plan-git \
  FISHYSTUFF_GITOPS_DOLT_COMMIT=beta-plan-dolt \
  FISHYSTUFF_GITOPS_DOLT_REMOTE_URL="file://${root}/dolt-origin" \
  FISHYSTUFF_GITOPS_API_CLOSURE="$missing_closure" \
  FISHYSTUFF_GITOPS_SITE_CLOSURE="$site_closure" \
  FISHYSTUFF_GITOPS_CDN_RUNTIME_CLOSURE="$cdn_runtime_closure" \
  FISHYSTUFF_GITOPS_DOLT_SERVICE_CLOSURE="$dolt_service_closure" \
  bash scripts/recipes/gitops-beta-current-handoff-plan.sh \
    "${root}/missing.desired.json" \
    beta \
    "$fake_mgmt" \
    auto \
    "${root}/missing.handoff-summary.json" \
  >"${root}/missing.stdout"

grep -F "api_closure_status=provided_missing" "${root}/missing.stdout" >/dev/null
grep -F "handoff_plan_status=blocked" "${root}/missing.stdout" >/dev/null
grep -F "handoff_can_run=false" "${root}/missing.stdout" >/dev/null
pass "report missing exact closure input"

expect_fail_contains \
  "reject production environment" \
  "only describes beta handoff input readiness" \
  env \
    FISHYSTUFF_GITOPS_ENVIRONMENT=production \
    FISHYSTUFF_GITOPS_DOLT_COMMIT=beta-plan-dolt \
    FISHYSTUFF_GITOPS_DOLT_REMOTE_URL="file://${root}/dolt-origin" \
    bash scripts/recipes/gitops-beta-current-handoff-plan.sh

printf '[gitops-beta-current-handoff-plan-test] %s checks passed\n' "$pass_count"
