#!/usr/bin/env bash
set -euo pipefail

SCRIPT_PATH="$(readlink -f "${BASH_SOURCE[0]}")"
SCRIPT_DIR="$(cd "$(dirname "$SCRIPT_PATH")" && pwd)"
source "${SCRIPT_DIR}/lib/common.sh"

cd "$RECIPE_REPO_ROOT"

pass_count=0

pass() {
  printf '[gitops-beta-host-bootstrap-plan-test] pass: %s\n' "$1"
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
    printf '[gitops-beta-host-bootstrap-plan-test] expected failure: %s\n' "$name" >&2
    exit 1
  fi
  if ! grep -F "$expected" "$stderr" >/dev/null; then
    printf '[gitops-beta-host-bootstrap-plan-test] expected %s stderr to contain %q\n' "$name" "$expected" >&2
    cat "$stderr" >&2
    exit 1
  fi
  pass "$name"
}

root="$(mktemp -d)"
fake_bin="${root}/bin"
mkdir -p "$fake_bin"
cat >"${fake_bin}/hostname" <<'EOF'
#!/usr/bin/env bash
printf 'site-nbg1-beta\n'
EOF
chmod +x "${fake_bin}/hostname"
PATH="${fake_bin}:${PATH}"
plan_output="${root}/plan.out"

bash scripts/recipes/gitops-beta-host-bootstrap-plan.sh >"$plan_output"
grep -F "gitops_beta_host_bootstrap_plan_ok=true" "$plan_output" >/dev/null
grep -F "current_hostname=site-nbg1-beta" "$plan_output" >/dev/null
grep -F "resident_target=root@beta.fishystuff.fish" "$plan_output" >/dev/null
grep -F "resident_expected_hostname=site-nbg1-beta" "$plan_output" >/dev/null
grep -F "resident_expected_hostname_match=true" "$plan_output" >/dev/null
grep -F "site_base_url=https://beta.fishystuff.fish/" "$plan_output" >/dev/null
grep -F "api_base_url=https://api.beta.fishystuff.fish/" "$plan_output" >/dev/null
grep -F "cdn_base_url=https://cdn.beta.fishystuff.fish/" "$plan_output" >/dev/null
grep -F "api_runtime_env_path=/var/lib/fishystuff/gitops-beta/api/runtime.env" "$plan_output" >/dev/null
grep -F "api_release_env_path=/var/lib/fishystuff/gitops-beta/api/beta.env" "$plan_output" >/dev/null
grep -F "dolt_runtime_env_path=/var/lib/fishystuff/gitops-beta/dolt/beta.env" "$plan_output" >/dev/null
grep -F "service_unit_01=fishystuff-beta-dolt.service" "$plan_output" >/dev/null
grep -F "service_unit_02=fishystuff-beta-api.service" "$plan_output" >/dev/null
grep -F "service_unit_03=fishystuff-beta-edge.service" "$plan_output" >/dev/null
grep -F "handoff_to_service_start_packet=just gitops-beta-service-start-packet" "$plan_output" >/dev/null
grep -F "handoff_to_admission_packet=just gitops-beta-admission-packet" "$plan_output" >/dev/null
grep -F "remote_deploy_performed=false" "$plan_output" >/dev/null
grep -F "infrastructure_mutation_performed=false" "$plan_output" >/dev/null
grep -F "local_host_mutation_performed=false" "$plan_output" >/dev/null
pass "valid beta host bootstrap plan"

if grep -E 'fishystuff-api\.service|fishystuff-dolt\.service|/run/fishystuff/api/env|https://api\.fishystuff\.fish/|https://cdn\.fishystuff\.fish/' "$plan_output" >/dev/null; then
  printf '[gitops-beta-host-bootstrap-plan-test] bootstrap plan leaked production/shared material\n' >&2
  exit 1
fi
pass "no production service material in bootstrap plan"

expect_fail_contains \
  "reject production SecretSpec profile" \
  "must not run with production SecretSpec profile" \
  env \
    FISHYSTUFF_OPERATOR_SECRETSPEC_PROFILE=production-deploy \
    bash scripts/recipes/gitops-beta-host-bootstrap-plan.sh

expect_fail_contains \
  "reject production beta site URL" \
  "unsafe beta site URL host" \
  env \
    FISHYSTUFF_BETA_SITE_BASE_URL=https://fishystuff.fish/ \
    bash scripts/recipes/gitops-beta-host-bootstrap-plan.sh

expect_fail_contains \
  "reject reserved infra cluster label" \
  "unsafe beta infra cluster DNS label is reserved" \
  env \
    FISHYSTUFF_HETZNER_CLUSTER=production \
    bash scripts/recipes/gitops-beta-host-bootstrap-plan.sh

expect_fail_contains \
  "reject API runtime env path mismatch" \
  "API runtime env path must be /var/lib/fishystuff/gitops-beta/api/runtime.env" \
  bash scripts/recipes/gitops-beta-host-bootstrap-plan.sh \
    /var/lib/fishystuff/gitops/api/runtime.env \
    /var/lib/fishystuff/gitops-beta/api/beta.env \
    /var/lib/fishystuff/gitops-beta/dolt/beta.env

expect_fail_contains \
  "reject Dolt runtime env path mismatch" \
  "Dolt runtime env path must be /var/lib/fishystuff/gitops-beta/dolt/beta.env" \
  bash scripts/recipes/gitops-beta-host-bootstrap-plan.sh \
    /var/lib/fishystuff/gitops-beta/api/runtime.env \
    /var/lib/fishystuff/gitops-beta/api/beta.env \
    /var/lib/fishystuff/gitops/dolt/beta.env

printf '[gitops-beta-host-bootstrap-plan-test] %s checks passed\n' "$pass_count"
