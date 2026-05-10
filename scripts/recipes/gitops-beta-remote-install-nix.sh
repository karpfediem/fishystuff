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

require_env_value() {
  local name="$1"
  local expected="$2"
  local value="${!name-}"

  if [[ "$value" != "$expected" ]]; then
    fail "gitops-beta-remote-install-nix requires ${name}=${expected}"
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
      fail "gitops-beta-remote-install-nix must not run with production SecretSpec profile active: ${active_profile}"
      ;;
    *)
      fail "gitops-beta-remote-install-nix requires FISHYSTUFF_OPERATOR_SECRETSPEC_PROFILE=beta-deploy"
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
    fail "fresh beta Nix install currently expects root SSH, got user: ${user}"
  fi
  if [[ ! "$host" =~ ^([0-9]{1,3}\.){3}[0-9]{1,3}$ ]]; then
    fail "target host must be an IPv4 address, got: ${host}"
  fi
}

require_safe_expected_hostname() {
  local value="$1"

  if [[ "$value" != "$(deployment_resident_hostname beta)" ]]; then
    fail "expected hostname must be $(deployment_resident_hostname beta), got: ${value}"
  fi
}

require_env_value FISHYSTUFF_GITOPS_ENABLE_BETA_REMOTE_NIX_INSTALL 1
require_env_value FISHYSTUFF_GITOPS_ENABLE_BETA_REMOTE_NIX_APT_PREREQS 1
require_beta_deploy_profile
assert_deployment_configuration_safe beta
assert_beta_infra_cluster_dns_scope_safe
require_safe_target "$target"
require_safe_expected_hostname "$expected_hostname"
require_command_or_executable "$ssh_bin" ssh_bin

tmp_key="$(create_temp_ssh_key_from_env /tmp/fishystuff-beta-remote-nix-key.XXXXXX)"
known_hosts="$(mktemp /tmp/fishystuff-beta-remote-nix-known-hosts.XXXXXX)"
cleanup() {
  rm -f "$tmp_key" "$known_hosts"
}
trap cleanup EXIT

printf 'gitops_beta_remote_install_nix_ok=true\n'
printf 'deployment=beta\n'
printf 'resident_target=%s\n' "$target"
printf 'expected_hostname=%s\n' "$expected_hostname"

"$ssh_bin" \
  -i "$tmp_key" \
  -o IdentitiesOnly=yes \
  -o StrictHostKeyChecking=accept-new \
  -o UserKnownHostsFile="$known_hosts" \
  "$target" \
  "EXPECTED_HOSTNAME=$expected_hostname sh -s" <<'REMOTE'
set -eu

fail() {
  echo "$1" >&2
  exit 2
}

command_path() {
  if test -x "/nix/var/nix/profiles/default/bin/$1"; then
    printf '/nix/var/nix/profiles/default/bin/%s' "$1"
    return
  fi
  if command -v "$1" >/dev/null 2>&1; then
    command -v "$1"
    return
  fi
  printf ''
}

remote_hostname="$(hostname)"
if test "$remote_hostname" != "$EXPECTED_HOSTNAME"; then
  fail "remote hostname must be $EXPECTED_HOSTNAME, got: $remote_hostname"
fi
if ! command -v systemctl >/dev/null 2>&1; then
  fail "systemctl is required for a multi-user Nix install"
fi

nix_path="$(command_path nix)"
nix_daemon_path="$(command_path nix-daemon)"
if test -n "$nix_path" && test -n "$nix_daemon_path"; then
  printf 'remote_hostname=%s\n' "$remote_hostname"
  printf 'remote_nix_install_action=already_present\n'
  printf 'nix_path=%s\n' "$nix_path"
  printf 'nix_version=%s\n' "$("$nix_path" --version 2>/dev/null || true)"
  printf 'nix_daemon_path=%s\n' "$nix_daemon_path"
  printf 'remote_host_mutation_performed=false\n'
  printf 'remote_deploy_performed=false\n'
  printf 'infrastructure_mutation_performed=false\n'
  printf 'local_host_mutation_performed=false\n'
  exit 0
fi

if ! command -v apt-get >/dev/null 2>&1; then
  fail "apt-get is required to install Nix prerequisites on this Debian beta host"
fi

export DEBIAN_FRONTEND=noninteractive
apt-get update
apt-get install -y --no-install-recommends ca-certificates curl xz-utils

installer=/tmp/fishystuff-nix-install
rm -f "$installer"
curl -fsSL https://nixos.org/nix/install -o "$installer"
sh "$installer" --daemon --yes
rm -f "$installer"

systemctl daemon-reload || true
if systemctl list-unit-files nix-daemon.service >/dev/null 2>&1; then
  systemctl enable --now nix-daemon.service >/dev/null 2>&1 || true
fi
if systemctl list-unit-files nix-daemon.socket >/dev/null 2>&1; then
  systemctl enable --now nix-daemon.socket >/dev/null 2>&1 || true
fi

nix_path="$(command_path nix)"
nix_daemon_path="$(command_path nix-daemon)"
if test -z "$nix_path"; then
  fail "Nix install completed but nix was not found"
fi
if test -z "$nix_daemon_path"; then
  fail "Nix install completed but nix-daemon was not found"
fi

printf 'remote_hostname=%s\n' "$remote_hostname"
printf 'remote_nix_install_action=installed\n'
printf 'nix_path=%s\n' "$nix_path"
printf 'nix_version=%s\n' "$("$nix_path" --version 2>/dev/null || true)"
printf 'nix_daemon_path=%s\n' "$nix_daemon_path"
printf 'nix_daemon_service_state=%s\n' "$(systemctl is-active nix-daemon.service 2>/dev/null || true)"
printf 'nix_daemon_socket_state=%s\n' "$(systemctl is-active nix-daemon.socket 2>/dev/null || true)"
printf 'remote_host_mutation_performed=true\n'
printf 'remote_deploy_performed=false\n'
printf 'infrastructure_mutation_performed=false\n'
printf 'local_host_mutation_performed=false\n'
REMOTE
