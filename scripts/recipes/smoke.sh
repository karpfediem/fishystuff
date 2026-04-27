#!/usr/bin/env bash
set -euo pipefail

SCRIPT_PATH="$(readlink -f "${BASH_SOURCE[0]}")"
SCRIPT_DIR="$(cd "$(dirname "$SCRIPT_PATH")" && pwd)"
source "${SCRIPT_DIR}/lib/common.sh"

cd "$RECIPE_REPO_ROOT"

deployment="${1-}"
require_value "$deployment" "usage: smoke.sh <deployment>"
deployment="$(canonical_deployment_name "$deployment")"

case "$deployment" in
  local)
    api_base_url="$(deployment_public_base_url "$deployment" "api")"
    site_base_url="$(deployment_public_base_url "$deployment" "site")"
    ;;
  *)
    profile="$(deployment_secretspec_profile "$deployment")"
    exec_with_secretspec_profile_if_needed "$profile" bash "$SCRIPT_PATH" "$deployment"
    api_base_url="$(deployment_public_base_url "$deployment" "api")"
    site_base_url="$(deployment_public_base_url "$deployment" "site")"
    ;;
esac

api_base_url="${api_base_url%/}"
site_base_url="${site_base_url%/}"
timeout_secs="${FISHYSTUFF_SMOKE_TIMEOUT_SECS:-900}"
interval_secs="${FISHYSTUFF_SMOKE_INTERVAL_SECS:-5}"
started_at="$(date +%s)"

probe() {
  local name="$1"
  local url="$2"
  local expected="${3:-200}"
  local tmp_body
  local http_code

  tmp_body="$(mktemp /tmp/fishystuff-smoke.XXXXXX)"
  http_code="$(
    curl -sS \
      --max-time 60 \
      --connect-timeout 10 \
      --output "$tmp_body" \
      --write-out '%{http_code}' \
      "$url" 2>"$tmp_body.err" || true
  )"
  if [[ "$http_code" == "$expected" ]]; then
    rm -f "$tmp_body" "$tmp_body.err"
    return 0
  fi

  printf '[smoke] %s failed: expected HTTP %s, got %s\n' "$name" "$expected" "${http_code:-curl-error}" >&2
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
  probe "site homepage" "$site_base_url/"
  probe "api healthz" "$api_base_url/healthz"
  probe "api readyz" "$api_base_url/readyz"
  probe "api meta" "$api_base_url/api/v1/meta"
  probe "calculator catalog" "$api_base_url/api/v1/calculator?lang=en"
  probe "calculator datastar init" "$api_base_url/api/v1/calculator/datastar/init?lang=en&locale=en-US"
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
