#!/usr/bin/env bash
set -euo pipefail

SCRIPT_PATH="$(readlink -f "${BASH_SOURCE[0]}")"
SCRIPT_DIR="$(cd "$(dirname "$SCRIPT_PATH")" && pwd)"
source "${SCRIPT_DIR}/lib/common.sh"

cd "$RECIPE_REPO_ROOT"

pass_count=0

pass() {
  printf '[deploy-safety-test] pass: %s\n' "$1"
  pass_count="$((pass_count + 1))"
}

expect_ok() {
  local name="$1"
  shift
  if "$@"; then
    pass "$name"
    return
  fi
  printf '[deploy-safety-test] expected success: %s\n' "$name" >&2
  exit 1
}

expect_fail() {
  local name="$1"
  shift
  if "$@"; then
    printf '[deploy-safety-test] expected failure: %s\n' "$name" >&2
    exit 1
  fi
  pass "$name"
}

expect_eq() {
  local name="$1"
  local expected="$2"
  local actual="$3"
  if [[ "$actual" == "$expected" ]]; then
    pass "$name"
    return
  fi
  printf '[deploy-safety-test] expected %s to equal %q, got %q\n' "$name" "$expected" "$actual" >&2
  exit 1
}

authority_value() {
  local deployment="$1"
  local key="$2"
  shift 2
  bash scripts/recipes/deploy-authority-check.sh "$deployment" "$@" \
    | awk -F': ' -v key="$key" '$1 == key { print $2; found = 1 } END { exit found ? 0 : 1 }'
}

expect_ok "beta default safety" assert_deployment_configuration_safe beta
expect_ok "production default safety" assert_deployment_configuration_safe production
expect_ok "beta infra DNS cluster default is safe" assert_beta_infra_cluster_dns_scope_safe

expect_fail "beta site URL cannot point at production" \
  env FISHYSTUFF_BETA_SITE_BASE_URL=https://fishystuff.fish/ bash -c \
    'source scripts/recipes/lib/common.sh; assert_deployment_configuration_safe beta'

expect_fail "production cannot run under beta secret scope" \
  env FISHYSTUFF_OPERATOR_SECRETSPEC_PROFILE=beta-deploy bash -c \
    'source scripts/recipes/lib/common.sh; assert_deployment_configuration_safe production'

expect_fail "beta resident target cannot point at production" \
  env FISHYSTUFF_BETA_RESIDENT_TARGET=root@fishystuff.fish bash -c \
    'source scripts/recipes/lib/common.sh; assert_deployment_configuration_safe beta'

expect_fail "beta telemetry target cannot point at production" \
  env FISHYSTUFF_BETA_TELEMETRY_TARGET=root@fishystuff.fish bash -c \
    'source scripts/recipes/lib/common.sh; assert_deployment_configuration_safe beta'

expect_fail "beta telemetry target must be dedicated" \
  env FISHYSTUFF_BETA_TELEMETRY_TARGET=root@beta.fishystuff.fish bash -c \
    'source scripts/recipes/lib/common.sh; assert_deployment_configuration_safe beta'

expect_fail "production Dolt branch cannot be beta" \
  env FISHYSTUFF_PRODUCTION_DOLT_REMOTE_BRANCH=beta bash -c \
    'source scripts/recipes/lib/common.sh; assert_deployment_configuration_safe production'

expect_fail "production control target cannot use beta mgmt-root" \
  env FISHYSTUFF_PRODUCTION_CONTROL_TARGET=mgmt-root bash -c \
    'source scripts/recipes/lib/common.sh; assert_deployment_configuration_safe production'

expect_fail "beta infra DNS cluster cannot be dotted" \
  env FISHYSTUFF_HETZNER_CLUSTER=fishystuff.fish bash -c \
    'source scripts/recipes/lib/common.sh; assert_beta_infra_cluster_dns_scope_safe'

expect_fail "beta infra DNS cluster cannot be production" \
  env FISHYSTUFF_HETZNER_CLUSTER=production bash -c \
    'source scripts/recipes/lib/common.sh; assert_beta_infra_cluster_dns_scope_safe'

expect_fail "beta infra DNS cluster cannot be api" \
  env FISHYSTUFF_HETZNER_CLUSTER=api bash -c \
    'source scripts/recipes/lib/common.sh; assert_beta_infra_cluster_dns_scope_safe'

