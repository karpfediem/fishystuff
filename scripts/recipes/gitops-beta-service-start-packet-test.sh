#!/usr/bin/env bash
set -euo pipefail

SCRIPT_PATH="$(readlink -f "${BASH_SOURCE[0]}")"
SCRIPT_DIR="$(cd "$(dirname "$SCRIPT_PATH")" && pwd)"
source "${SCRIPT_DIR}/lib/common.sh"

cd "$RECIPE_REPO_ROOT"

FISHYSTUFF_GITOPS_BETA_SERVICE_START_PLAN_TEST_SOURCE_ONLY=1
source scripts/recipes/gitops-beta-service-start-plan-test.sh
unset FISHYSTUFF_GITOPS_BETA_SERVICE_START_PLAN_TEST_SOURCE_ONLY

pass_count=0

pass() {
  printf '[gitops-beta-service-start-packet-test] pass: %s\n' "$1"
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
    printf '[gitops-beta-service-start-packet-test] expected failure: %s\n' "$name" >&2
    exit 1
  fi
  if ! grep -F "$expected" "$stderr" >/dev/null; then
    printf '[gitops-beta-service-start-packet-test] expected %s stderr to contain %q\n' "$name" "$expected" >&2
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
printf 'operator-dev\n'
EOF
chmod +x "${fake_bin}/hostname"
PATH="${fake_bin}:${PATH}"

api_bundle="${root}/api-bundle"
dolt_bundle="${root}/dolt-bundle"
api_env="${root}/api/runtime.env"
dolt_env="${root}/dolt/beta.env"
summary="${root}/beta-current.handoff-summary.json"
make_beta_service_bundle "$api_bundle" api
make_beta_service_bundle "$dolt_bundle" dolt
read -r api_unit_sha256 _ < <(sha256sum "${api_bundle}/artifacts/systemd/unit")
read -r dolt_unit_sha256 _ < <(sha256sum "${dolt_bundle}/artifacts/systemd/unit")
jq -n \
  --arg api_bundle "$api_bundle" \
  --arg dolt_bundle "$dolt_bundle" \
  '{
    environment: {
      name: "beta"
    },
    active_release: {
      closures: {
        api: $api_bundle,
        dolt_service: $dolt_bundle
      }
    }
  }' >"$summary"

env \
  FISHYSTUFF_GITOPS_ENABLE_BETA_API_RUNTIME_ENV_WRITE=1 \
  FISHYSTUFF_GITOPS_BETA_API_DATABASE_URL="mysql://fishy:secret@127.0.0.1:3316/fishystuff" \
  bash scripts/recipes/gitops-beta-write-runtime-env.sh api "$api_env" >/dev/null
env \
  FISHYSTUFF_GITOPS_ENABLE_BETA_DOLT_RUNTIME_ENV_WRITE=1 \
  bash scripts/recipes/gitops-beta-write-runtime-env.sh dolt "$dolt_env" >/dev/null

FISHYSTUFF_GITOPS_BETA_SERVICE_START_PLAN_ALLOW_ENV_FILE_FIXTURE=1 \
  bash scripts/recipes/gitops-beta-service-start-packet.sh \
    auto \
    auto \
    "$api_env" \
    "$dolt_env" \
    "$summary" >"${root}/packet.stdout"

grep -F "gitops_beta_service_start_packet_ok=true" "${root}/packet.stdout" >/dev/null
grep -F "service_start_packet_status=ready" "${root}/packet.stdout" >/dev/null
grep -F "service_start_packet_current_hostname=operator-dev" "${root}/packet.stdout" >/dev/null
grep -F "service_start_packet_expected_hostname=site-nbg1-beta" "${root}/packet.stdout" >/dev/null
grep -F "service_start_packet_expected_hostname_match=false" "${root}/packet.stdout" >/dev/null
grep -F "service_start_packet_resident_target=root@beta.fishystuff.fish" "${root}/packet.stdout" >/dev/null
grep -F "service_start_packet_bundle_source=handoff_summary" "${root}/packet.stdout" >/dev/null
grep -F "service_start_packet_handoff_summary=${summary}" "${root}/packet.stdout" >/dev/null
grep -F "service_start_packet_api_bundle=${api_bundle}" "${root}/packet.stdout" >/dev/null
grep -F "service_start_packet_dolt_bundle=${dolt_bundle}" "${root}/packet.stdout" >/dev/null
grep -F "service_start_packet_api_unit=fishystuff-beta-api.service" "${root}/packet.stdout" >/dev/null
grep -F "service_start_packet_dolt_unit=fishystuff-beta-dolt.service" "${root}/packet.stdout" >/dev/null
grep -F "service_start_packet_api_unit_sha256=${api_unit_sha256}" "${root}/packet.stdout" >/dev/null
grep -F "service_start_packet_dolt_unit_sha256=${dolt_unit_sha256}" "${root}/packet.stdout" >/dev/null
grep -F "service_start_packet_order_01=dolt" "${root}/packet.stdout" >/dev/null
grep -F "service_start_packet_order_02=api" "${root}/packet.stdout" >/dev/null
grep -F "service_start_packet_next_command_01=FISHYSTUFF_GITOPS_ENABLE_BETA_SERVICE_START=1" "${root}/packet.stdout" >/dev/null
grep -F "FISHYSTUFF_GITOPS_BETA_DOLT_UNIT_SHA256=${dolt_unit_sha256}" "${root}/packet.stdout" >/dev/null
grep -F "FISHYSTUFF_GITOPS_BETA_API_UNIT_SHA256=${api_unit_sha256}" "${root}/packet.stdout" >/dev/null
grep -F "just gitops-beta-start-services api_bundle=${api_bundle} dolt_bundle=${dolt_bundle} api_env_file=${api_env} dolt_env_file=${dolt_env} summary_file=${summary}" "${root}/packet.stdout" >/dev/null
grep -F "service_start_packet_after_success_command=just gitops-beta-admission-packet summary_file=${summary} api_upstream=http://127.0.0.1:18192" "${root}/packet.stdout" >/dev/null
grep -F "remote_deploy_performed=false" "${root}/packet.stdout" >/dev/null
grep -F "infrastructure_mutation_performed=false" "${root}/packet.stdout" >/dev/null
grep -F "local_host_mutation_performed=false" "${root}/packet.stdout" >/dev/null
pass "ready service start packet"

