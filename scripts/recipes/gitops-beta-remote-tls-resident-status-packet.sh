#!/usr/bin/env bash
set -euo pipefail

SCRIPT_PATH="$(readlink -f "${BASH_SOURCE[0]}")"
SCRIPT_DIR="$(cd "$(dirname "$SCRIPT_PATH")" && pwd)"
source "${SCRIPT_DIR}/lib/common.sh"

target="$(normalize_named_arg target "${1:-${FISHYSTUFF_BETA_RESIDENT_TARGET:-}}")"
expected_hostname="$(normalize_named_arg expected_hostname "${2:-site-nbg1-beta}")"
ssh_bin="$(normalize_named_arg ssh_bin "${3:-${FISHYSTUFF_GITOPS_SSH_BIN:-ssh}}")"

cd "$RECIPE_REPO_ROOT"

fail() {
  echo "$1" >&2
  exit 2
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
      fail "gitops-beta-remote-tls-resident-status-packet must not run with production SecretSpec profile active: ${active_profile}"
      ;;
    *)
      fail "gitops-beta-remote-tls-resident-status-packet requires FISHYSTUFF_OPERATOR_SECRETSPEC_PROFILE=beta-deploy"
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
    fail "fresh beta TLS resident status currently expects root SSH, got user: ${user}"
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
    fail "expected_hostname must be a hostname, got ${value}; pass arguments in order: target expected_hostname"
  fi
  if [[ ! "$value" =~ ^[a-zA-Z0-9][a-zA-Z0-9.-]*$ ]]; then
    fail "expected_hostname must be a simple hostname, got: ${value}"
  fi
}

require_command_or_executable "$ssh_bin" ssh_bin
require_beta_deploy_profile
assert_deployment_configuration_safe beta
assert_beta_infra_cluster_dns_scope_safe
require_safe_target "$target"
require_expected_hostname "$expected_hostname"

tmp_key="$(create_temp_ssh_key_from_env /tmp/fishystuff-beta-tls-resident-status-key.XXXXXX)"
known_hosts="$(mktemp /tmp/fishystuff-beta-tls-resident-status-known-hosts.XXXXXX)"
cleanup() {
  rm -f "$tmp_key" "$known_hosts"
}
trap cleanup EXIT

ssh_common=(
  -i "$tmp_key"
  -o IdentitiesOnly=yes
  -o BatchMode=yes
  -o ConnectTimeout=120
  -o ConnectionAttempts=1
  -o ServerAliveInterval=10
  -o ServerAliveCountMax=3
  -o StrictHostKeyChecking=accept-new
  -o UserKnownHostsFile="$known_hosts"
)

printf 'gitops_beta_remote_tls_resident_status_packet_ok=true\n'
printf 'deployment=beta\n'
printf 'resident_target=%s\n' "$target"
printf 'expected_hostname=%s\n' "$expected_hostname"
printf 'remote_deploy_performed=false\n'
printf 'infrastructure_mutation_performed=false\n'
printf 'local_host_mutation_performed=false\n'

"$ssh_bin" "${ssh_common[@]}" "$target" bash -s -- "$expected_hostname" <<'REMOTE'
set -euo pipefail

expected_hostname="$1"
unit_name="fishystuff-beta-tls-reconciler.service"
desired_state="/var/lib/fishystuff/gitops-beta/desired/beta-tls.desired.json"
unit_file="/etc/systemd/system/${unit_name}"
cloudflare_token="/var/lib/fishystuff/gitops-beta/secrets/cloudflare-api-token"
fullchain="/var/lib/fishystuff/gitops-beta/tls/live/fullchain.pem"
privkey="/var/lib/fishystuff/gitops-beta/tls/live/privkey.pem"

file_inventory() {
  local label="$1"
  local path="$2"
  local sha=""

  printf '%s_path=%s\n' "$label" "$path"
  if [[ ! -e "$path" ]]; then
    printf '%s_exists=false\n' "$label"
    return
  fi
  printf '%s_exists=true\n' "$label"
  printf '%s_mode=%s\n' "$label" "$(stat -c '%a' "$path" 2>/dev/null || true)"
  printf '%s_owner_group=%s\n' "$label" "$(stat -c '%U:%G' "$path" 2>/dev/null || true)"
  if [[ -f "$path" ]]; then
    read -r sha _ < <(sha256sum "$path")
    printf '%s_sha256=%s\n' "$label" "$sha"
  fi
}

