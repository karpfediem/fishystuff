#!/usr/bin/env bash
set -euo pipefail

SCRIPT_PATH="$(readlink -f "${BASH_SOURCE[0]}")"
SCRIPT_DIR="$(cd "$(dirname "$SCRIPT_PATH")" && pwd)"
source "${SCRIPT_DIR}/lib/common.sh"
source "${SCRIPT_DIR}/lib/http-smoke.sh"

cd "$RECIPE_REPO_ROOT"
trap smoke_cleanup_temp_files EXIT

deployment="${1-}"
require_value "$deployment" "usage: smoke.sh <deployment>"
deployment="$(canonical_deployment_name "$deployment")"

case "$deployment" in
  local)
    api_base_url="$(deployment_public_base_url "$deployment" "api")"
    site_base_url="$(deployment_public_base_url "$deployment" "site")"
    cdn_base_url="$(deployment_public_base_url "$deployment" "cdn")"
    ;;
  *)
    assert_deployment_public_urls_safe "$deployment"
    api_base_url="$(deployment_public_base_url "$deployment" "api")"
    site_base_url="$(deployment_public_base_url "$deployment" "site")"
    cdn_base_url="$(deployment_public_base_url "$deployment" "cdn")"
    ;;
esac

api_base_url="${api_base_url%/}"
site_base_url="${site_base_url%/}"
cdn_base_url="${cdn_base_url%/}"
timeout_secs="${FISHYSTUFF_SMOKE_TIMEOUT_SECS:-900}"
interval_secs="${FISHYSTUFF_SMOKE_INTERVAL_SECS:-5}"
started_at="$(date +%s)"

probe() {
  local name="$1"
  local url="$2"
  local expected="${3:-200}"

  smoke_fetch "$name" "$url" "$expected"
}

probe_site_contract() {
  smoke_fetch "site homepage" "$site_base_url/" || return 1
  smoke_assert_site_html_contract "$deployment" "$SMOKE_LAST_BODY" || return 1
  if [[ "$deployment" != "local" ]]; then
    smoke_headers_match "$SMOKE_LAST_HEADERS" '^cache-control:.*no-store' "site homepage cache policy" || return 1
  fi

  smoke_fetch "site runtime config" "$site_base_url/runtime-config.js" || return 1
  smoke_assert_runtime_config_contract "$deployment" "$SMOKE_LAST_BODY" || return 1
  SMOKE_LAST_RUNTIME_MAP_ASSET_CACHE_KEY="$(smoke_runtime_config_map_asset_cache_key "$SMOKE_LAST_BODY")"
  if [[ "$deployment" != "local" ]]; then
    smoke_headers_match "$SMOKE_LAST_HEADERS" '^cache-control:.*no-store' "runtime config cache policy" || return 1
  fi

  smoke_fetch "site asset manifest" "$site_base_url/asset-manifest.json" || return 1
  smoke_assert_asset_manifest_contract "$SMOKE_LAST_BODY" || {
    echo "[smoke] site asset manifest failed integrity contract" >&2
    return 1
  }
  if [[ "$deployment" != "local" ]]; then
    smoke_headers_match "$SMOKE_LAST_HEADERS" '^cache-control:.*no-store' "asset manifest cache policy" || return 1
  fi
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

  smoke_fetch "cdn runtime manifest" "$cdn_base_url/map/runtime-manifest.json" || return 1
  manifest_body="$SMOKE_LAST_BODY"
  smoke_assert_cdn_runtime_manifest_contract "$manifest_body" || {
    echo "[smoke] cdn runtime manifest failed module/wasm contract" >&2
    return 1
  }
  if [[ "$deployment" != "local" ]]; then
    smoke_headers_match "$SMOKE_LAST_HEADERS" '^cache-control:.*no-store' "cdn runtime manifest cache policy" || return 1
  fi

  if [[ -n "$cache_key" ]]; then
    keyed_manifest_url="$(smoke_join_url "$cdn_base_url/map" "runtime-manifest.$cache_key.json")"
    smoke_fetch "cdn cache-keyed runtime manifest" "$keyed_manifest_url" || return 1
    keyed_manifest_body="$SMOKE_LAST_BODY"
    smoke_assert_cdn_runtime_manifest_contract "$keyed_manifest_body" || {
      echo "[smoke] cdn cache-keyed runtime manifest failed module/wasm contract" >&2
      return 1
    }
    if [[ "$deployment" != "local" ]]; then
      smoke_headers_match "$SMOKE_LAST_HEADERS" '^cache-control:.*max-age=31536000.*immutable' "cdn cache-keyed runtime manifest cache policy" || return 1
    fi
    if ! cmp -s "$manifest_body" "$keyed_manifest_body"; then
      echo "[smoke] cdn stable and cache-keyed runtime manifests differ for key: $cache_key" >&2
      return 1
    fi
  fi

  module_path="$(jq -er '.module' "$manifest_body")"
  wasm_path="$(jq -er '.wasm' "$manifest_body")"
  module_url="$(smoke_join_url "$cdn_base_url/map" "$module_path")"
  wasm_url="$(smoke_join_url "$cdn_base_url/map" "$wasm_path")"

  smoke_fetch "cdn runtime module" "$module_url" || return 1
  if [[ "$deployment" != "local" ]]; then
    smoke_headers_match "$SMOKE_LAST_HEADERS" '^cache-control:.*max-age=31536000.*immutable' "cdn runtime module cache policy" || return 1
  fi

  smoke_fetch "cdn runtime wasm" "$wasm_url" || return 1
  if [[ "$deployment" != "local" ]]; then
    smoke_headers_match "$SMOKE_LAST_HEADERS" '^cache-control:.*max-age=31536000.*immutable' "cdn runtime wasm cache policy" || return 1
  fi
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
    printf '[smoke] %s passed\n' "$deployment"
    exit 0
  fi

  now="$(date +%s)"
  if (( now - started_at >= timeout_secs )); then
    printf '[smoke] %s failed after %ss\n' "$deployment" "$timeout_secs" >&2
    exit 1
  fi
  sleep "$interval_secs"
done