FISHYSTUFF_GITOPS_BETA_SERVICE_START_PLAN_ALLOW_ENV_FILE_FIXTURE=1 \
  bash scripts/recipes/gitops-beta-service-start-packet.sh \
    "$api_bundle" \
    "$dolt_bundle" \
    "${root}/missing-api.env" \
    "$dolt_env" \
    "$summary" >"${root}/missing-runtime.stdout"
grep -F "gitops_beta_service_start_packet_ok=true" "${root}/missing-runtime.stdout" >/dev/null
grep -F "service_start_packet_status=pending_runtime_env" "${root}/missing-runtime.stdout" >/dev/null
grep -F "service_start_packet_current_hostname=operator-dev" "${root}/missing-runtime.stdout" >/dev/null
grep -F "service_start_packet_expected_hostname_match=false" "${root}/missing-runtime.stdout" >/dev/null
grep -F "service_start_packet_api_status=missing" "${root}/missing-runtime.stdout" >/dev/null
grep -F "service_start_packet_dolt_status=ready" "${root}/missing-runtime.stdout" >/dev/null
grep -F "service_start_packet_before_write_command=just gitops-beta-runtime-env-host-preflight api_env_file=${root}/missing-api.env dolt_env_file=${dolt_env}" "${root}/missing-runtime.stdout" >/dev/null
grep -F "service_start_packet_host_preflight_status=blocked" "${root}/missing-runtime.stdout" >/dev/null
grep -F "service_start_packet_host_preflight_next_required_action=run_on_expected_beta_host" "${root}/missing-runtime.stdout" >/dev/null
grep -F "service_start_packet_host_preflight_current_hostname=operator-dev" "${root}/missing-runtime.stdout" >/dev/null
grep -F "service_start_packet_host_preflight_expected_hostname=site-nbg1-beta" "${root}/missing-runtime.stdout" >/dev/null
grep -F "service_start_packet_host_preflight_expected_hostname_match=false" "${root}/missing-runtime.stdout" >/dev/null
grep -F "service_start_packet_host_preflight_resident_target=root@beta.fishystuff.fish" "${root}/missing-runtime.stdout" >/dev/null
grep -F "service_start_packet_host_preflight_path_ready=true" "${root}/missing-runtime.stdout" >/dev/null
grep -F "service_start_packet_host_preflight_ready=false" "${root}/missing-runtime.stdout" >/dev/null
grep -F "service_start_packet_host_preflight_next_command_01=just gitops-beta-runtime-env-host-preflight api_env_file=${root}/missing-api.env dolt_env_file=${dolt_env}" "${root}/missing-runtime.stdout" >/dev/null
grep -F "service_start_packet_host_preflight_next_note_01=run this preflight on site-nbg1-beta before writing beta runtime env" "${root}/missing-runtime.stdout" >/dev/null
grep -F "service_start_packet_next_command_01=FISHYSTUFF_GITOPS_ENABLE_BETA_API_RUNTIME_ENV_WRITE=1 just gitops-beta-write-runtime-env-secretspec service=api output=${root}/missing-api.env profile=beta-runtime" "${root}/missing-runtime.stdout" >/dev/null
grep -F "remote_deploy_performed=false" "${root}/missing-runtime.stdout" >/dev/null
grep -F "infrastructure_mutation_performed=false" "${root}/missing-runtime.stdout" >/dev/null
grep -F "local_host_mutation_performed=false" "${root}/missing-runtime.stdout" >/dev/null
pass "pending missing runtime env"

if grep -E 'fishystuff-api\.service|fishystuff-dolt\.service|/run/fishystuff/api/env|https://api\.fishystuff\.fish|https://cdn\.fishystuff\.fish' "${root}/packet.stdout" >/dev/null; then
  printf '[gitops-beta-service-start-packet-test] beta service start packet leaked production/shared service material\n' >&2
  exit 1
fi
pass "no production service material in packet"

printf '[gitops-beta-service-start-packet-test] %s checks passed\n' "$pass_count"
