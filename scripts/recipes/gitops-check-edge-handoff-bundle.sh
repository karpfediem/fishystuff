#!/usr/bin/env bash
set -euo pipefail

SCRIPT_PATH="$(readlink -f "${BASH_SOURCE[0]}")"
SCRIPT_DIR="$(cd "$(dirname "$SCRIPT_PATH")" && pwd)"
source "${SCRIPT_DIR}/lib/common.sh"

bundle="$(normalize_named_arg bundle "${1-auto}")"
environment="$(normalize_named_arg environment "${2-production}")"

cd "$RECIPE_REPO_ROOT"

require_command() {
  local command_name="$1"
  if ! command -v "$command_name" >/dev/null 2>&1; then
    echo "missing required command: ${command_name}" >&2
    exit 127
  fi
}

require_command jq
require_command openssl

case "$environment" in
  production)
    auto_package="edge-service-bundle-production-gitops-handoff"
    service_id="fishystuff-edge"
    unit_name="fishystuff-edge.service"
    site_vhost="https://fishystuff.fish {"
    api_vhost="https://api.fishystuff.fish {"
    cdn_vhost="https://cdn.fishystuff.fish {"
    telemetry_vhost="https://telemetry.fishystuff.fish {"
    site_root="/var/lib/fishystuff/gitops/served/production/site"
    cdn_root="/var/lib/fishystuff/gitops/served/production/cdn"
    api_upstream="127.0.0.1:18092"
    tls_dir="/run/fishystuff/edge/tls"
    tls_fullchain_path="${tls_dir}/fullchain.pem"
    tls_privkey_path="${tls_dir}/privkey.pem"
    admin_address="127.0.0.1:2019"
    placeholder_cn="fishystuff.fish"
    placeholder_san="DNS:fishystuff.fish,DNS:api.fishystuff.fish,DNS:cdn.fishystuff.fish,DNS:telemetry.fishystuff.fish"
    service_dependency_line="Wants=network-online.target fishystuff-api.service fishystuff-vector.service"
    forbidden_caddy_fragments=(
      "beta.fishystuff.fish"
      "/srv/fishystuff"
      "root * /nix/store/"
    )
    forbidden_unit_fragments=(
      "beta.fishystuff.fish"
      "/srv/fishystuff"
      "/var/lib/fishystuff/gitops/served"
    )
    ;;
  beta)
    auto_package="edge-service-bundle-beta-gitops-handoff"
    service_id="fishystuff-beta-edge"
    unit_name="fishystuff-beta-edge.service"
    site_vhost="https://beta.fishystuff.fish {"
    api_vhost="https://api.beta.fishystuff.fish {"
    cdn_vhost="https://cdn.beta.fishystuff.fish {"
    telemetry_vhost="https://telemetry.beta.fishystuff.fish {"
    site_root="/var/lib/fishystuff/gitops-beta/served/beta/site"
    cdn_root="/var/lib/fishystuff/gitops-beta/served/beta/cdn"
    api_upstream="127.0.0.1:18192"
    tls_dir="/run/fishystuff/beta-edge/tls"
    tls_fullchain_path="${tls_dir}/fullchain.pem"
    tls_privkey_path="${tls_dir}/privkey.pem"
    admin_address="127.0.0.1:2119"
    placeholder_cn="beta.fishystuff.fish"
    placeholder_san="DNS:beta.fishystuff.fish,DNS:api.beta.fishystuff.fish,DNS:cdn.beta.fishystuff.fish,DNS:telemetry.beta.fishystuff.fish"
    service_dependency_line="Wants=network-online.target fishystuff-beta-api.service fishystuff-beta-vector.service"
    forbidden_caddy_fragments=(
      "https://fishystuff.fish"
      "https://api.fishystuff.fish"
      "https://cdn.fishystuff.fish"
      "https://telemetry.fishystuff.fish"
      "/var/lib/fishystuff/gitops/served/production"
      "/srv/fishystuff"
      "root * /nix/store/"
    )
    forbidden_unit_fragments=(
      "Wants=network-online.target fishystuff-api.service fishystuff-vector.service"
      "LoadCredential=fullchain.pem:/run/fishystuff/edge/tls/fullchain.pem"
      "LoadCredential=privkey.pem:/run/fishystuff/edge/tls/privkey.pem"
      "/var/lib/fishystuff/gitops/served/production"
      "/srv/fishystuff"
    )
    ;;
  *)
    echo "unsupported GitOps edge handoff environment: ${environment}" >&2
    exit 2
    ;;
esac

if [[ "$bundle" == "auto" ]]; then
  require_command nix
  bundle="$(nix build --no-link --print-out-paths ".#${auto_package}" | tail -n 1)"