expect_fail "beta resident manifest cannot carry prod host" \
  bash -c 'source scripts/recipes/lib/common.sh; assert_resident_push_scope_safe beta root@beta.fishystuff.fish root@telemetry.beta.fishystuff.fish site-nbg1-beta telemetry-nbg1 site-nbg1-prod https://beta.fishystuff.fish https://api.beta.fishystuff.fish https://cdn.beta.fishystuff.fish https://telemetry.beta.fishystuff.fish beta'

expect_fail "beta resident push requires telemetry host" \
  bash -c 'source scripts/recipes/lib/common.sh; assert_resident_push_scope_safe beta root@beta.fishystuff.fish "" site-nbg1-beta "" "" https://beta.fishystuff.fish https://api.beta.fishystuff.fish https://cdn.beta.fishystuff.fish https://telemetry.beta.fishystuff.fish beta'

expect_fail "beta resident push requires dedicated telemetry target" \
  bash -c 'source scripts/recipes/lib/common.sh; assert_resident_push_scope_safe beta root@beta.fishystuff.fish root@beta.fishystuff.fish site-nbg1-beta telemetry-nbg1 "" https://beta.fishystuff.fish https://api.beta.fishystuff.fish https://cdn.beta.fishystuff.fish https://telemetry.beta.fishystuff.fish beta'

expect_ok "production resident push scope" \
  bash -c 'source scripts/recipes/lib/common.sh; assert_resident_push_scope_safe production root@116.203.126.191 "" site-nbg1-prod "" site-nbg1-prod https://fishystuff.fish https://api.fishystuff.fish https://cdn.fishystuff.fish https://telemetry.fishystuff.fish main'

expect_eq "production telemetry tunnel has no beta fallback" "" "$(deployment_tunnel_target production grafana)"
expect_eq "beta telemetry tunnel target" "root@telemetry.beta.fishystuff.fish" "$(deployment_tunnel_target beta grafana)"
expect_eq "production resident target defaults to production DNS" "root@fishystuff.fish" "$(deployment_resident_target production)"
expect_eq "production control target defaults to production site" "root@fishystuff.fish" "$(deployment_control_target production)"
expect_eq "production default services stay lightweight" $'api\ndolt\nedge\nsite\ncdn\nvector' "$(deployment_default_services production)"
expect_eq "production default mutating services stay lightweight" $'api\ndolt\nedge\nsite' "$(deployment_default_mutating_services production)"
expect_eq "production resident bundle services stay lightweight" $'api\ndolt\nedge' "$(deployment_resident_bundle_services production)"
expect_eq "beta authority check profile" "beta-deploy" "$(authority_value beta secretspec_profile)"
expect_eq "beta authority check Dolt branch" "beta" "$(authority_value beta dolt_remote_branch)"
expect_eq "beta authority check DNS-01 risk is explicit" "accepted_parent_zone_scope_until_split" "$(authority_value beta dns_mutation_authority_risk)"
expect_eq "authority check reports no mutation by default" "none" "$(authority_value beta remote_mutation)"
expect_eq "authority check can report pending deploy mutation" "pending_after_report" \
  "$(RECIPE_DEPLOY_AUTHORITY_REMOTE_MUTATION=pending_after_report authority_value beta remote_mutation)"
expect_eq "production authority check profile" "production-deploy" "$(authority_value production secretspec_profile)"
expect_eq "production authority check Dolt branch" "main" "$(authority_value production dolt_remote_branch)"
expect_eq "production authority check default services are lightweight" "api,dolt,edge,site" "$(authority_value production selected_mutating_services)"
expect_eq "explicit site deploy expands edge/site/cdn" "edge,site,cdn" "$(authority_value beta selected_mutating_services site)"

expect_fail "authority check refuses API without Dolt" \
  bash scripts/recipes/deploy-authority-check.sh beta api

expect_eq "authority check allows documented API with active Dolt" "true" \
  "$(authority_value beta api_with_active_dolt api --allow-api-with-active-dolt --reason 'inspection only')"

expect_ok "bundled resident beta manifest has safe target identity" \
  jq -e '
    .deployment_environment == "beta"
    and .hostname == "site-nbg1-beta"
    and .telemetry_hostname == "telemetry-nbg1"
    and .prod_hostname == ""
    and .public_urls.site_base_url == "https://beta.fishystuff.fish"
    and .public_urls.api_base_url == "https://api.beta.fishystuff.fish"
    and .public_urls.cdn_base_url == "https://cdn.beta.fishystuff.fish"
    and .public_urls.telemetry_base_url == "https://telemetry.beta.fishystuff.fish"
    and .dolt.remote_branch == "beta"
  ' mgmt/resident-beta/files/resident-manifest.json >/dev/null

printf '[deploy-safety-test] %s checks passed\n' "$pass_count"
