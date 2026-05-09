#!/usr/bin/env bash
set -euo pipefail

SCRIPT_PATH="$(readlink -f "${BASH_SOURCE[0]}")"
SCRIPT_DIR="$(cd "$(dirname "$SCRIPT_PATH")" && pwd)"
source "${SCRIPT_DIR}/lib/common.sh"

target="$(normalize_named_arg target "${1:-${FISHYSTUFF_BETA_RESIDENT_TARGET:-}}")"
expected_hostname="$(normalize_named_arg expected_hostname "${2:-site-nbg1-beta}")"
fullchain_source="$(normalize_named_arg fullchain "${3:-}")"
privkey_source="$(normalize_named_arg privkey "${4:-}")"
ssh_bin="$(normalize_named_arg ssh_bin "${5:-${FISHYSTUFF_GITOPS_SSH_BIN:-ssh}}")"
scp_bin="$(normalize_named_arg scp_bin "${6:-${FISHYSTUFF_GITOPS_SCP_BIN:-scp}}")"

cd "$RECIPE_REPO_ROOT"

fail() {
  echo "$1" >&2
  exit 2
}

require_env_value() {
  local name="$1"
  local expected="$2"
  local value="${!name-}"

  if [[ "$value" != "$expected" ]]; then
    fail "gitops-beta-remote-install-edge-tls requires ${name}=${expected}"
  fi
}

require_env_nonempty() {
  local name="$1"
  local value="${!name-}"

  if [[ -z "$value" ]]; then
    fail "gitops-beta-remote-install-edge-tls requires ${name}"
  fi
}

