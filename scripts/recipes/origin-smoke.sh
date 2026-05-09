#!/usr/bin/env bash
set -euo pipefail

SCRIPT_PATH="$(readlink -f "${BASH_SOURCE[0]}")"
SCRIPT_DIR="$(cd "$(dirname "$SCRIPT_PATH")" && pwd)"
source "${SCRIPT_DIR}/lib/common.sh"
RECIPE_SMOKE_LOG_PREFIX="origin-smoke"
source "${SCRIPT_DIR}/lib/http-smoke.sh"

cd "$RECIPE_REPO_ROOT"
trap smoke_cleanup_temp_files EXIT

deployment="${1-}"
origin_ipv4="${2:-${FISHYSTUFF_ORIGIN_SMOKE_IPV4:-${FISHYSTUFF_SMOKE_ORIGIN_IPV4:-}}}"
require_value "$deployment" "usage: origin-smoke.sh <deployment> <origin-ipv4>"
require_value "$origin_ipv4" "usage: origin-smoke.sh <deployment> <origin-ipv4>"
deployment="$(canonical_deployment_name "$deployment")"

if [[ ! "$origin_ipv4" =~ ^([0-9]{1,3}\.){3}[0-9]{1,3}$ ]]; then
  echo "origin IPv4 does not look valid: $origin_ipv4" >&2
  exit 2
fi
IFS=. read -r origin_octet_a origin_octet_b origin_octet_c origin_octet_d <<< "$origin_ipv4"
for origin_octet in "$origin_octet_a" "$origin_octet_b" "$origin_octet_c" "$origin_octet_d"; do
  if (( 10#$origin_octet > 255 )); then
    echo "origin IPv4 does not look valid: $origin_ipv4" >&2
    exit 2
  fi
done

case "$deployment" in
  local)
    echo "origin smoke is for remote deployments; use just smoke local for local checks" >&2
    exit 2
    ;;
  *)
    assert_deployment_public_urls_safe "$deployment"
    ;;
esac

api_base_url="$(deployment_public_base_url "$deployment" "api")"
site_base_url="$(deployment_public_base_url "$deployment" "site")"
cdn_base_url="$(deployment_public_base_url "$deployment" "cdn")"

api_base_url="${api_base_url%/}"
site_base_url="${site_base_url%/}"
cdn_base_url="${cdn_base_url%/}"
timeout_secs="${FISHYSTUFF_SMOKE_TIMEOUT_SECS:-900}"
interval_secs="${FISHYSTUFF_SMOKE_INTERVAL_SECS:-5}"
started_at="$(date +%s)"

resolve_for_url() {
  local url="$1"
  local scheme=""
  local rest=""
  local hostport=""
  local host=""
  local port=""

  case "$url" in
    http://*) scheme="http"; rest="${url#http://}" ;;
    https://*) scheme="https"; rest="${url#https://}" ;;
    *)
      echo "unsupported URL for origin smoke: $url" >&2
      exit 2
      ;;
  esac

  hostport="${rest%%/*}"
  host="${hostport%%:*}"
  if [[ "$hostport" == *:* ]]; then
    port="${hostport##*:}"
  elif [[ "$scheme" == "https" ]]; then
    port="443"
  else
    port="80"
  fi

  printf '%s:%s:%s' "$host" "$port" "$origin_ipv4"
}

probe() {
  local name="$1"
  local url="$2"
  local expected="${3:-200}"
  local resolve

  resolve="$(resolve_for_url "$url")"
  smoke_fetch "$name through $origin_ipv4" "$url" "$expected" "$resolve"
}

probe_site_contract() {
  probe "site homepage" "$site_base_url/" || return 1
  smoke_assert_site_html_contract "$deployment" "$SMOKE_LAST_BODY" || return 1
  smoke_headers_match "$SMOKE_LAST_HEADERS" '^cache-control:.*no-store' "site homepage cache policy" || return 1

  probe "site runtime config" "$site_base_url/runtime-config.js" || return 1
  smoke_assert_runtime_config_contract "$deployment" "$SMOKE_LAST_BODY" || return 1
  SMOKE_LAST_RUNTIME_MAP_ASSET_CACHE_KEY="$(smoke_runtime_config_map_asset_cache_key "$SMOKE_LAST_BODY")"
  smoke_headers_match "$SMOKE_LAST_HEADERS" '^cache-control:.*no-store' "runtime config cache policy" || return 1

  probe "site asset manifest" "$site_base_url/asset-manifest.json" || return 1
  smoke_assert_asset_manifest_contract "$SMOKE_LAST_BODY" || {
    echo "[origin-smoke] site asset manifest failed integrity contract" >&2
    return 1
  }
  smoke_headers_match "$SMOKE_LAST_HEADERS" '^cache-control:.*no-store' "asset manifest cache policy" || return 1
}

