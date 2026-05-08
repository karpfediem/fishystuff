#!/usr/bin/env bash
set -euo pipefail

SCRIPT_PATH="$(readlink -f "${BASH_SOURCE[0]}")"
SCRIPT_DIR="$(cd "$(dirname "$SCRIPT_PATH")" && pwd)"
source "${SCRIPT_DIR}/lib/common.sh"

bundle="$(normalize_named_arg bundle "${1-auto}")"

cd "$RECIPE_REPO_ROOT"

require_command() {
  local command_name="$1"
  if ! command -v "$command_name" >/dev/null 2>&1; then
    echo "missing required command: ${command_name}" >&2
    exit 127
  fi
}

if [[ "$bundle" == "auto" ]]; then
  require_command nix
  bundle="$(nix build --no-link --print-out-paths .#edge-service-bundle-production-gitops-handoff | tail -n 1)"
elif [[ "$bundle" != /* ]]; then
  bundle="${RECIPE_REPO_ROOT}/${bundle}"
fi

if [[ ! -d "$bundle" ]]; then
  echo "production GitOps edge handoff bundle does not exist: ${bundle}" >&2
  exit 2
fi

caddy_bin="${bundle}/artifacts/exe/main"
caddyfile="${bundle}/artifacts/config/base"

if [[ ! -x "$caddy_bin" ]]; then
  echo "production GitOps edge handoff Caddy executable is missing or not executable: ${caddy_bin}" >&2
  exit 2
fi
if [[ ! -f "$caddyfile" ]]; then
  echo "production GitOps edge handoff Caddyfile is missing: ${caddyfile}" >&2
  exit 2
fi

require_caddy_line() {
  local label="$1"
  local needle="$2"
  if ! grep -F "$needle" "$caddyfile" >/dev/null; then
    echo "production GitOps edge handoff Caddyfile is missing ${label}: ${needle}" >&2
    exit 2
  fi
}

reject_caddy_line() {
  local label="$1"
  local needle="$2"
  if grep -F "$needle" "$caddyfile" >/dev/null; then
    echo "production GitOps edge handoff Caddyfile must not contain ${label}: ${needle}" >&2
    exit 2
  fi
}

require_caddy_line "manual TLS mode" "auto_https off"
require_caddy_line "site vhost" "https://fishystuff.fish {"
require_caddy_line "API vhost" "https://api.fishystuff.fish {"
require_caddy_line "CDN vhost" "https://cdn.fishystuff.fish {"
require_caddy_line "telemetry vhost" "https://telemetry.fishystuff.fish {"
require_caddy_line "credential-directory TLS" 'tls {$CREDENTIALS_DIRECTORY}/fullchain.pem {$CREDENTIALS_DIRECTORY}/privkey.pem'
require_caddy_line "GitOps site root" "root * /var/lib/fishystuff/gitops/served/production/site"
require_caddy_line "GitOps CDN root" "root * /var/lib/fishystuff/gitops/served/production/cdn"
require_caddy_line "loopback candidate API upstream" "reverse_proxy 127.0.0.1:18092"
require_caddy_line "CDN runtime manifest no-store matcher" "@runtime_manifest path /map/runtime-manifest.json"
require_caddy_line "no-store cache header" 'header Cache-Control "no-store"'
require_caddy_line "immutable cache header" 'header Cache-Control "public, max-age=31536000, immutable"'

reject_caddy_line "legacy serving root" "/srv/fishystuff"
reject_caddy_line "fixed store serving root" "root * /nix/store/"
reject_caddy_line "beta hostname" "beta.fishystuff.fish"

printf 'gitops_edge_handoff_bundle_ok=%s\n' "$bundle"
printf 'gitops_edge_handoff_caddyfile=%s\n' "$caddyfile"
printf 'gitops_edge_handoff_executable=%s\n' "$caddy_bin"
printf 'gitops_edge_handoff_site_root=%s\n' "/var/lib/fishystuff/gitops/served/production/site"
printf 'gitops_edge_handoff_cdn_root=%s\n' "/var/lib/fishystuff/gitops/served/production/cdn"
printf 'gitops_edge_handoff_api_upstream=%s\n' "127.0.0.1:18092"
