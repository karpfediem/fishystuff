#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
CDN_BASE_URL="${FISHYSTUFF_CDN_CHECK_BASE_URL:-https://cdn.fishystuff.fish}"
CDN_CHECK_REFERRER_URL="${FISHYSTUFF_CDN_CHECK_REFERRER_URL:-https://fishystuff.fish/map/}"
CDN_CHECK_ORIGIN_URL="${FISHYSTUFF_CDN_CHECK_ORIGIN_URL:-https://fishystuff.fish}"
MAP_CACHE_KEY="${FISHYSTUFF_RUNTIME_MAP_ASSET_CACHE_KEY:-$("$ROOT_DIR/tools/scripts/resolve_map_runtime_cache_key.sh")}"
CURL_CONNECT_TIMEOUT="${CDN_CHECK_CONNECT_TIMEOUT_SECONDS:-10}"
CURL_MAX_TIME="${CDN_CHECK_MAX_TIME_SECONDS:-30}"

trim_trailing_slash() {
  printf '%s' "${1%/}"
}

CDN_BASE_URL="$(trim_trailing_slash "$CDN_BASE_URL")"

fetch_json() {
  local url="$1"
  local out_file="$2"
  curl -fsS \
    --connect-timeout "$CURL_CONNECT_TIMEOUT" \
    --max-time "$CURL_MAX_TIME" \
    -H "Referer: ${CDN_CHECK_REFERRER_URL}" \
    -H "Origin: ${CDN_CHECK_ORIGIN_URL}" \
    "$url" > "$out_file"
}

check_url_exists() {
  local url="$1"
  curl -fsSI \
    --connect-timeout "$CURL_CONNECT_TIMEOUT" \
    --max-time "$CURL_MAX_TIME" \
    -H "Referer: ${CDN_CHECK_REFERRER_URL}" \
    -H "Origin: ${CDN_CHECK_ORIGIN_URL}" \
    "$url" >/dev/null
}

check_url_cors() {
  local url="$1"
  local headers_file
  local allow_origin

  headers_file="$(mktemp)"
  curl -fsSI \
    --connect-timeout "$CURL_CONNECT_TIMEOUT" \
    --max-time "$CURL_MAX_TIME" \
    -H "Referer: ${CDN_CHECK_REFERRER_URL}" \
    -H "Origin: ${CDN_CHECK_ORIGIN_URL}" \
    "$url" >"$headers_file"

  allow_origin="$(
    awk '
      BEGIN { IGNORECASE = 1 }
      /^Access-Control-Allow-Origin:/ {
        sub(/^[^:]+:[[:space:]]*/, "", $0)
        sub(/\r$/, "", $0)
        print
        exit
      }
    ' "$headers_file"
  )"
  rm -f "$headers_file"

  if [ -z "$allow_origin" ]; then
    echo "missing Access-Control-Allow-Origin header: $url" >&2
    exit 1
  fi

  if [ "$allow_origin" != "*" ] && [ "$allow_origin" != "$CDN_CHECK_ORIGIN_URL" ]; then
    echo "unexpected Access-Control-Allow-Origin header for $url: $allow_origin" >&2
    exit 1
  fi
}

join_url() {
  local base="$1"
  local path="$2"
  if [[ "$path" == /* ]]; then
    printf '%s%s\n' "$base" "$path"
  else
    printf '%s/%s\n' "$base" "$path"
  fi
}

tmp_current_manifest="$(mktemp)"
tmp_stable_manifest="$(mktemp)"
cleanup() {
  rm -f "$tmp_current_manifest" "$tmp_stable_manifest"
}
trap cleanup EXIT

current_manifest_url="$(join_url "$CDN_BASE_URL" "/map/runtime-manifest.${MAP_CACHE_KEY}.json")"
stable_manifest_url="$(join_url "$CDN_BASE_URL" "/map/runtime-manifest.json")"

echo "Checking CDN map runtime assets against $CDN_BASE_URL" >&2
echo "Expected map runtime cache key: $MAP_CACHE_KEY" >&2
echo "Using referrer: $CDN_CHECK_REFERRER_URL" >&2

fetch_json "$current_manifest_url" "$tmp_current_manifest"
fetch_json "$stable_manifest_url" "$tmp_stable_manifest"

current_module="$(jq -r '.module // empty' "$tmp_current_manifest")"
current_wasm="$(jq -r '.wasm // empty' "$tmp_current_manifest")"
stable_module="$(jq -r '.module // empty' "$tmp_stable_manifest")"
stable_wasm="$(jq -r '.wasm // empty' "$tmp_stable_manifest")"

if [ -z "$current_module" ] || [ -z "$current_wasm" ]; then
  echo "current cache-keyed runtime manifest is missing module/wasm entries: $current_manifest_url" >&2
  exit 1
fi

if [ -z "$stable_module" ] || [ -z "$stable_wasm" ]; then
  echo "stable runtime manifest is missing module/wasm entries: $stable_manifest_url" >&2
  exit 1
fi

if [ "$stable_module" != "$current_module" ] || [ "$stable_wasm" != "$current_wasm" ]; then
  echo "stable runtime manifest does not match the current cache-keyed runtime manifest" >&2
  echo "  stable module:  $stable_module" >&2
  echo "  current module: $current_module" >&2
  echo "  stable wasm:    $stable_wasm" >&2
  echo "  current wasm:   $current_wasm" >&2
  exit 1
fi

required_urls=(
  "$current_manifest_url"
  "$stable_manifest_url"
  "$(join_url "$CDN_BASE_URL" "/map/${current_module}")"
  "$(join_url "$CDN_BASE_URL" "/map/${current_wasm}")"
  "$(join_url "$CDN_BASE_URL" "/map/loader.js")"
  "$(join_url "$CDN_BASE_URL" "/map/map-host.js")"
  "$(join_url "$CDN_BASE_URL" "/map/ui/fishystuff.css")"
  "$(join_url "$CDN_BASE_URL" "/fields/regions.v1.bin")"
  "$(join_url "$CDN_BASE_URL" "/fields/region_groups.v1.bin")"
  "$(join_url "$CDN_BASE_URL" "/images/exact_lookup/zone_mask.v1.bin")"
  "$(join_url "$CDN_BASE_URL" "/images/tiles/minimap_visual/v1/tileset.json")"
  "$(join_url "$CDN_BASE_URL" "/images/tiles/zone_mask_visual/v1/tileset.json")"
)

cors_required_urls=(
  "$current_manifest_url"
  "$stable_manifest_url"
  "$(join_url "$CDN_BASE_URL" "/map/${current_module}")"
  "$(join_url "$CDN_BASE_URL" "/map/${current_wasm}")"
  "$(join_url "$CDN_BASE_URL" "/fields/regions.v1.bin")"
  "$(join_url "$CDN_BASE_URL" "/fields/region_groups.v1.bin")"
  "$(join_url "$CDN_BASE_URL" "/images/exact_lookup/zone_mask.v1.bin")"
  "$(join_url "$CDN_BASE_URL" "/images/tiles/minimap_visual/v1/tileset.json")"
  "$(join_url "$CDN_BASE_URL" "/images/tiles/zone_mask_visual/v1/tileset.json")"
)

for url in "${required_urls[@]}"; do
  check_url_exists "$url"
done

for url in "${cors_required_urls[@]}"; do
  check_url_cors "$url"
done

echo "CDN map runtime assets are reachable and CORS-readable." >&2
