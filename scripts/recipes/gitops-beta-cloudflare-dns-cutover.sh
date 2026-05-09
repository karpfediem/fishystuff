#!/usr/bin/env bash
set -euo pipefail

SCRIPT_PATH="$(readlink -f "${BASH_SOURCE[0]}")"
SCRIPT_DIR="$(cd "$(dirname "$SCRIPT_PATH")" && pwd)"
source "${SCRIPT_DIR}/lib/common.sh"

target_ipv4="$(normalize_named_arg target_ipv4 "${1:-}")"
zone_name="$(normalize_named_arg zone_name "${2:-fishystuff.fish}")"
curl_bin="$(normalize_named_arg curl_bin "${3:-curl}")"

cd "$RECIPE_REPO_ROOT"

fail() {
  echo "$1" >&2
  exit 2
}

require_env_value() {
  local name="$1"
  local expected="$2"
  local value="${!name-}"

  if [[ "$value" != "$expected" ]]; then
    fail "gitops-beta-cloudflare-dns-cutover requires ${name}=${expected}"
  fi
}

require_env_nonempty() {
  local name="$1"
  local value="${!name-}"

  if [[ -z "$value" ]]; then
    fail "gitops-beta-cloudflare-dns-cutover requires ${name}"
  fi
}

require_command_or_executable() {
  local command_name="$1"
  local label="$2"

  if [[ "$command_name" == */* ]]; then
    if [[ ! -x "$command_name" ]]; then
      fail "${label} is not executable: ${command_name}"
    fi
    return
  fi
  if ! command -v "$command_name" >/dev/null 2>&1; then
    echo "missing required command: ${command_name}" >&2
    exit 127
  fi
}

require_beta_deploy_profile() {
  local active_profile="${FISHYSTUFF_OPERATOR_SECRETSPEC_PROFILE:-}"

  case "$active_profile" in
    beta-deploy)
      ;;
    production-deploy | prod-deploy | production)
      fail "gitops-beta-cloudflare-dns-cutover must not run with production SecretSpec profile active: ${active_profile}"
      ;;
    *)
      fail "gitops-beta-cloudflare-dns-cutover requires FISHYSTUFF_OPERATOR_SECRETSPEC_PROFILE=beta-deploy"
      ;;
  esac
}

require_ipv4() {
  local value="$1"

  if [[ ! "$value" =~ ^([0-9]{1,3}\.){3}[0-9]{1,3}$ ]]; then
    fail "target_ipv4 must be an IPv4 address, got: ${value:-<empty>}"
  fi
  IFS=. read -r a b c d <<<"$value"
  for octet in "$a" "$b" "$c" "$d"; do
    if (( 10#$octet > 255 )); then
      fail "target_ipv4 must be an IPv4 address, got: ${value}"
    fi
  done
  if [[ "$value" == "178.104.230.121" ]]; then
    fail "target_ipv4 points at the previous beta host; use the fresh replacement IP"
  fi
}

require_zone() {
  local value="$1"

  if [[ "$value" != "fishystuff.fish" ]]; then
    fail "only the fishystuff.fish Cloudflare zone is supported for beta DNS cutover"
  fi
}

cf_get() {
  local url="$1"
  "$curl_bin" \
    -fsS \
    -H "Authorization: Bearer ${CLOUDFLARE_API_TOKEN:?}" \
    -H "Content-Type: application/json" \
    "$url"
}

cf_patch() {
  local url="$1"
  local payload="$2"
  "$curl_bin" \
    -fsS \
    -X PATCH \
    -H "Authorization: Bearer ${CLOUDFLARE_API_TOKEN:?}" \
    -H "Content-Type: application/json" \
    --data "$payload" \
    "$url"
}

json_success() {
  local file="$1"
  jq -e '.success == true' "$file" >/dev/null
}

require_env_value FISHYSTUFF_GITOPS_ENABLE_BETA_DNS_CUTOVER 1
require_env_value FISHYSTUFF_GITOPS_BETA_DNS_TARGET_IPV4 "$target_ipv4"
require_env_nonempty CLOUDFLARE_API_TOKEN
require_beta_deploy_profile
assert_deployment_configuration_safe beta
assert_beta_infra_cluster_dns_scope_safe
require_ipv4 "$target_ipv4"
require_zone "$zone_name"
require_command_or_executable jq jq
require_command_or_executable "$curl_bin" curl_bin

hostnames=(
  "beta.fishystuff.fish"
  "api.beta.fishystuff.fish"
  "cdn.beta.fishystuff.fish"
  "telemetry.beta.fishystuff.fish"
)

tmp_dir="$(mktemp -d)"
cleanup() {
  rm -rf "$tmp_dir"
}
trap cleanup EXIT

zone_response="${tmp_dir}/zone.json"
cf_get "https://api.cloudflare.com/client/v4/zones?name=${zone_name}" >"$zone_response"
if ! json_success "$zone_response"; then
  fail "Cloudflare zone lookup failed for ${zone_name}"
fi
zone_count="$(jq -er '.result | length' "$zone_response")"
if [[ "$zone_count" != "1" ]]; then
  fail "expected exactly one Cloudflare zone for ${zone_name}, got ${zone_count}"
fi
zone_id="$(jq -er '.result[0].id' "$zone_response")"
resolved_zone_name="$(jq -er '.result[0].name' "$zone_response")"
if [[ "$resolved_zone_name" != "$zone_name" ]]; then
  fail "Cloudflare zone lookup returned ${resolved_zone_name}, expected ${zone_name}"
fi

printf 'gitops_beta_cloudflare_dns_cutover_checked=true\n'
printf 'deployment=beta\n'
printf 'zone_name=%s\n' "$zone_name"
printf 'target_ipv4=%s\n' "$target_ipv4"
printf 'record_type=A\n'

for hostname in "${hostnames[@]}"; do
  case "$hostname" in
    beta.fishystuff.fish | api.beta.fishystuff.fish | cdn.beta.fishystuff.fish | telemetry.beta.fishystuff.fish)
      ;;
    *)
      fail "refusing non-beta DNS hostname: ${hostname}"
      ;;
  esac

  record_response="${tmp_dir}/${hostname}.record.json"
  cf_get "https://api.cloudflare.com/client/v4/zones/${zone_id}/dns_records?type=A&name=${hostname}" >"$record_response"
  if ! json_success "$record_response"; then
    fail "Cloudflare DNS record lookup failed for ${hostname}"
  fi
  record_count="$(jq -er '.result | length' "$record_response")"
  if [[ "$record_count" != "1" ]]; then
    fail "expected exactly one A record for ${hostname}, got ${record_count}"
  fi

  record_id="$(jq -er '.result[0].id' "$record_response")"
  current_content="$(jq -er '.result[0].content' "$record_response")"
  ttl="$(jq -er '.result[0].ttl' "$record_response")"
  proxied="$(jq -r '.result[0].proxied' "$record_response")"
  payload="$(jq -cn \
    --arg type A \
    --arg name "$hostname" \
    --arg content "$target_ipv4" \
    --argjson ttl "$ttl" \
    --argjson proxied "$proxied" \
    '{type: $type, name: $name, content: $content, ttl: $ttl, proxied: $proxied}')"

  update_response="${tmp_dir}/${hostname}.update.json"
  cf_patch "https://api.cloudflare.com/client/v4/zones/${zone_id}/dns_records/${record_id}" "$payload" >"$update_response"
  if ! json_success "$update_response"; then
    fail "Cloudflare DNS record update failed for ${hostname}"
  fi
  updated_content="$(jq -er '.result.content' "$update_response")"
  updated_name="$(jq -er '.result.name' "$update_response")"
  updated_type="$(jq -er '.result.type' "$update_response")"
  if [[ "$updated_name" != "$hostname" || "$updated_type" != "A" || "$updated_content" != "$target_ipv4" ]]; then
    fail "Cloudflare DNS update returned unexpected record for ${hostname}"
  fi

  printf 'record_%s_before=%s\n' "${hostname//./_}" "$current_content"
  printf 'record_%s_after=%s\n' "${hostname//./_}" "$updated_content"
done

printf 'gitops_beta_cloudflare_dns_cutover_ok=true\n'
printf 'cloudflare_dns_mutation_performed=true\n'
printf 'remote_host_mutation_performed=false\n'
printf 'remote_deploy_performed=false\n'
printf 'infrastructure_mutation_performed=false\n'
printf 'production_mutation_performed=false\n'
