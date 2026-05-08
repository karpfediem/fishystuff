#!/usr/bin/env bash
set -euo pipefail

SCRIPT_PATH="$(readlink -f "${BASH_SOURCE[0]}")"
SCRIPT_DIR="$(cd "$(dirname "$SCRIPT_PATH")" && pwd)"
source "${SCRIPT_DIR}/lib/common.sh"

cd "$RECIPE_REPO_ROOT"

export FISHYSTUFF_GITOPS_HOST_HANDOFF_PLAN_TEST_SOURCE_ONLY=1
# Reuse the edge-bundle fixture builder so inventory tests exercise the same
# bundle shape consumed by the handoff plan.
# shellcheck source=scripts/recipes/gitops-production-host-handoff-plan-test.sh
source scripts/recipes/gitops-production-host-handoff-plan-test.sh
unset FISHYSTUFF_GITOPS_HOST_HANDOFF_PLAN_TEST_SOURCE_ONLY

pass_count=0

pass() {
  printf '[gitops-production-host-inventory-test] pass: %s\n' "$1"
  pass_count="$((pass_count + 1))"
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

write_served_state() {
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
make_edge_bundle "${root}/edge-bundle"
write_placeholder_tls "${root}/tls"
write_served_state "${root}/state" "${root}/run" "${root}/site-root" "${root}/cdn-root"
cp "${root}/edge-bundle/artifacts/systemd/unit" "${root}/fishystuff-edge.service"

bash scripts/recipes/gitops-production-host-inventory.sh \
  "${root}/state" \
  "${root}/run" \
  "${root}/edge-bundle" \
  "${root}/fishystuff-edge.service" \
  "${root}/tls/fullchain.pem" \
  "${root}/tls/privkey.pem" \
  production >"${root}/inventory.stdout"

grep -F "gitops_production_host_inventory_ok=production" "${root}/inventory.stdout" >/dev/null
grep -F "edge_bundle_check_ok=true" "${root}/inventory.stdout" >/dev/null
grep -F "edge_caddy_validate=true" "${root}/inventory.stdout" >/dev/null
grep -F "status_release_id=production-release" "${root}/inventory.stdout" >/dev/null
grep -F 'rollback_set_retained_release_ids=["previous-production-release"]' "${root}/inventory.stdout" >/dev/null
grep -F "served_site_link_type=symlink" "${root}/inventory.stdout" >/dev/null
grep -F "installed_edge_unit_matches_bundle=true" "${root}/inventory.stdout" >/dev/null
grep -F "installed_edge_unit_execstart_matches_bundle=true" "${root}/inventory.stdout" >/dev/null
grep -F "installed_edge_unit_execreload_matches_bundle=true" "${root}/inventory.stdout" >/dev/null
grep -F "tls_fullchain_parse_ok=true" "${root}/inventory.stdout" >/dev/null
grep -F "tls_privkey_parse_ok=true" "${root}/inventory.stdout" >/dev/null
grep -F "remote_deploy_performed=false" "${root}/inventory.stdout" >/dev/null
pass "valid production host inventory"

bad_unit="${root}/fishystuff-edge-mismatch.service"
cp "${root}/fishystuff-edge.service" "$bad_unit"
perl -0pi -e 's/--address 127\.0\.0\.1:2019 --force/--address 127.0.0.1:2020 --force/' "$bad_unit"
bash scripts/recipes/gitops-production-host-inventory.sh \
  "${root}/state" \
  "${root}/run" \
  "${root}/edge-bundle" \
  "$bad_unit" \
  "${root}/tls/fullchain.pem" \
  "${root}/tls/privkey.pem" \
  production >"${root}/inventory-bad-unit.stdout"
grep -F "installed_edge_unit_matches_bundle=false" "${root}/inventory-bad-unit.stdout" >/dev/null
grep -F "installed_edge_unit_execreload_matches_bundle=false" "${root}/inventory-bad-unit.stdout" >/dev/null
pass "inventory reports installed unit mismatch without mutating"

bash scripts/recipes/gitops-production-host-inventory.sh \
  "${root}/missing-state" \
  "${root}/missing-run" \
  skip \
  "${root}/missing.service" \
  "${root}/missing-fullchain.pem" \
  "${root}/missing-privkey.pem" \
  production >"${root}/inventory-missing.stdout"
grep -F "edge_bundle_check_ok=skipped" "${root}/inventory-missing.stdout" >/dev/null
grep -F "status_exists=false" "${root}/inventory-missing.stdout" >/dev/null
grep -F "installed_edge_unit_exists=false" "${root}/inventory-missing.stdout" >/dev/null
grep -F "tls_fullchain_parse_ok=false" "${root}/inventory-missing.stdout" >/dev/null
grep -F "tls_privkey_parse_ok=false" "${root}/inventory-missing.stdout" >/dev/null
pass "inventory tolerates missing host files"

printf '[gitops-production-host-inventory-test] %s checks passed\n' "$pass_count"
