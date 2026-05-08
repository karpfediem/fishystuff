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
require_command jq

if [[ ! -d "$bundle" ]]; then
  echo "production GitOps edge handoff bundle does not exist: ${bundle}" >&2
  exit 2
fi

caddy_bin="${bundle}/artifacts/exe/main"
caddyfile="${bundle}/artifacts/config/base"
systemd_unit="${bundle}/artifacts/systemd/unit"
bundle_json="${bundle}/bundle.json"

if [[ ! -x "$caddy_bin" ]]; then
  echo "production GitOps edge handoff Caddy executable is missing or not executable: ${caddy_bin}" >&2
  exit 2
fi
if [[ ! -f "$caddyfile" ]]; then
  echo "production GitOps edge handoff Caddyfile is missing: ${caddyfile}" >&2
  exit 2
fi
if [[ ! -f "$systemd_unit" ]]; then
  echo "production GitOps edge handoff systemd unit is missing: ${systemd_unit}" >&2
  exit 2
fi
if [[ ! -f "$bundle_json" ]]; then
  echo "production GitOps edge handoff bundle metadata is missing: ${bundle_json}" >&2
  exit 2
fi

caddy_bin_store="$(jq -er '.artifacts."exe/main".storePath | select(type == "string" and length > 0)' "$bundle_json")"
caddyfile_store="$(jq -er '.artifacts."config/base".storePath | select(type == "string" and length > 0)' "$bundle_json")"
systemd_unit_store="$(jq -er '.artifacts."systemd/unit".storePath | select(type == "string" and length > 0)' "$bundle_json")"
caddy_bin_real="$(readlink -f "$caddy_bin")"
caddyfile_real="$(readlink -f "$caddyfile")"
systemd_unit_real="$(readlink -f "$systemd_unit")"

require_same_path() {
  local label="$1"
  local actual="$2"
  local expected="$3"
  if [[ "$actual" != "$expected" ]]; then
    echo "production GitOps edge handoff ${label} path mismatch" >&2
    echo "actual:   ${actual}" >&2
    echo "expected: ${expected}" >&2
    exit 2
  fi
}

require_same_path "Caddy executable artifact" "$caddy_bin_real" "$caddy_bin_store"
require_same_path "Caddyfile artifact" "$caddyfile_real" "$caddyfile_store"
require_same_path "systemd unit artifact" "$systemd_unit_real" "$systemd_unit_store"

require_bundle_metadata() {
  local label="$1"
  local filter="$2"
  if ! jq -e \
    --arg caddy_bin_store "$caddy_bin_store" \
    --arg caddyfile_store "$caddyfile_store" \
    --arg systemd_unit_store "$systemd_unit_store" \
    "$filter" \
    "$bundle_json" >/dev/null; then
    echo "production GitOps edge handoff bundle metadata is missing ${label}" >&2
    exit 2
  fi
}

require_unit_line() {
  local label="$1"
  local needle="$2"
  if ! grep -Fx "$needle" "$systemd_unit" >/dev/null; then
    echo "production GitOps edge handoff systemd unit is missing ${label}: ${needle}" >&2
    exit 2
  fi
}

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

require_unit_line "dynamic user" "DynamicUser=true"
require_unit_line "Caddy ExecStart" "ExecStart=${caddy_bin_store} run --config ${caddyfile_store} --adapter caddyfile"
require_unit_line "Caddy ExecReload" "ExecReload=${caddy_bin_store} reload --config ${caddyfile_store} --adapter caddyfile --address 127.0.0.1:2019 --force"
require_unit_line "TLS fullchain credential" "LoadCredential=fullchain.pem:/run/fishystuff/edge/tls/fullchain.pem"
require_unit_line "TLS private key credential" "LoadCredential=privkey.pem:/run/fishystuff/edge/tls/privkey.pem"
require_unit_line "bind service capability" "AmbientCapabilities=CAP_NET_BIND_SERVICE"
require_unit_line "strict system protection" "ProtectSystem=strict"

require_bundle_metadata "fishystuff-edge service ID" '.id == "fishystuff-edge"'
require_bundle_metadata "GitOps site required path" '.activation.requiredPaths | index("/var/lib/fishystuff/gitops/served/production/site") != null'
require_bundle_metadata "GitOps CDN required path" '.activation.requiredPaths | index("/var/lib/fishystuff/gitops/served/production/cdn") != null'
require_bundle_metadata "no activation writable paths" '((.activation.writablePaths // []) | length) == 0 and ((.activation.writable_paths // []) | length) == 0'
require_bundle_metadata "TLS runtime directory" '.activation.directories[]? | select(.path == "/run/fishystuff/edge/tls" and .create == true)'
require_bundle_metadata "systemd unit install" '.backends.systemd.daemon_reload == true and (.backends.systemd.units[]? | select(.name == "fishystuff-edge.service" and .install_path == "/etc/systemd/system/fishystuff-edge.service" and .state == "running" and .startup == "enabled"))'
require_bundle_metadata "systemd unit artifact" '.artifacts."systemd/unit".storePath == $systemd_unit_store and .artifacts."systemd/unit".destination == "fishystuff-edge.service"'
require_bundle_metadata "Caddy executable artifact" '.artifacts."exe/main".storePath == $caddy_bin_store and .artifacts."exe/main".executable == true'
require_bundle_metadata "Caddyfile artifact" '.artifacts."config/base".storePath == $caddyfile_store and .artifacts."config/base".destination == "Caddyfile"'
require_bundle_metadata "supervision run argv" '.supervision.argv == [$caddy_bin_store, "run", "--config", $caddyfile_store, "--adapter", "caddyfile"]'
require_bundle_metadata "supervision reload argv" '.supervision.reload.mode == "command" and .supervision.reload.argv == [$caddy_bin_store, "reload", "--config", $caddyfile_store, "--adapter", "caddyfile", "--address", "127.0.0.1:2019", "--force"]'
require_bundle_metadata "TLS fullchain runtime overlay" '.runtimeOverlays[]? | select(.targetPath == "/run/fishystuff/edge/tls/fullchain.pem" and .required == true and .secret == false and .onChange == "restart")'
require_bundle_metadata "TLS private key runtime overlay" '.runtimeOverlays[]? | select(.targetPath == "/run/fishystuff/edge/tls/privkey.pem" and .required == true and .secret == true and .onChange == "restart")'

printf 'gitops_edge_handoff_bundle_ok=%s\n' "$bundle"
printf 'gitops_edge_handoff_caddyfile=%s\n' "$caddyfile"
printf 'gitops_edge_handoff_executable=%s\n' "$caddy_bin"
printf 'gitops_edge_handoff_systemd_unit=%s\n' "$systemd_unit"
printf 'gitops_edge_handoff_caddyfile_store=%s\n' "$caddyfile_store"
printf 'gitops_edge_handoff_executable_store=%s\n' "$caddy_bin_store"
printf 'gitops_edge_handoff_systemd_unit_store=%s\n' "$systemd_unit_store"
printf 'gitops_edge_handoff_site_root=%s\n' "/var/lib/fishystuff/gitops/served/production/site"
printf 'gitops_edge_handoff_cdn_root=%s\n' "/var/lib/fishystuff/gitops/served/production/cdn"
printf 'gitops_edge_handoff_api_upstream=%s\n' "127.0.0.1:18092"
