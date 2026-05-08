#!/usr/bin/env bash
set -euo pipefail

SCRIPT_PATH="$(readlink -f "${BASH_SOURCE[0]}")"
SCRIPT_DIR="$(cd "$(dirname "$SCRIPT_PATH")" && pwd)"
source "${SCRIPT_DIR}/lib/common.sh"

cd "$RECIPE_REPO_ROOT"

export FISHYSTUFF_GITOPS_HOST_HANDOFF_PLAN_TEST_SOURCE_ONLY=1
# Reuse the host-handoff fixture builders so the proof wrapper exercises the
# same activation/admission/edge shape as the handoff plan.
# shellcheck source=scripts/recipes/gitops-production-host-handoff-plan-test.sh
source scripts/recipes/gitops-production-host-handoff-plan-test.sh
unset FISHYSTUFF_GITOPS_HOST_HANDOFF_PLAN_TEST_SOURCE_ONLY

pass_count=0

pass() {
  printf '[gitops-production-operator-proof-test] pass: %s\n' "$1"
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
    printf '[gitops-production-operator-proof-test] expected failure: %s\n' "$name" >&2
    exit 1
  fi
  if ! grep -F "$expected" "$stderr" >/dev/null; then
    printf '[gitops-production-operator-proof-test] expected %s stderr to contain %q\n' "$name" "$expected" >&2
    cat "$stderr" >&2
    exit 1
  fi
  pass "$name"
}

write_placeholder_tls() {
  local credentials_dir="$1"

  mkdir -p "$credentials_dir"
  openssl req \
    -x509 \
    -newkey rsa:2048 \
    -nodes \
    -keyout "${credentials_dir}/privkey.pem" \
    -out "${credentials_dir}/fullchain.pem" \
    -days 1 \
    -subj "/CN=fishystuff.fish" \
    -addext "subjectAltName=DNS:fishystuff.fish,DNS:api.fishystuff.fish,DNS:cdn.fishystuff.fish,DNS:telemetry.fishystuff.fish" \
    >"${credentials_dir}/openssl.log" 2>&1
}

write_inventory_state() {
  local state_dir="$1"
  local run_dir="$2"
  local site_root="$3"
  local cdn_root="$4"

  mkdir -p \
    "${state_dir}/status" \
    "${state_dir}/active" \
    "${state_dir}/rollback-set/production" \
    "${state_dir}/rollback" \
    "${state_dir}/served/production" \
    "${run_dir}/admission" \
    "${run_dir}/routes" \
    "${run_dir}/roots" \
    "$site_root" \
    "$cdn_root"
  ln -s "$site_root" "${state_dir}/served/production/site"
  ln -s "$cdn_root" "${state_dir}/served/production/cdn"
  jq -n '{
    desired_generation: 42,
    environment: "production",
    host: "production-single-host",
    release_id: "production-release",
    phase: "served",
    admission_state: "passed_fixture",
    rollback_available: true
  }' >"${state_dir}/status/production.json"
  jq -n '{
    desired_generation: 42,
    environment: "production",
    host: "production-single-host",
    release_id: "production-release",
    api_upstream: "http://127.0.0.1:18092"
  }' >"${state_dir}/active/production.json"
  jq -n '{
    desired_generation: 42,
    environment: "production",
    host: "production-single-host",
    current_release_id: "production-release",
    retained_release_count: 1,
    retained_release_ids: ["previous-production-release"],
    retained_release_document_paths: ["/tmp/previous-production-release.json"],
    rollback_set_available: true
  }' >"${state_dir}/rollback-set/production.json"
  jq -n '{
    environment: "production",
    host: "production-single-host",
    current_release_id: "production-release",
    rollback_release_id: "previous-production-release",
    rollback_available: true
  }' >"${state_dir}/rollback/production.json"
  jq -n '{
    environment: "production",
    host: "production-single-host",
    release_id: "production-release",
    admission_state: "passed_fixture",
    url: "http://127.0.0.1:18092/api/v1/meta"
  }' >"${run_dir}/admission/production.json"
  jq -n '{
    environment: "production",
    host: "production-single-host",
    release_id: "production-release",
    api_upstream: "http://127.0.0.1:18092",
    state: "selected_local_route"
  }' >"${run_dir}/routes/production.json"
}

root="$(mktemp -d)"
make_fixture "$root"
make_edge_bundle "${root}/edge-bundle"
make_fake_deploy "${root}/fishystuff_deploy"
write_placeholder_tls "${root}/tls"
write_inventory_state "${root}/state" "${root}/run" "${root}/site-root" "${root}/cdn-root"
cp "${root}/edge-bundle/artifacts/systemd/unit" "${root}/fishystuff-edge.service"

draft="$(cat "${root}/draft.path")"
summary="$(cat "${root}/summary.path")"
admission="$(cat "${root}/admission.path")"
proof_dir="${root}/proofs"

bash scripts/recipes/gitops-production-operator-proof.sh \
  "$proof_dir" \
  "$draft" \
  "$summary" \
  "$admission" \
  "${root}/edge-bundle" \
  "${root}/fishystuff_deploy" \
  false \
  "" \
  "" \
  "${root}/state" \
  "${root}/run" \
  "${root}/fishystuff-edge.service" \
  "${root}/tls/fullchain.pem" \
  "${root}/tls/privkey.pem" \
  production >"${root}/proof.stdout"

proof_file="$(awk -F= '$1 == "gitops_production_operator_proof_ok" { print $2 }' "${root}/proof.stdout")"
test -n "$proof_file"
test -f "$proof_file"

jq -e \
  --arg draft "$draft" \
  --arg summary "$summary" \
  --arg admission "$admission" \
  --arg edge_bundle "${root}/edge-bundle" \
  '
    .schema == "fishystuff.gitops.production-operator-proof.v1"
    and .environment == "production"
    and .inputs.draft_file == $draft
    and .inputs.summary_file == $summary
    and .inputs.admission_file == $admission
    and .inputs.edge_bundle == $edge_bundle
    and .commands.inventory.success == true
    and .commands.preflight.success == true
    and .commands.host_handoff_plan.success == true
    and .commands.inventory.kv.edge_caddy_validate == "true"
    and .commands.preflight.kv.gitops_production_preflight_ok == $draft
    and .commands.host_handoff_plan.kv.edge_caddy_validate == "true"
    and .remote_deploy_performed == false
    and .infrastructure_mutation_performed == false
  ' "$proof_file" >/dev/null
grep -F "remote_deploy_performed=false" "${root}/proof.stdout" >/dev/null
pass "valid production operator proof"

expect_fail_contains \
  "missing admission evidence" \
  "gitops-production-operator-proof requires admission_file or FISHYSTUFF_GITOPS_ADMISSION_EVIDENCE_FILE" \
  bash scripts/recipes/gitops-production-operator-proof.sh \
    "$proof_dir" \
    "$draft" \
    "$summary" \
    "" \
    "${root}/edge-bundle" \
    "${root}/fishystuff_deploy" \
    false \
    "" \
    "" \
    "${root}/state" \
    "${root}/run" \
    "${root}/fishystuff-edge.service" \
    "${root}/tls/fullchain.pem" \
    "${root}/tls/privkey.pem" \
    production

printf '[gitops-production-operator-proof-test] %s checks passed\n' "$pass_count"
