#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
# shellcheck source=tools/scripts/public-endpoints.sh
source "$ROOT_DIR/tools/scripts/public-endpoints.sh"
fishystuff_resolve_public_base_urls

CDN_BASE_URL="${FISHYSTUFF_CDN_CHECK_BASE_URL:-$FISHYSTUFF_RESOLVED_PUBLIC_CDN_BASE_URL}"
CDN_CHECK_REFERRER_URL="${FISHYSTUFF_CDN_CHECK_REFERRER_URL:-$FISHYSTUFF_RESOLVED_PUBLIC_SITE_BASE_URL/map/}"
CDN_CHECK_ORIGIN_URL="${FISHYSTUFF_CDN_CHECK_ORIGIN_URL:-$FISHYSTUFF_RESOLVED_PUBLIC_SITE_BASE_URL}"
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

fetch_headers() {
  local url="$1"
  local out_file="$2"
  curl -fsSI \
    --connect-timeout "$CURL_CONNECT_TIMEOUT" \
    --max-time "$CURL_MAX_TIME" \
    -H "Referer: ${CDN_CHECK_REFERRER_URL}" \
    -H "Origin: ${CDN_CHECK_ORIGIN_URL}" \
    "$url" >"$out_file"
}

read_header_value() {
  local headers_file="$1"
  local header_name="$2"

  awk -v header_name="$header_name" '
    BEGIN {
      prefix = tolower(header_name ":")
    }
    index(tolower($0), prefix) == 1 {
      sub(/^[^:]+:[[:space:]]*/, "", $0)
      sub(/\r$/, "", $0)
      print
      exit
    }
  ' "$headers_file"
}

check_url_exists() {
  local url="$1"
  local headers_file

  headers_file="$(mktemp)"
  fetch_headers "$url" "$headers_file"
  rm -f "$headers_file"
}

check_url_cache_control() {
  local url="$1"
  shift
  local headers_file
  local cache_control
  local normalized_cache_control
  local expected
  local normalized_expected

  headers_file="$(mktemp)"
  fetch_headers "$url" "$headers_file"
  cache_control="$(read_header_value "$headers_file" "Cache-Control")"
  rm -f "$headers_file"

  if [ -z "$cache_control" ]; then
    echo "missing Cache-Control header: $url" >&2
    exit 1
  fi

  normalized_cache_control="$(printf '%s' "$cache_control" | tr '[:upper:]' '[:lower:]')"
  for expected in "$@"; do
    normalized_expected="$(printf '%s' "$expected" | tr '[:upper:]' '[:lower:]')"
    case "$normalized_cache_control" in
      *"$normalized_expected"*) ;;
      *)
        echo "unexpected Cache-Control header for $url: $cache_control" >&2
        echo "  missing expected value: $expected" >&2
        exit 1
        ;;
    esac
  done
}

check_url_cors() {
  local url="$1"
  local headers_file
  local allow_origin

  headers_file="$(mktemp)"
  fetch_headers "$url" "$headers_file"
  allow_origin="$(read_header_value "$headers_file" "Access-Control-Allow-Origin")"
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
  "$(join_url "$CDN_BASE_URL" "/map/map-host.js")"
  "$(join_url "$CDN_BASE_URL" "/map/ui/fishystuff.css")"
  "$(join_url "$CDN_BASE_URL" "/fields/regions.v1.bin")"
  "$(join_url "$CDN_BASE_URL" "/fields/regions.v1.meta.json")"
  "$(join_url "$CDN_BASE_URL" "/fields/region_groups.v1.bin")"
  "$(join_url "$CDN_BASE_URL" "/fields/region_groups.v1.meta.json")"
  "$(join_url "$CDN_BASE_URL" "/fields/zone_mask.v1.bin")"
  "$(join_url "$CDN_BASE_URL" "/fields/zone_mask.v1.meta.json")"
  "$(join_url "$CDN_BASE_URL" "/images/tiles/minimap_visual/v1/tileset.json")"
)

immutable_cache_urls=(
  "$current_manifest_url"
  "$(join_url "$CDN_BASE_URL" "/map/${current_module}")"
  "$(join_url "$CDN_BASE_URL" "/map/${current_wasm}")"
  "$(join_url "$CDN_BASE_URL" "/fields/regions.v1.bin")"
  "$(join_url "$CDN_BASE_URL" "/fields/regions.v1.meta.json")"
  "$(join_url "$CDN_BASE_URL" "/fields/region_groups.v1.bin")"
  "$(join_url "$CDN_BASE_URL" "/fields/region_groups.v1.meta.json")"
  "$(join_url "$CDN_BASE_URL" "/fields/zone_mask.v1.bin")"
  "$(join_url "$CDN_BASE_URL" "/fields/zone_mask.v1.meta.json")"
  "$(join_url "$CDN_BASE_URL" "/images/tiles/minimap_visual/v1/tileset.json")"
)

short_cache_urls=(
  "$(join_url "$CDN_BASE_URL" "/map/map-host.js")"
  "$(join_url "$CDN_BASE_URL" "/map/ui/fishystuff.css")"
)

cors_required_urls=(
  "$current_manifest_url"
  "$stable_manifest_url"
  "$(join_url "$CDN_BASE_URL" "/map/${current_module}")"
  "$(join_url "$CDN_BASE_URL" "/map/${current_wasm}")"
  "$(join_url "$CDN_BASE_URL" "/fields/regions.v1.bin")"
  "$(join_url "$CDN_BASE_URL" "/fields/regions.v1.meta.json")"
  "$(join_url "$CDN_BASE_URL" "/fields/region_groups.v1.bin")"
  "$(join_url "$CDN_BASE_URL" "/fields/region_groups.v1.meta.json")"
  "$(join_url "$CDN_BASE_URL" "/fields/zone_mask.v1.bin")"
  "$(join_url "$CDN_BASE_URL" "/fields/zone_mask.v1.meta.json")"
  "$(join_url "$CDN_BASE_URL" "/images/tiles/minimap_visual/v1/tileset.json")"
)

for url in "${required_urls[@]}"; do
  check_url_exists "$url"
done

check_url_cache_control "$stable_manifest_url" "no-store"

for url in "${immutable_cache_urls[@]}"; do
  check_url_cache_control "$url" "max-age=31536000" "immutable"
done

for url in "${short_cache_urls[@]}"; do
  check_url_cache_control "$url" "max-age=3600"
done

for url in "${cors_required_urls[@]}"; do
  check_url_cors "$url"
done

echo "CDN map runtime assets are reachable, CORS-readable, and cache headers match expectations." >&2
