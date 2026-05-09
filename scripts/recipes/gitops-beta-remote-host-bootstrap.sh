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
    fail "gitops-beta-remote-host-bootstrap requires ${name}=${expected}"
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
      fail "gitops-beta-remote-host-bootstrap must not run with production SecretSpec profile active: ${active_profile}"
      ;;
    *)
      fail "gitops-beta-remote-host-bootstrap requires FISHYSTUFF_OPERATOR_SECRETSPEC_PROFILE=beta-deploy"
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
    fail "fresh beta host bootstrap currently expects root SSH, got user: ${user}"
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

require_env_value FISHYSTUFF_GITOPS_ENABLE_BETA_REMOTE_BOOTSTRAP 1
require_env_value FISHYSTUFF_GITOPS_ENABLE_BETA_REMOTE_DIRECTORIES 1
require_env_value FISHYSTUFF_GITOPS_ENABLE_BETA_REMOTE_USER_GROUPS 1
require_beta_deploy_profile
assert_deployment_configuration_safe beta
assert_beta_infra_cluster_dns_scope_safe
require_safe_target "$target"
require_safe_expected_hostname "$expected_hostname"
require_command_or_executable "$ssh_bin" ssh_bin

tmp_key="$(create_temp_ssh_key_from_env /tmp/fishystuff-beta-remote-bootstrap-key.XXXXXX)"
known_hosts="$(mktemp /tmp/fishystuff-beta-remote-bootstrap-known-hosts.XXXXXX)"
cleanup() {
  rm -f "$tmp_key" "$known_hosts"
}
trap cleanup EXIT

printf 'gitops_beta_remote_host_bootstrap_ok=true\n'
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

ensure_group() {
  group="$1"
  if getent group "$group" >/dev/null 2>&1; then
    printf 'existing'
    return
  fi
  groupadd --system "$group"
  printf 'created'
}

ensure_user() {
  user="$1"
  group="$2"
  home="$3"
  if getent passwd "$user" >/dev/null 2>&1; then
    printf 'existing'
    return
  fi
  useradd --system --gid "$group" --home-dir "$home" --no-create-home "$user"
  printf 'created'
}

remote_hostname="$(hostname)"
if test "$remote_hostname" != "$EXPECTED_HOSTNAME"; then
  fail "remote hostname must be $EXPECTED_HOSTNAME, got: $remote_hostname"
fi

group_action="$(ensure_group fishystuff-beta-dolt)"
user_action="$(ensure_user fishystuff-beta-dolt fishystuff-beta-dolt /var/lib/fishystuff/beta-dolt/home)"

install -d -m 0750 /var/lib/fishystuff/gitops-beta
install -d -m 0750 /var/lib/fishystuff/gitops-beta/api
install -d -m 0750 /var/lib/fishystuff/gitops-beta/dolt
install -d -m 0750 /var/lib/fishystuff/gitops-beta/dolt-cache/fishystuff
install -d -m 0755 /var/lib/fishystuff/gitops-beta/served/beta
install -d -m 0750 /run/fishystuff/gitops-beta
install -d -m 0700 /run/fishystuff/beta-edge/tls
install -d -m 0750 /var/lib/fishystuff/beta-dolt

printf 'remote_hostname=%s\n' "$remote_hostname"
printf 'expected_hostname_match=true\n'
printf 'beta_group=fishystuff-beta-dolt\n'
printf 'beta_group_action=%s\n' "$group_action"
printf 'beta_user=fishystuff-beta-dolt\n'
printf 'beta_user_action=%s\n' "$user_action"
printf 'beta_directory_01=0750:/var/lib/fishystuff/gitops-beta\n'
printf 'beta_directory_02=0750:/var/lib/fishystuff/gitops-beta/api\n'
printf 'beta_directory_03=0750:/var/lib/fishystuff/gitops-beta/dolt\n'
printf 'beta_directory_04=0750:/var/lib/fishystuff/gitops-beta/dolt-cache/fishystuff\n'
printf 'beta_directory_05=0755:/var/lib/fishystuff/gitops-beta/served/beta\n'
printf 'beta_directory_06=0750:/run/fishystuff/gitops-beta\n'
printf 'beta_directory_07=0700:/run/fishystuff/beta-edge/tls\n'
printf 'beta_directory_08=0750:/var/lib/fishystuff/beta-dolt\n'
REMOTE

printf 'remote_host_mutation_performed=true\n'
printf 'remote_deploy_performed=false\n'
printf 'infrastructure_mutation_performed=false\n'
printf 'local_host_mutation_performed=false\n'