require_command_or_executable() {
  local command_name="$1"
  local label="$2"

  if [[ "$command_name" == */* ]]; then
    if [[ ! -x "$command_name" ]]; then
      fail "${label} is not executable: ${command_name}"
    fi
    return
  fi
  if ! command -v "$command_name" >/dev/null 2>&1; then
    echo "missing required command: ${command_name}" >&2
    exit 127
  fi
}

require_beta_deploy_profile() {
  local active_profile="${FISHYSTUFF_OPERATOR_SECRETSPEC_PROFILE:-}"

  case "$active_profile" in
    beta-deploy)
      ;;
    production-deploy | prod-deploy | production)
      fail "gitops-beta-remote-install-edge-tls must not run with production SecretSpec profile active: ${active_profile}"
      ;;
    *)
      fail "gitops-beta-remote-install-edge-tls requires FISHYSTUFF_OPERATOR_SECRETSPEC_PROFILE=beta-deploy"
      ;;
  esac
}

require_safe_target() {
  local value="$1"
  local user=""
  local host=""

  if [[ -z "$value" ]]; then
    fail "target is required; use target=root@<fresh-beta-ip>"
  fi
  if [[ "$value" != *@* ]]; then
    fail "target must be user@IPv4, got: ${value}"
  fi
  user="${value%@*}"
  host="${value#*@}"
  if [[ "$user" != "root" ]]; then
    fail "fresh beta edge TLS install currently expects root SSH, got user: ${user}"
  fi
  if [[ ! "$host" =~ ^([0-9]{1,3}\.){3}[0-9]{1,3}$ ]]; then
    fail "target host must be an IPv4 address, got: ${host}"
  fi
  if [[ "$host" == "178.104.230.121" ]]; then
    fail "target points at the previous beta host; use the fresh replacement IP"
  fi
}

require_expected_hostname() {
  local value="$1"

  if [[ -z "$value" ]]; then
    fail "expected_hostname is required"
  fi
  if [[ "$value" == *=* ]]; then
    fail "expected_hostname must be a hostname, got ${value}; pass arguments in order: target expected_hostname fullchain privkey"
  fi
  if [[ ! "$value" =~ ^[a-zA-Z0-9][a-zA-Z0-9.-]*$ ]]; then
    fail "expected_hostname must be a simple hostname, got: ${value}"
  fi
}

require_file() {
  local label="$1"
  local path="$2"

  if [[ -z "$path" ]]; then
    fail "${label} path is required"
  fi
  if [[ ! -f "$path" ]]; then
    fail "${label} file does not exist: ${path}"
  fi
}

require_cert_san() {
  local cert_text="$1"
  local hostname="$2"

  if ! grep -F "DNS:${hostname}" <<<"$cert_text" >/dev/null; then
    fail "beta edge TLS certificate is missing SAN DNS:${hostname}"
  fi
}

require_matching_cert_key() {
  local fullchain="$1"
  local privkey="$2"
  local cert_pub_hash=""
  local key_pub_hash=""

  cert_pub_hash="$(openssl x509 -in "$fullchain" -pubkey -noout | openssl sha256)"
  key_pub_hash="$(openssl pkey -in "$privkey" -pubout | openssl sha256)"
  if [[ "$cert_pub_hash" != "$key_pub_hash" ]]; then
    fail "beta edge TLS private key does not match certificate public key"
  fi
}

require_env_value FISHYSTUFF_GITOPS_ENABLE_BETA_REMOTE_EDGE_TLS_INSTALL 1
require_env_value FISHYSTUFF_GITOPS_ENABLE_BETA_REMOTE_EDGE_RESTART 1
require_env_value FISHYSTUFF_GITOPS_BETA_REMOTE_EDGE_TLS_TARGET "$target"
require_env_nonempty FISHYSTUFF_GITOPS_BETA_EDGE_TLS_FULLCHAIN_SHA256
require_env_nonempty FISHYSTUFF_GITOPS_BETA_EDGE_TLS_PRIVKEY_SHA256
require_beta_deploy_profile
assert_deployment_configuration_safe beta
assert_beta_infra_cluster_dns_scope_safe
require_safe_target "$target"
require_expected_hostname "$expected_hostname"
require_command_or_executable openssl openssl
require_command_or_executable "$ssh_bin" ssh_bin
require_command_or_executable "$scp_bin" scp_bin
require_file fullchain "$fullchain_source"
require_file privkey "$privkey_source"

fullchain_sha256=""
privkey_sha256=""
read -r fullchain_sha256 _ < <(sha256sum "$fullchain_source")
read -r privkey_sha256 _ < <(sha256sum "$privkey_source")
if [[ "$fullchain_sha256" != "$FISHYSTUFF_GITOPS_BETA_EDGE_TLS_FULLCHAIN_SHA256" ]]; then
  fail "FISHYSTUFF_GITOPS_BETA_EDGE_TLS_FULLCHAIN_SHA256 does not match checked beta edge fullchain"
fi
if [[ "$privkey_sha256" != "$FISHYSTUFF_GITOPS_BETA_EDGE_TLS_PRIVKEY_SHA256" ]]; then
  fail "FISHYSTUFF_GITOPS_BETA_EDGE_TLS_PRIVKEY_SHA256 does not match checked beta edge private key"
fi
if ! openssl x509 -checkend 604800 -noout -in "$fullchain_source" >/dev/null; then
  fail "beta edge TLS certificate expires within 7 days"
fi
cert_text="$(openssl x509 -in "$fullchain_source" -noout -text)"
require_cert_san "$cert_text" beta.fishystuff.fish
require_cert_san "$cert_text" api.beta.fishystuff.fish
require_cert_san "$cert_text" cdn.beta.fishystuff.fish
require_cert_san "$cert_text" telemetry.beta.fishystuff.fish
require_matching_cert_key "$fullchain_source" "$privkey_source"

tmp_key="$(create_temp_ssh_key_from_env /tmp/fishystuff-beta-edge-tls-key.XXXXXX)"
known_hosts="$(mktemp /tmp/fishystuff-beta-edge-tls-known-hosts.XXXXXX)"
cleanup() {
  rm -f "$tmp_key" "$known_hosts"
}
trap cleanup EXIT

printf 'gitops_beta_remote_install_edge_tls_checked=true\n'
printf 'deployment=beta\n'
printf 'resident_target=%s\n' "$target"
printf 'fullchain_sha256=%s\n' "$fullchain_sha256"
printf 'privkey_sha256=%s\n' "$privkey_sha256"
printf 'tls_mode=operator_supplied\n'

ssh_common=(
  -i "$tmp_key"
  -o IdentitiesOnly=yes
  -o StrictHostKeyChecking=accept-new
  -o UserKnownHostsFile="$known_hosts"
)

remote_fullchain="/tmp/fishystuff-beta-edge-operator-fullchain.pem"
remote_privkey="/tmp/fishystuff-beta-edge-operator-privkey.pem"
"$scp_bin" "${ssh_common[@]}" "$fullchain_source" "${target}:${remote_fullchain}"
"$scp_bin" "${ssh_common[@]}" "$privkey_source" "${target}:${remote_privkey}"

"$ssh_bin" "${ssh_common[@]}" "$target" bash -s -- \
  "$expected_hostname" \
  "$remote_fullchain" \
  "$remote_privkey" \
  "$fullchain_sha256" \
  "$privkey_sha256" <<'REMOTE'
set -euo pipefail

expected_hostname="$1"
remote_fullchain="$2"
remote_privkey="$3"
fullchain_sha256="$4"
privkey_sha256="$5"
edge_tls_dir="/var/lib/fishystuff/gitops-beta/tls/live"

fail() {
  echo "$1" >&2
  exit 2
}

require_file() {
  local label="$1"
  local path="$2"

  if [[ ! -f "$path" ]]; then
    fail "${label} missing: ${path}"
  fi
}

require_sha256() {
  local label="$1"
  local path="$2"
  local expected="$3"
  local actual=""

  read -r actual _ < <(sha256sum "$path")
  if [[ "$actual" != "$expected" ]]; then
    fail "${label} sha256 mismatch: expected ${expected}, got ${actual}"
  fi
}

wait_trusted_edge_url() {
  local label="$1"
  local host="$2"
  local path="$3"
  local expected="$4"
  local attempts="$5"
  local n=0
  local body=""

  if ! command -v curl >/dev/null 2>&1; then
    fail "curl is required for beta edge TLS readiness"
  fi

  while (( n < attempts )); do
    body="$(curl -fsS --max-time 3 --resolve "${host}:443:127.0.0.1" "https://${host}${path}" 2>/dev/null || true)"
    if [[ -n "$body" && ( -z "$expected" || "$body" == *"$expected"* ) ]]; then
      printf '%s_trusted_ready=true\n' "$label"
      return 0
    fi
    if ! systemctl is-active --quiet fishystuff-beta-edge.service; then
      systemctl status fishystuff-beta-edge.service --no-pager || true
      journalctl -u fishystuff-beta-edge.service --no-pager -n 120 || true
      fail "beta edge unit stopped before trusted TLS ${host}${path} became ready"
    fi
    n="$((n + 1))"
    sleep 1
  done
  journalctl -u fishystuff-beta-edge.service --no-pager -n 120 || true
  fail "beta edge trusted TLS ${host}${path} did not become ready"
}

if [[ "$(hostname)" != "$expected_hostname" ]]; then
  fail "remote hostname mismatch: expected ${expected_hostname}, got $(hostname)"
fi

require_file "operator fullchain" "$remote_fullchain"
require_file "operator private key" "$remote_privkey"
require_sha256 "operator fullchain" "$remote_fullchain" "$fullchain_sha256"
require_sha256 "operator private key" "$remote_privkey" "$privkey_sha256"

install -d -m 0700 "$edge_tls_dir"
install -m 0644 "$remote_fullchain" "${edge_tls_dir}/fullchain.pem"
install -m 0600 "$remote_privkey" "${edge_tls_dir}/privkey.pem"
rm -f "$remote_fullchain" "$remote_privkey"
systemctl restart fishystuff-beta-edge.service
systemctl is-active --quiet fishystuff-beta-edge.service
wait_trusted_edge_url edge_site beta.fishystuff.fish / '' 120
wait_trusted_edge_url edge_api_meta api.beta.fishystuff.fish /api/v1/meta release- 120
wait_trusted_edge_url edge_cdn_runtime cdn.beta.fishystuff.fish /map/runtime-manifest.json fishystuff_ui_bevy 120

printf 'remote_hostname=%s\n' "$(hostname)"
printf 'remote_edge_tls_install_ok=true\n'
printf 'remote_edge_service_restart_ok=fishystuff-beta-edge.service\n'
printf 'remote_host_mutation_performed=true\n'
printf 'remote_deploy_performed=false\n'
printf 'infrastructure_mutation_performed=false\n'
printf 'local_host_mutation_performed=false\n'
REMOTE

printf 'gitops_beta_remote_install_edge_tls_ok=true\n'
printf 'remote_host_mutation_performed=true\n'
printf 'remote_deploy_performed=false\n'
printf 'infrastructure_mutation_performed=false\n'
printf 'local_host_mutation_performed=false\n'