probe_cdn_runtime_contract() {
  local cache_key="${SMOKE_LAST_RUNTIME_MAP_ASSET_CACHE_KEY:-}"
  local manifest_body=""
  local keyed_manifest_body=""
  local keyed_manifest_url=""
  local module_path=""
  local wasm_path=""
  local module_url=""
  local wasm_url=""

  probe "cdn runtime manifest" "$cdn_base_url/map/runtime-manifest.json" || return 1
  manifest_body="$SMOKE_LAST_BODY"
  smoke_assert_cdn_runtime_manifest_contract "$manifest_body" || {
    echo "[origin-smoke] cdn runtime manifest failed module/wasm contract" >&2
    return 1
  }
  smoke_headers_match "$SMOKE_LAST_HEADERS" '^cache-control:.*no-store' "cdn runtime manifest cache policy" || return 1

  if [[ -n "$cache_key" ]]; then
    keyed_manifest_url="$(smoke_join_url "$cdn_base_url/map" "runtime-manifest.$cache_key.json")"
    probe "cdn cache-keyed runtime manifest" "$keyed_manifest_url" || return 1
    keyed_manifest_body="$SMOKE_LAST_BODY"
    smoke_assert_cdn_runtime_manifest_contract "$keyed_manifest_body" || {
      echo "[origin-smoke] cdn cache-keyed runtime manifest failed module/wasm contract" >&2
      return 1
    }
    smoke_headers_match "$SMOKE_LAST_HEADERS" '^cache-control:.*max-age=31536000.*immutable' "cdn cache-keyed runtime manifest cache policy" || return 1
    if ! cmp -s "$manifest_body" "$keyed_manifest_body"; then
      echo "[origin-smoke] cdn stable and cache-keyed runtime manifests differ for key: $cache_key" >&2
      return 1
    fi
  fi

  module_path="$(jq -er '.module' "$manifest_body")"
  wasm_path="$(jq -er '.wasm' "$manifest_body")"
  module_url="$(smoke_join_url "$cdn_base_url/map" "$module_path")"
  wasm_url="$(smoke_join_url "$cdn_base_url/map" "$wasm_path")"

  probe "cdn runtime module" "$module_url" || return 1
  smoke_headers_match "$SMOKE_LAST_HEADERS" '^cache-control:.*max-age=31536000.*immutable' "cdn runtime module cache policy" || return 1

  probe "cdn runtime wasm" "$wasm_url" || return 1
  smoke_headers_match "$SMOKE_LAST_HEADERS" '^cache-control:.*max-age=31536000.*immutable' "cdn runtime wasm cache policy" || return 1
}

run_once() {
  local failed=0
  probe_site_contract || failed=1
  probe "api healthz" "$api_base_url/healthz" || failed=1
  probe "api readyz" "$api_base_url/readyz" || failed=1
  probe "api meta" "$api_base_url/api/v1/meta" || failed=1
  probe "calculator catalog" "$api_base_url/api/v1/calculator?lang=en" || failed=1
  probe "calculator datastar init" "$api_base_url/api/v1/calculator/datastar/init?lang=en&locale=en-US" || failed=1
  probe_cdn_runtime_contract || failed=1
  return "$failed"
}

while true; do
  if run_once; then
    printf '[origin-smoke] %s passed through %s\n' "$deployment" "$origin_ipv4"
    exit 0
  fi

  now="$(date +%s)"
  if (( now - started_at >= timeout_secs )); then
    printf '[origin-smoke] %s failed through %s after %ss\n' "$deployment" "$origin_ipv4" "$timeout_secs" >&2
    exit 1
  fi
  sleep "$interval_secs"
done
