#!/usr/bin/env bash
set -euo pipefail

SCRIPT_PATH="$(readlink -f "${BASH_SOURCE[0]}")"
SCRIPT_DIR="$(cd "$(dirname "$SCRIPT_PATH")" && pwd)"
source "${SCRIPT_DIR}/lib/common.sh"

cd "$RECIPE_REPO_ROOT"

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
    profile="$(deployment_secretspec_profile "$deployment")"
    exec_with_secretspec_profile_if_needed "$profile" bash "$SCRIPT_PATH" "$deployment" "$origin_ipv4"
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
  local tmp_body
  local http_code
  local resolve

  tmp_body="$(mktemp /tmp/fishystuff-origin-smoke.XXXXXX)"
  resolve="$(resolve_for_url "$url")"
  http_code="$(
    curl -sS \
      --max-time 60 \
      --connect-timeout 10 \
      --resolve "$resolve" \
      --output "$tmp_body" \
      --write-out '%{http_code}' \
      "$url" 2>"$tmp_body.err" || true
  )"
  if [[ "$http_code" == "$expected" ]]; then
    rm -f "$tmp_body" "$tmp_body.err"
    return 0
  fi

  printf '[origin-smoke] %s failed through %s: expected HTTP %s, got %s\n' "$name" "$origin_ipv4" "$expected" "${http_code:-curl-error}" >&2
  if [[ -s "$tmp_body.err" ]]; then
    sed -n '1,20p' "$tmp_body.err" >&2
  fi
  if [[ -s "$tmp_body" ]]; then
    head -c 2000 "$tmp_body" >&2
    printf '\n' >&2
  fi
  rm -f "$tmp_body" "$tmp_body.err"
  return 1
}

run_once() {
  local failed=0
  probe "site homepage" "$site_base_url/" || failed=1
  probe "api healthz" "$api_base_url/healthz" || failed=1
  probe "api readyz" "$api_base_url/readyz" || failed=1
  probe "api meta" "$api_base_url/api/v1/meta" || failed=1
  probe "calculator catalog" "$api_base_url/api/v1/calculator?lang=en" || failed=1
  probe "calculator datastar init" "$api_base_url/api/v1/calculator/datastar/init?lang=en&locale=en-US" || failed=1
  probe "cdn runtime manifest" "$cdn_base_url/map/runtime-manifest.json" || failed=1
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
