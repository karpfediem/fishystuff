#!/usr/bin/env bash

RECIPE_LIB_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
RECIPE_REPO_ROOT="$(cd "${RECIPE_LIB_DIR}/../../.." && pwd)"

normalize_named_arg() {
  local name="$1"
  local value="${2-}"
  if [[ "$value" == "$name="* ]]; then
    printf '%s' "${value#*=}"
    return
  fi
  printf '%s' "$value"
}

require_value() {
  local value="$1"
  local message="$2"
  if [[ -z "$value" ]]; then
    echo "$message" >&2
    exit 2
  fi
}

normalize_deployment_environment() {
  local value="$1"
  value="$(printf '%s' "$value" | tr '[:upper:]' '[:lower:]')"
  if [[ -z "$value" ]]; then
    printf '%s' "beta"
    return
  fi
  printf '%s' "$value"
}

deployment_domain() {
  local value="$1"
  if [[ "$value" == "production" ]]; then
    printf '%s' "fishystuff.fish"
    return
  fi
  printf '%s' "${value}.fishystuff.fish"
}

merge_json_env_from_keys() {
  local base_json="$1"
  local pairs_csv="$2"
  local merged_json="$base_json"
  local -a env_entries=()
  local entry=""
  local key=""
  local env_name=""
  local value=""

  [[ -n "$pairs_csv" ]] || {
    printf '%s' "$merged_json"
    return
  }

  IFS=',' read -r -a env_entries <<< "$pairs_csv"
  for entry in "${env_entries[@]}"; do
    [[ -n "$entry" ]] || continue
    key="${entry%%=*}"
    env_name="${entry#*=}"
    if [[ "$entry" != *=* ]]; then
      env_name="$entry"
    fi
    if [[ -z "$key" || -z "$env_name" ]]; then
      echo "invalid key/env entry: $entry" >&2
      exit 2
    fi
    value="${!env_name:-}"
    if [[ -z "$value" ]]; then
      echo "missing environment variable for entry: $entry" >&2
      exit 2
    fi
    merged_json="$(
      jq -cn \
        --argjson current "$merged_json" \
        --arg key "$key" \
        --arg value "$value" \
        '$current + {($key): $value}'
    )"
  done

  printf '%s' "$merged_json"
}

create_temp_ssh_key_from_env() {
  local prefix="${1:-/tmp/fishystuff-ssh.XXXXXX}"
  local tmp_key=""

  tmp_key="$(mktemp "$prefix")"
  umask 077
  printf '%s\n' "${HETZNER_SSH_PRIVATE_KEY:?}" > "$tmp_key"
  chmod 600 "$tmp_key"
  printf '%s' "$tmp_key"
}

detect_remote_nix_probe() {
  local ssh_target="$1"
  local tmp_key="$2"

  ssh \
    -i "$tmp_key" \
    -o IdentitiesOnly=yes \
    -o StrictHostKeyChecking=accept-new \
    "$ssh_target" \
    '
      nix_path=""
      nix_daemon_path=""
      if test -x /nix/var/nix/profiles/default/bin/nix; then
        nix_path=/nix/var/nix/profiles/default/bin/nix
      elif command -v nix >/dev/null 2>&1; then
        nix_path="$(command -v nix)"
      fi
      if test -x /nix/var/nix/profiles/default/bin/nix-daemon; then
        nix_daemon_path=/nix/var/nix/profiles/default/bin/nix-daemon
      elif command -v nix-daemon >/dev/null 2>&1; then
        nix_daemon_path="$(command -v nix-daemon)"
      fi
      printf "%s\t%s\n" "$nix_path" "$nix_daemon_path"
    ' \
    2>/dev/null || true
}

read_remote_nix_paths() {
  local ssh_target="$1"
  local tmp_key="$2"
  local probe=""
  local nix_path=""
  local nix_daemon_path=""

  probe="$(detect_remote_nix_probe "$ssh_target" "$tmp_key")"
  if [[ -n "$probe" ]]; then
    IFS=$'\t' read -r nix_path nix_daemon_path <<< "$probe"
  fi
  printf '%s\t%s\n' "$nix_path" "$nix_daemon_path"
}

detect_remote_nix_daemon_path() {
  local ssh_target="$1"
  local tmp_key="$2"
  local nix_probe=""
  local nix_daemon_path=""

  nix_probe="$(read_remote_nix_paths "$ssh_target" "$tmp_key")"
  if [[ -n "$nix_probe" ]]; then
    IFS=$'\t' read -r _nix_path nix_daemon_path <<< "$nix_probe"
  fi
  printf '%s' "$nix_daemon_path"
}

build_nix_copy_target() {
  local ssh_target="$1"
  local tmp_key="$2"
  local remote_program="${3-}"
  local target="ssh-ng://$ssh_target?ssh-key=$tmp_key"

  if [[ -n "$remote_program" ]]; then
    target="${target}&remote-program=$remote_program"
  fi
  printf '%s' "$target"
}

copy_resident_common_modules() {
  local deploy_dir="$1"
  local mgmt_modules_dir="$2"
  local module_name=""

  mkdir -p "$deploy_dir/modules/lib" "$deploy_dir/modules/providers"
  for module_name in fishystuff-beta-access hetzner-firewall-gate systemd-daemon-reload; do
    cp -a "$RECIPE_REPO_ROOT/mgmt/modules/lib/$module_name" "$deploy_dir/modules/lib/"
  done
  cp -a "$RECIPE_REPO_ROOT/mgmt/modules/providers/hetzner-firewall" "$deploy_dir/modules/providers/"
  mkdir -p "$deploy_dir/modules/github.com/purpleidea/mgmt/modules"
  cp -a "$mgmt_modules_dir/misc" "$deploy_dir/modules/github.com/purpleidea/mgmt/modules/"
}
