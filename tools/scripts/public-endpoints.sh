#!/usr/bin/env bash

fishystuff_trim_trailing_slash() {
  local value="${1:-}"
  while [ -n "$value" ] && [ "${value%/}" != "$value" ]; do
    value="${value%/}"
  done
  printf '%s' "$value"
}

fishystuff_trim_ascii_whitespace() {
  printf '%s' "$1" | sed -e 's/^[[:space:]]*//' -e 's/[[:space:]]*$//'
}

fishystuff_normalize_base_url() {
  local normalized
  local rest
  normalized="$(fishystuff_trim_ascii_whitespace "${1:-}")"
  normalized="$(fishystuff_trim_trailing_slash "$normalized")"
  if [ -z "$normalized" ]; then
    return 1
  fi
  case "$normalized" in
    http://*|https://*) ;;
    *)
      return 1
      ;;
  esac
  rest="${normalized#*://}"
  case "$rest" in
    ""|*/*|*\?*|*#*)
      return 1
      ;;
  esac
  printf '%s\n' "$normalized"
}

fishystuff_derive_sibling_base_url() {
  local base_url
  local scheme
  local host
  local hostname
  local subdomain

  base_url="$(fishystuff_normalize_base_url "${1:-}")" || return 1
  subdomain="$(fishystuff_trim_ascii_whitespace "${2:-}")"
  subdomain="${subdomain#.}"
  subdomain="${subdomain%.}"
  if [ -z "$subdomain" ]; then
    return 1
  fi
  scheme="${base_url%%://*}"
  host="${base_url#*://}"
  hostname="${host%%:*}"
  if [ "$hostname" = "localhost" ] || [ "$hostname" = "127.0.0.1" ]; then
    return 1
  fi
  printf '%s://%s.%s\n' "$scheme" "$subdomain" "$host"
}

fishystuff_resolve_public_base_urls() {
  FISHYSTUFF_RESOLVED_PUBLIC_SITE_BASE_URL="$(
    fishystuff_normalize_base_url "${FISHYSTUFF_PUBLIC_SITE_BASE_URL:-}" \
      || printf '%s\n' "https://fishystuff.fish"
  )"
  FISHYSTUFF_RESOLVED_PUBLIC_API_BASE_URL="$(
    fishystuff_normalize_base_url "${FISHYSTUFF_PUBLIC_API_BASE_URL:-}" \
      || fishystuff_derive_sibling_base_url "$FISHYSTUFF_RESOLVED_PUBLIC_SITE_BASE_URL" "api" \
      || printf '%s\n' "https://api.fishystuff.fish"
  )"
  FISHYSTUFF_RESOLVED_PUBLIC_CDN_BASE_URL="$(
    fishystuff_normalize_base_url "${FISHYSTUFF_PUBLIC_CDN_BASE_URL:-}" \
      || fishystuff_derive_sibling_base_url "$FISHYSTUFF_RESOLVED_PUBLIC_SITE_BASE_URL" "cdn" \
      || printf '%s\n' "https://cdn.fishystuff.fish"
  )"
  FISHYSTUFF_RESOLVED_PUBLIC_OTEL_BASE_URL="$(
    fishystuff_normalize_base_url "${FISHYSTUFF_PUBLIC_OTEL_BASE_URL:-}" \
      || fishystuff_derive_sibling_base_url "$FISHYSTUFF_RESOLVED_PUBLIC_SITE_BASE_URL" "otel" \
      || printf '%s\n' "https://otel.fishystuff.fish"
  )"
  FISHYSTUFF_RESOLVED_PUBLIC_OTEL_TRACES_ENDPOINT="${FISHYSTUFF_PUBLIC_OTEL_TRACES_ENDPOINT:-$FISHYSTUFF_RESOLVED_PUBLIC_OTEL_BASE_URL/v1/traces}"
}
