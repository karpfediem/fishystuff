#!/usr/bin/env bash
set -euo pipefail

SCRIPT_PATH="$(readlink -f "${BASH_SOURCE[0]}")"
SCRIPT_DIR="$(cd "$(dirname "$SCRIPT_PATH")" && pwd)"
source "${SCRIPT_DIR}/lib/common.sh"

target="$(normalize_named_arg target "${1:-${FISHYSTUFF_BETA_RESIDENT_TARGET:-}}")"
expected_hostname="$(normalize_named_arg expected_hostname "${2:-${FISHYSTUFF_BETA_RESIDENT_HOSTNAME:-$(deployment_resident_hostname beta)}}")"
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
      fail "gitops-beta-remote-host-preflight must not run with production SecretSpec profile active: ${active_profile}"
      ;;
    *)
      fail "gitops-beta-remote-host-preflight requires FISHYSTUFF_OPERATOR_SECRETSPEC_PROFILE=beta-deploy"
      ;;
  esac
}

require_safe_target() {
  local value="$1"
  local user=""
  local host=""

  if [[ -z "$value" ]]; then
    fail "target is required; use FISHYSTUFF_BETA_RESIDENT_TARGET=root@<fresh-beta-ip>"
  fi
  if [[ "$value" != *@* ]]; then
    fail "target must be user@IPv4, got: ${value}"
  fi
  user="${value%@*}"
  host="${value#*@}"
  if [[ "$user" != "root" ]]; then
    fail "fresh beta host preflight currently expects root SSH, got user: ${user}"
  fi
  if [[ ! "$host" =~ ^([0-9]{1,3}\.){3}[0-9]{1,3}$ ]]; then
    fail "target host must be an IPv4 address, got: ${host}"
  fi
  if [[ "$host" == "178.104.230.121" ]]; then
    fail "target points at the previous beta host; use the fresh replacement IP"
  fi
}

require_safe_expected_hostname() {
  local value="$1"

  if [[ "$value" != "$(deployment_resident_hostname beta)" ]]; then
    fail "expected hostname must be $(deployment_resident_hostname beta), got: ${value}"
  fi
}

require_beta_deploy_profile
assert_deployment_configuration_safe beta
assert_beta_infra_cluster_dns_scope_safe
require_safe_target "$target"
require_safe_expected_hostname "$expected_hostname"
require_command_or_executable "$ssh_bin" ssh_bin

tmp_key="$(create_temp_ssh_key_from_env /tmp/fishystuff-beta-remote-preflight-key.XXXXXX)"
known_hosts="$(mktemp /tmp/fishystuff-beta-remote-preflight-known-hosts.XXXXXX)"
cleanup() {
  rm -f "$tmp_key" "$known_hosts"
}
trap cleanup EXIT

printf 'gitops_beta_remote_host_preflight_ok=true\n'
printf 'deployment=beta\n'
printf 'resident_target=%s\n' "$target"
printf 'expected_hostname=%s\n' "$expected_hostname"
printf 'ssh_probe_performed=true\n'

"$ssh_bin" \
  -i "$tmp_key" \
  -o IdentitiesOnly=yes \
  -o StrictHostKeyChecking=accept-new \
  -o UserKnownHostsFile="$known_hosts" \
  "$target" \
  "EXPECTED_HOSTNAME=$expected_hostname sh -s" <<'REMOTE'
set -eu

bool_command() {
  if command -v "$1" >/dev/null 2>&1; then
    printf 'true'
  else
    printf 'false'
  fi
}

bool_group() {
  if getent group "$1" >/dev/null 2>&1; then
    printf 'true'
  else
    printf 'false'
  fi
}

bool_user() {
  if getent passwd "$1" >/dev/null 2>&1; then
    printf 'true'
  else
    printf 'false'
  fi
}

bool_dir() {
  if test -d "$1"; then
    printf 'true'
  else
    printf 'false'
  fi
}

path_or_empty() {
  if command -v "$1" >/dev/null 2>&1; then
    command -v "$1"
  else
    printf ''
  fi
}

remote_hostname="$(hostname)"
os_id="unknown"
os_version_id="unknown"
if test -r /etc/os-release; then
  . /etc/os-release
  os_id="${ID:-unknown}"
  os_version_id="${VERSION_ID:-unknown}"
fi
systemd_state="unknown"
if command -v systemctl >/dev/null 2>&1; then
  systemd_state="$(systemctl is-system-running 2>/dev/null || true)"
fi
nix_path="$(path_or_empty nix)"
nix_daemon_path="$(path_or_empty nix-daemon)"

printf 'remote_hostname=%s\n' "$remote_hostname"
if test "$remote_hostname" = "$EXPECTED_HOSTNAME"; then
  printf 'expected_hostname_match=true\n'
else
  printf 'expected_hostname_match=false\n'
fi
printf 'os_id=%s\n' "$os_id"
printf 'os_version_id=%s\n' "$os_version_id"
printf 'systemd_available=%s\n' "$(bool_command systemctl)"
printf 'systemd_state=%s\n' "$systemd_state"
printf 'nix_available=%s\n' "$(bool_command nix)"
printf 'nix_path=%s\n' "$nix_path"
printf 'nix_daemon_available=%s\n' "$(bool_command nix-daemon)"
printf 'nix_daemon_path=%s\n' "$nix_daemon_path"
printf 'beta_group_exists=%s\n' "$(bool_group fishystuff-beta-dolt)"
printf 'beta_user_exists=%s\n' "$(bool_user fishystuff-beta-dolt)"
printf 'beta_directory_01_exists=%s\n' "$(bool_dir /var/lib/fishystuff/gitops-beta)"
printf 'beta_directory_02_exists=%s\n' "$(bool_dir /var/lib/fishystuff/gitops-beta/api)"
printf 'beta_directory_03_exists=%s\n' "$(bool_dir /var/lib/fishystuff/gitops-beta/dolt)"
printf 'beta_directory_04_exists=%s\n' "$(bool_dir /var/lib/fishystuff/gitops-beta/dolt-cache/fishystuff)"
printf 'beta_directory_05_exists=%s\n' "$(bool_dir /var/lib/fishystuff/gitops-beta/served/beta)"
printf 'beta_directory_06_exists=%s\n' "$(bool_dir /run/fishystuff/gitops-beta)"
printf 'beta_directory_07_exists=%s\n' "$(bool_dir /run/fishystuff/beta-edge/tls)"
printf 'beta_directory_08_exists=%s\n' "$(bool_dir /var/lib/fishystuff/beta-dolt)"
REMOTE

printf 'next_required_action=bootstrap_remote_beta_host\n'
printf 'next_command_01=FISHYSTUFF_OPERATOR_SECRETSPEC_PROFILE=beta-deploy FISHYSTUFF_GITOPS_ENABLE_BETA_REMOTE_BOOTSTRAP=1 FISHYSTUFF_GITOPS_ENABLE_BETA_REMOTE_DIRECTORIES=1 FISHYSTUFF_GITOPS_ENABLE_BETA_REMOTE_USER_GROUPS=1 secretspec run --profile beta-deploy -- just gitops-beta-remote-host-bootstrap target=%s\n' "$target"
printf 'remote_deploy_performed=false\n'
printf 'remote_host_mutation_performed=false\n'
printf 'infrastructure_mutation_performed=false\n'
printf 'local_host_mutation_performed=false\n'