cert_inventory() {
  local label="$1"
  local path="$2"
  local san_text=""

  file_inventory "$label" "$path"
  if [[ ! -f "$path" ]] || ! openssl x509 -in "$path" -noout >/dev/null 2>&1; then
    printf '%s_parse_ok=false\n' "$label"
    return
  fi
  san_text="$(openssl x509 -in "$path" -noout -ext subjectAltName 2>/dev/null || true)"
  printf '%s_parse_ok=true\n' "$label"
  printf '%s_subject=%s\n' "$label" "$(openssl x509 -in "$path" -noout -subject | sed 's/^subject=//')"
  printf '%s_issuer=%s\n' "$label" "$(openssl x509 -in "$path" -noout -issuer | sed 's/^issuer=//')"
  printf '%s_not_after=%s\n' "$label" "$(openssl x509 -in "$path" -noout -enddate | sed 's/^notAfter=//')"
  if openssl x509 -checkend 604800 -noout -in "$path" >/dev/null 2>&1; then
    printf '%s_valid_more_than_7d=true\n' "$label"
  else
    printf '%s_valid_more_than_7d=false\n' "$label"
  fi
  for host in beta.fishystuff.fish api.beta.fishystuff.fish cdn.beta.fishystuff.fish telemetry.beta.fishystuff.fish; do
    if grep -F "DNS:${host}" <<<"$san_text" >/dev/null; then
      printf '%s_san_%s=true\n' "$label" "${host//./_}"
    else
      printf '%s_san_%s=false\n' "$label" "${host//./_}"
    fi
  done
}

if [[ "$(hostname)" == "$expected_hostname" ]]; then
  printf 'remote_hostname_match=true\n'
else
  printf 'remote_hostname_match=false\n'
fi
printf 'remote_hostname=%s\n' "$(hostname)"
printf 'remote_tls_resident_unit_load_state=%s\n' "$(systemctl show "$unit_name" -p LoadState --value 2>/dev/null || true)"
printf 'remote_tls_resident_unit_active_state=%s\n' "$(systemctl show "$unit_name" -p ActiveState --value 2>/dev/null || true)"
printf 'remote_tls_resident_unit_sub_state=%s\n' "$(systemctl show "$unit_name" -p SubState --value 2>/dev/null || true)"
printf 'remote_tls_resident_unit_file_state=%s\n' "$(systemctl show "$unit_name" -p UnitFileState --value 2>/dev/null || true)"
printf 'remote_tls_resident_unit_main_pid=%s\n' "$(systemctl show "$unit_name" -p MainPID --value 2>/dev/null || true)"
printf 'remote_tls_resident_unit_result=%s\n' "$(systemctl show "$unit_name" -p Result --value 2>/dev/null || true)"
printf 'remote_tls_resident_unit_exec_main_status=%s\n' "$(systemctl show "$unit_name" -p ExecMainStatus --value 2>/dev/null || true)"
printf 'remote_tls_resident_unit_n_restarts=%s\n' "$(systemctl show "$unit_name" -p NRestarts --value 2>/dev/null || true)"

file_inventory remote_tls_resident_desired_state "$desired_state"
file_inventory remote_tls_resident_unit_file "$unit_file"
file_inventory remote_tls_resident_cloudflare_token "$cloudflare_token"
cert_inventory remote_tls_resident_fullchain "$fullchain"
file_inventory remote_tls_resident_privkey "$privkey"

if [[ -f "$fullchain" && -f "$privkey" ]]; then
  cert_pub_hash="$(openssl x509 -in "$fullchain" -pubkey -noout 2>/dev/null | openssl sha256 2>/dev/null | awk '{ print $NF }' || true)"
  key_pub_hash="$(openssl pkey -in "$privkey" -pubout 2>/dev/null | openssl sha256 2>/dev/null | awk '{ print $NF }' || true)"
  if [[ -n "$cert_pub_hash" && "$cert_pub_hash" == "$key_pub_hash" ]]; then
    printf 'remote_tls_resident_cert_key_match=true\n'
  else
    printf 'remote_tls_resident_cert_key_match=false\n'
  fi
else
  printf 'remote_tls_resident_cert_key_match=unknown\n'
fi

printf 'remote_host_mutation_performed=false\n'
printf 'remote_deploy_performed=false\n'
printf 'infrastructure_mutation_performed=false\n'
printf 'local_host_mutation_performed=false\n'
REMOTE