elif [[ "$bundle" != /* ]]; then
  bundle="${RECIPE_REPO_ROOT}/${bundle}"
fi

if [[ ! -d "$bundle" ]]; then
  echo "${environment} GitOps edge handoff bundle does not exist: ${bundle}" >&2
  exit 2
fi

caddy_bin="${bundle}/artifacts/exe/main"
caddyfile="${bundle}/artifacts/config/base"
systemd_unit="${bundle}/artifacts/systemd/unit"
bundle_json="${bundle}/bundle.json"

if [[ ! -x "$caddy_bin" ]]; then
  echo "${environment} GitOps edge handoff Caddy executable is missing or not executable: ${caddy_bin}" >&2
  exit 2
fi
if [[ ! -f "$caddyfile" ]]; then
  echo "${environment} GitOps edge handoff Caddyfile is missing: ${caddyfile}" >&2
  exit 2
fi
if [[ ! -f "$systemd_unit" ]]; then
  echo "${environment} GitOps edge handoff systemd unit is missing: ${systemd_unit}" >&2
  exit 2
fi
if [[ ! -f "$bundle_json" ]]; then
  echo "${environment} GitOps edge handoff bundle metadata is missing: ${bundle_json}" >&2
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
    echo "${environment} GitOps edge handoff ${label} path mismatch" >&2
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
    --arg service_id "$service_id" \
    --arg unit_name "$unit_name" \
    --arg site_root "$site_root" \
    --arg cdn_root "$cdn_root" \
    --arg tls_dir "$tls_dir" \
    --arg tls_fullchain_path "$tls_fullchain_path" \
    --arg tls_privkey_path "$tls_privkey_path" \
    --arg admin_address "$admin_address" \
    "$filter" \
    "$bundle_json" >/dev/null; then
    echo "${environment} GitOps edge handoff bundle metadata is missing ${label}" >&2
    exit 2
  fi
}

require_unit_line() {
  local label="$1"
  local needle="$2"
  if ! grep -Fx "$needle" "$systemd_unit" >/dev/null; then
    echo "${environment} GitOps edge handoff systemd unit is missing ${label}: ${needle}" >&2
    exit 2
  fi
}

require_caddy_line() {
  local label="$1"
  local needle="$2"
  if ! grep -F "$needle" "$caddyfile" >/dev/null; then
    echo "${environment} GitOps edge handoff Caddyfile is missing ${label}: ${needle}" >&2
    exit 2
  fi
}

reject_caddy_line() {
  local label="$1"
  local needle="$2"
  if grep -F "$needle" "$caddyfile" >/dev/null; then
    echo "${environment} GitOps edge handoff Caddyfile must not contain ${label}: ${needle}" >&2
    exit 2
  fi
}

reject_unit_line() {
  local label="$1"
  local needle="$2"
  if grep -F "$needle" "$systemd_unit" >/dev/null; then
    echo "${environment} GitOps edge handoff systemd unit must not contain ${label}: ${needle}" >&2
    exit 2
  fi
}

validate_caddyfile_with_placeholder_tls() {
  local tmp_dir=""
  local credentials_dir=""
  local caddy_validate_log=""

  tmp_dir="$(mktemp -d)"
  credentials_dir="${tmp_dir}/credentials"
  caddy_validate_log="${tmp_dir}/caddy-validate.log"
  mkdir -p "$credentials_dir" "${tmp_dir}/home" "${tmp_dir}/xdg-config" "${tmp_dir}/xdg-data"
  if ! openssl req \
    -x509 \
    -newkey rsa:2048 \
    -nodes \
    -keyout "${credentials_dir}/privkey.pem" \
    -out "${credentials_dir}/fullchain.pem" \
    -days 1 \
    -subj "/CN=${placeholder_cn}" \
    -addext "subjectAltName=${placeholder_san}" \
    >"${tmp_dir}/openssl.log" 2>&1; then
    echo "${environment} GitOps edge handoff placeholder TLS generation failed" >&2
    cat "${tmp_dir}/openssl.log" >&2
    rm -rf "$tmp_dir"
    exit 2
  fi

  if ! env \
    CREDENTIALS_DIRECTORY="$credentials_dir" \
    HOME="${tmp_dir}/home" \
    XDG_CONFIG_HOME="${tmp_dir}/xdg-config" \
    XDG_DATA_HOME="${tmp_dir}/xdg-data" \
    "$caddy_bin_store" validate --config "$caddyfile_store" --adapter caddyfile \
    >"$caddy_validate_log" 2>&1; then
    echo "${environment} GitOps edge handoff Caddyfile failed caddy validate" >&2
    cat "$caddy_validate_log" >&2
    rm -rf "$tmp_dir"
    exit 2
  fi
  rm -rf "$tmp_dir"
}

require_caddy_line "manual TLS mode" "auto_https off"
require_caddy_line "site vhost" "$site_vhost"
require_caddy_line "API vhost" "$api_vhost"
require_caddy_line "CDN vhost" "$cdn_vhost"
require_caddy_line "telemetry vhost" "$telemetry_vhost"
require_caddy_line "credential-directory TLS" 'tls {$CREDENTIALS_DIRECTORY}/fullchain.pem {$CREDENTIALS_DIRECTORY}/privkey.pem'
require_caddy_line "GitOps site root" "root * ${site_root}"
require_caddy_line "GitOps CDN root" "root * ${cdn_root}"
require_caddy_line "loopback candidate API upstream" "reverse_proxy ${api_upstream}"
require_caddy_line "CDN runtime manifest no-store matcher" "@runtime_manifest path /map/runtime-manifest.json"
require_caddy_line "no-store cache header" 'header Cache-Control "no-store"'
require_caddy_line "immutable cache header" 'header Cache-Control "public, max-age=31536000, immutable"'

for forbidden_fragment in "${forbidden_caddy_fragments[@]}"; do
  reject_caddy_line "forbidden ${environment} Caddy fragment" "$forbidden_fragment"
done

require_unit_line "dynamic user" "DynamicUser=true"
require_unit_line "service dependency units" "$service_dependency_line"
require_unit_line "Caddy ExecStart" "ExecStart=${caddy_bin_store} run --config ${caddyfile_store} --adapter caddyfile"
require_unit_line "Caddy ExecReload" "ExecReload=${caddy_bin_store} reload --config ${caddyfile_store} --adapter caddyfile --address ${admin_address} --force"
require_unit_line "TLS fullchain credential" "LoadCredential=fullchain.pem:${tls_fullchain_path}"
require_unit_line "TLS private key credential" "LoadCredential=privkey.pem:${tls_privkey_path}"
require_unit_line "bind service capability" "AmbientCapabilities=CAP_NET_BIND_SERVICE"
require_unit_line "strict system protection" "ProtectSystem=strict"

for forbidden_fragment in "${forbidden_unit_fragments[@]}"; do
  reject_unit_line "forbidden ${environment} unit fragment" "$forbidden_fragment"
done

require_bundle_metadata "edge service ID" '.id == $service_id'
require_bundle_metadata "GitOps site required path" '.activation.requiredPaths | index($site_root) != null'
require_bundle_metadata "GitOps CDN required path" '.activation.requiredPaths | index($cdn_root) != null'
require_bundle_metadata "no activation writable paths" '((.activation.writablePaths // []) | length) == 0 and ((.activation.writable_paths // []) | length) == 0'
require_bundle_metadata "TLS runtime directory" '.activation.directories[]? | select(.path == $tls_dir and .create == true)'
require_bundle_metadata "systemd unit install" '.backends.systemd.daemon_reload == true and (.backends.systemd.units[]? | select(.name == $unit_name and .install_path == ("/etc/systemd/system/" + $unit_name) and .state == "running" and .startup == "enabled"))'
require_bundle_metadata "systemd unit artifact" '.artifacts."systemd/unit".storePath == $systemd_unit_store and .artifacts."systemd/unit".destination == $unit_name'
require_bundle_metadata "Caddy executable artifact" '.artifacts."exe/main".storePath == $caddy_bin_store and .artifacts."exe/main".executable == true'
require_bundle_metadata "Caddyfile artifact" '.artifacts."config/base".storePath == $caddyfile_store and .artifacts."config/base".destination == "Caddyfile"'
require_bundle_metadata "supervision run argv" '.supervision.argv == [$caddy_bin_store, "run", "--config", $caddyfile_store, "--adapter", "caddyfile"]'
require_bundle_metadata "supervision reload argv" '.supervision.reload.mode == "command" and .supervision.reload.argv == [$caddy_bin_store, "reload", "--config", $caddyfile_store, "--adapter", "caddyfile", "--address", $admin_address, "--force"]'
require_bundle_metadata "TLS fullchain runtime overlay" '.runtimeOverlays[]? | select(.targetPath == $tls_fullchain_path and .required == true and .secret == false and .onChange == "restart")'
require_bundle_metadata "TLS private key runtime overlay" '.runtimeOverlays[]? | select(.targetPath == $tls_privkey_path and .required == true and .secret == true and .onChange == "restart")'

validate_caddyfile_with_placeholder_tls

printf 'gitops_edge_handoff_bundle_ok=%s\n' "$bundle"
printf 'gitops_edge_handoff_caddyfile=%s\n' "$caddyfile"
printf 'gitops_edge_handoff_executable=%s\n' "$caddy_bin"
printf 'gitops_edge_handoff_systemd_unit=%s\n' "$systemd_unit"
printf 'gitops_edge_handoff_caddyfile_store=%s\n' "$caddyfile_store"
printf 'gitops_edge_handoff_executable_store=%s\n' "$caddy_bin_store"
printf 'gitops_edge_handoff_systemd_unit_store=%s\n' "$systemd_unit_store"
printf 'gitops_edge_handoff_caddy_validate=%s\n' "true"
printf 'gitops_edge_handoff_environment=%s\n' "$environment"
printf 'gitops_edge_handoff_service_id=%s\n' "$service_id"
printf 'gitops_edge_handoff_unit_name=%s\n' "$unit_name"
printf 'gitops_edge_handoff_site_root=%s\n' "$site_root"
printf 'gitops_edge_handoff_cdn_root=%s\n' "$cdn_root"
printf 'gitops_edge_handoff_api_upstream=%s\n' "$api_upstream"
printf 'gitops_edge_handoff_tls_dir=%s\n' "$tls_dir"
