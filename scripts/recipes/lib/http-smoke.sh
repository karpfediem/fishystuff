#!/usr/bin/env bash

RECIPE_SMOKE_TEMP_FILES=()
RECIPE_SMOKE_LOG_PREFIX="${RECIPE_SMOKE_LOG_PREFIX:-smoke}"
SMOKE_LAST_BODY=""
SMOKE_LAST_HEADERS=""
SMOKE_LAST_ERROR=""
SMOKE_LAST_HTTP_CODE=""
SMOKE_LAST_RUNTIME_MAP_ASSET_CACHE_KEY=""

smoke_cleanup_temp_files() {
  if (( ${#RECIPE_SMOKE_TEMP_FILES[@]} > 0 )); then
    rm -f "${RECIPE_SMOKE_TEMP_FILES[@]}"
  fi
}

smoke_new_temp_file() {
  local path
  path="$(mktemp /tmp/fishystuff-smoke.XXXXXX)"
  RECIPE_SMOKE_TEMP_FILES+=("$path")
  printf '%s' "$path"
}

smoke_fetch() {
  local name="$1"
  local url="$2"
  local expected="${3:-200}"
  local resolve="${4:-}"
  local tmp_body
  local tmp_headers
  local tmp_error
  local http_code
  local -a curl_args=()

  tmp_body="$(smoke_new_temp_file)"
  tmp_headers="$(smoke_new_temp_file)"
  tmp_error="$(smoke_new_temp_file)"
  SMOKE_LAST_BODY="$tmp_body"
  SMOKE_LAST_HEADERS="$tmp_headers"
  SMOKE_LAST_ERROR="$tmp_error"
  SMOKE_LAST_HTTP_CODE=""

  curl_args=(
    curl
    -sS
    --max-time 60
    --connect-timeout 10
    --dump-header "$tmp_headers"
    --output "$tmp_body"
    --write-out '%{http_code}'
  )
  if [[ -n "$resolve" ]]; then
    curl_args+=(--resolve "$resolve")
  fi

  http_code="$("${curl_args[@]}" "$url" 2>"$tmp_error" || true)"
  SMOKE_LAST_HTTP_CODE="$http_code"
  if [[ "$http_code" == "$expected" ]]; then
    return 0
  fi

  printf '[%s] %s failed: expected HTTP %s, got %s\n' "$RECIPE_SMOKE_LOG_PREFIX" "$name" "$expected" "${http_code:-curl-error}" >&2
  if [[ -s "$tmp_error" ]]; then
    sed -n '1,20p' "$tmp_error" >&2
  fi
  if [[ -s "$tmp_body" ]]; then
    head -c 2000 "$tmp_body" >&2
    printf '\n' >&2
  fi
  return 1
}

smoke_body_contains() {
  local file="$1"
  local needle="$2"
  local label="$3"
  if grep -Fq "$needle" "$file"; then
    return 0
  fi
  printf '[%s] %s missing expected text: %s\n' "$RECIPE_SMOKE_LOG_PREFIX" "$label" "$needle" >&2
  return 1
}

smoke_body_not_contains() {
  local file="$1"
  local needle="$2"
  local label="$3"
  if ! grep -Fq "$needle" "$file"; then
    return 0
  fi
  printf '[%s] %s contains forbidden text: %s\n' "$RECIPE_SMOKE_LOG_PREFIX" "$label" "$needle" >&2
  return 1
}

smoke_headers_match() {
  local file="$1"
  local pattern="$2"
  local label="$3"
  if grep -Eiq "$pattern" "$file"; then
    return 0
  fi
  printf '[%s] %s missing expected header pattern: %s\n' "$RECIPE_SMOKE_LOG_PREFIX" "$label" "$pattern" >&2
  sed -n '1,40p' "$file" >&2
  return 1
}

smoke_trim_url_slash() {
  local value="$1"
  while [[ -n "$value" && "$value" == */ ]]; do
    value="${value%/}"
  done
  printf '%s' "$value"
}

smoke_url_origin() {
  local value="$1"
  local scheme=""
  local rest=""
  local hostport=""

  case "$value" in
    http://*) scheme="http"; rest="${value#http://}" ;;
    https://*) scheme="https"; rest="${value#https://}" ;;
    *)
      printf ''
      return
      ;;
  esac
  hostport="${rest%%/*}"
  printf '%s://%s' "$scheme" "$hostport"
}

smoke_join_url() {
  local base_url="$1"
  local relative="$2"
  local origin=""

  case "$relative" in
    http://* | https://*)
      printf '%s' "$relative"
      return
      ;;
    /*)
      origin="$(smoke_url_origin "$base_url")"
      printf '%s%s' "$origin" "$relative"
      return
      ;;
    *)
      base_url="${base_url%/}"
      printf '%s/%s' "$base_url" "$relative"
      return
      ;;
  esac
}

smoke_opposite_public_hosts_absent() {
  local deployment="$1"
  local file="$2"
  local label="$3"
  case "$deployment" in
    beta)
      smoke_body_not_contains "$file" "https://fishystuff.fish" "$label" || return 1
      smoke_body_not_contains "$file" "https://api.fishystuff.fish" "$label" || return 1
      smoke_body_not_contains "$file" "https://cdn.fishystuff.fish" "$label" || return 1
      smoke_body_not_contains "$file" "https://telemetry.fishystuff.fish" "$label" || return 1
      ;;
    production)
      smoke_body_not_contains "$file" "beta.fishystuff.fish" "$label" || return 1
      ;;
  esac
}

smoke_assert_site_html_contract() {
  local deployment="$1"
  local file="$2"

  smoke_body_contains "$file" "data-fishystuff-generated-csp" "site HTML CSP" || return 1
  smoke_body_contains "$file" "integrity=\"sha384-" "site HTML SRI" || return 1
  case "$deployment" in
    beta | production)
      smoke_body_not_contains "$file" "http://127.0.0.1" "site HTML public CSP" || return 1
      smoke_body_not_contains "$file" "http://localhost" "site HTML public CSP" || return 1
      smoke_body_not_contains "$file" "http://*.localhost" "site HTML public CSP" || return 1
      ;;
  esac
}

smoke_assert_runtime_config_contract() {
  local deployment="$1"
  local file="$2"
  local site_base_url
  local api_base_url
  local cdn_base_url
  local telemetry_base_url

  site_base_url="$(smoke_trim_url_slash "$(deployment_public_base_url "$deployment" site)")"
  api_base_url="$(smoke_trim_url_slash "$(deployment_public_base_url "$deployment" api)")"
  cdn_base_url="$(smoke_trim_url_slash "$(deployment_public_base_url "$deployment" cdn)")"
  telemetry_base_url="$(smoke_trim_url_slash "$(deployment_public_base_url "$deployment" telemetry)")"

  smoke_body_contains "$file" "window.__fishystuffRuntimeConfig" "runtime config" || return 1
  smoke_body_contains "$file" "\"siteBaseUrl\": \"$site_base_url\"" "runtime config site URL" || return 1
  smoke_body_contains "$file" "\"apiBaseUrl\": \"$api_base_url\"" "runtime config API URL" || return 1
  smoke_body_contains "$file" "\"cdnBaseUrl\": \"$cdn_base_url\"" "runtime config CDN URL" || return 1
  smoke_body_contains "$file" "\"exporterEndpoint\": \"$telemetry_base_url/v1/traces\"" "runtime config telemetry URL" || return 1
  smoke_body_contains "$file" "\"mapAssetCacheKey\": \"" "runtime config map cache key" || return 1
  smoke_opposite_public_hosts_absent "$deployment" "$file" "runtime config" || return 1
}

smoke_runtime_config_map_asset_cache_key() {
  local file="$1"
  node "$RECIPE_REPO_ROOT/tools/scripts/print_runtime_map_asset_cache_key.mjs" --allow-empty "$file"
}

smoke_assert_asset_manifest_contract() {
  local file="$1"

  jq -e '
    .version == 1
    and .integrityAlgorithm == "sha384"
    and (.assets | type == "object")
    and (.assets | length > 0)
    and ([.assets[] | select((.kind == "script" or .kind == "stylesheet") and ((.integrity // "") | startswith("sha384-")))] | length > 0)
  ' "$file" >/dev/null
}

smoke_assert_cdn_runtime_manifest_contract() {
  local file="$1"

  jq -e '
    (.module // "") | test("^fishystuff_ui_bevy\\.[0-9a-f]{16}\\.js$")
  ' "$file" >/dev/null || return 1
  jq -e '
    (.wasm // "") | test("^fishystuff_ui_bevy_bg\\.[0-9a-f]{16}\\.wasm$")
  ' "$file" >/dev/null
}
