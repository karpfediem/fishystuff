#!/usr/bin/env bash
set -euo pipefail

SCRIPT_PATH="$(readlink -f "${BASH_SOURCE[0]}")"
SCRIPT_DIR="$(cd "$(dirname "$SCRIPT_PATH")" && pwd)"
source "${SCRIPT_DIR}/lib/common.sh"

cd "$RECIPE_REPO_ROOT"

require_command() {
  local command_name="$1"
  if ! command -v "$command_name" >/dev/null 2>&1; then
    echo "missing required command: ${command_name}" >&2
    exit 127
  fi
}

secret_status() {
  local name="$1"
  local value="$2"
  local marker="${tmp_dir}/secret-${name}.unavailable"

  if [[ -e "$marker" ]]; then
    printf 'unavailable'
    return
  fi

  if [[ -n "$value" ]]; then
    printf 'present'
  else
    printf 'missing'
  fi
}

secret_value() {
  local name="$1"
  local output="${tmp_dir}/secret-${name}.out"
  local error="${tmp_dir}/secret-${name}.err"

  if secretspec get --profile beta-deploy "$name" >"$output" 2>"$error"; then
    cat "$output"
    return
  fi

  if grep -E 'DBus error|secure storage|Operation not permitted|permission denied' "$error" >/dev/null; then
    : >"${tmp_dir}/secret-${name}.unavailable"
  fi
}

kv_value() {
  local key="$1"
  local file="$2"
  awk -F= -v key="$key" '$1 == key { print substr($0, length(key) + 2); exit }' "$file"
}

public_key_identity() {
  local value="$1"
  awk '{ print $1 " " $2 }' <<<"$value"
}

fingerprint_public_key() {
  local value="$1"
  local file="$2"
  local fingerprint=""

  printf '%s\n' "$value" >"$file"
  fingerprint="$(ssh-keygen -lf "$file" 2>/dev/null | awk '{ print $2; exit }' || true)"
  printf '%s' "$fingerprint"
}

inspect_key_pair() {
  local private_key="$1"
  local public_key="$2"
  local tmp_dir="$3"
  local private_file="${tmp_dir}/beta-deploy-key"
  local public_file="${tmp_dir}/beta-deploy-key.pub"
  local derived_public=""
  local public_fingerprint=""

  umask 077
  printf '%s\n' "$private_key" >"$private_file"
  chmod 600 "$private_file"

  derived_public="$(ssh-keygen -y -f "$private_file" 2>/dev/null || true)"
  if [[ -z "$derived_public" ]]; then
    printf 'beta_deploy_ssh_private_key_status=invalid\n'
    printf 'beta_deploy_ssh_public_key_status=%s\n' "$(secret_status HETZNER_SSH_PUBLIC_KEY "$public_key")"
    printf 'beta_deploy_ssh_key_pair_match=unknown\n'
    return
  fi

  public_fingerprint="$(fingerprint_public_key "$public_key" "$public_file")"
  if [[ -z "$public_fingerprint" ]]; then
    printf 'beta_deploy_ssh_private_key_status=present\n'
    printf 'beta_deploy_ssh_public_key_status=invalid\n'
    printf 'beta_deploy_ssh_key_pair_match=unknown\n'
    return
  fi

  printf 'beta_deploy_ssh_private_key_status=present\n'
  printf 'beta_deploy_ssh_public_key_status=present\n'
  if [[ "$(public_key_identity "$derived_public")" == "$(public_key_identity "$public_key")" ]]; then
    printf 'beta_deploy_ssh_key_pair_match=true\n'
  else
    printf 'beta_deploy_ssh_key_pair_match=false\n'
  fi
  printf 'beta_deploy_ssh_public_key_fingerprint=%s\n' "$public_fingerprint"
}

active_profile="${FISHYSTUFF_OPERATOR_SECRETSPEC_PROFILE:-<not-loaded>}"
case "$active_profile" in
  production-deploy | prod-deploy | production)
    echo "gitops-beta-deploy-credentials-packet must not run with production SecretSpec profile active: ${active_profile}" >&2
    exit 2
    ;;
esac

require_command awk
require_command mktemp
require_command secretspec
require_command ssh-keygen

tmp_dir="$(mktemp -d)"
cleanup() {
  rm -rf "$tmp_dir"
}
trap cleanup EXIT

check_status="missing"
if secretspec check --profile beta-deploy --no-prompt >"${tmp_dir}/check.out" 2>"${tmp_dir}/check.err"; then
  check_status="present"
fi

hetzner_token="$(secret_value HETZNER_API_TOKEN)"
cloudflare_token="$(secret_value CLOUDFLARE_API_TOKEN)"
key_name="$(secret_value HETZNER_SSH_KEY_NAME)"
private_key="$(secret_value HETZNER_SSH_PRIVATE_KEY)"
public_key="$(secret_value HETZNER_SSH_PUBLIC_KEY)"
hetzner_token_status="$(secret_status HETZNER_API_TOKEN "$hetzner_token")"
cloudflare_token_status="$(secret_status CLOUDFLARE_API_TOKEN "$cloudflare_token")"

if [[ -z "$key_name" ]]; then
  key_name="fishystuff-beta-deploy"
fi

key_pair_output="${tmp_dir}/key-pair.out"
if [[ -n "$private_key" && -n "$public_key" ]]; then
  inspect_key_pair "$private_key" "$public_key" "$tmp_dir" >"$key_pair_output"
else
  printf 'beta_deploy_ssh_private_key_status=%s\n' "$(secret_status HETZNER_SSH_PRIVATE_KEY "$private_key")" >"$key_pair_output"
  printf 'beta_deploy_ssh_public_key_status=%s\n' "$(secret_status HETZNER_SSH_PUBLIC_KEY "$public_key")" >>"$key_pair_output"
  printf 'beta_deploy_ssh_key_pair_match=unknown\n' >>"$key_pair_output"
fi
private_key_status="$(kv_value beta_deploy_ssh_private_key_status "$key_pair_output")"
public_key_status="$(kv_value beta_deploy_ssh_public_key_status "$key_pair_output")"
key_pair_match="$(kv_value beta_deploy_ssh_key_pair_match "$key_pair_output")"

credentials_status="$check_status"
if [[ "$hetzner_token_status" == "unavailable" || "$private_key_status" == "unavailable" || "$public_key_status" == "unavailable" ]]; then
  credentials_status="unavailable"
elif [[ "$check_status" == "present" && "$hetzner_token_status" == "present" && "$private_key_status" == "present" && "$public_key_status" == "present" && "$key_pair_match" != "true" ]]; then
  credentials_status="invalid"
elif [[ "$check_status" == "present" && ( "$hetzner_token_status" != "present" || "$private_key_status" != "present" || "$public_key_status" != "present" ) ]]; then
  credentials_status="missing"
fi

printf 'gitops_beta_deploy_credentials_packet_ok=true\n'
printf 'beta_deploy_credentials_status=%s\n' "$credentials_status"
printf 'beta_deploy_secretspec_profile=beta-deploy\n'
printf 'beta_deploy_active_secretspec_profile=%s\n' "$active_profile"
printf 'beta_deploy_hetzner_api_token_status=%s\n' "$hetzner_token_status"
printf 'beta_deploy_cloudflare_api_token_status=%s\n' "$cloudflare_token_status"
printf 'beta_deploy_ssh_key_name=%s\n' "$key_name"
cat "$key_pair_output"

if [[ "$credentials_status" == "present" ]]; then
  printf 'beta_deploy_credentials_next_required_action=run_key_boundary_check\n'
elif [[ "$credentials_status" == "unavailable" ]]; then
  printf 'beta_deploy_credentials_next_required_action=load_or_unlock_beta_deploy_secrets\n'
  printf 'beta_deploy_credentials_next_command_01=secretspec run --profile beta-deploy -- just gitops-beta-deploy-credentials-packet\n'
elif [[ "$credentials_status" == "invalid" ]]; then
  printf 'beta_deploy_credentials_next_required_action=repair_or_rotate_beta_deploy_key\n'
  printf 'beta_deploy_credentials_next_command_01=FISHYSTUFF_GITOPS_ENABLE_BETA_DEPLOY_KEY_GENERATE=1 FISHYSTUFF_GITOPS_BETA_DEPLOY_KEY_ROTATE=1 just gitops-beta-deploy-key-ensure\n'
else
  printf 'beta_deploy_credentials_next_required_action=generate_or_store_beta_deploy_key\n'
  printf 'beta_deploy_credentials_next_command_01=FISHYSTUFF_GITOPS_ENABLE_BETA_DEPLOY_KEY_GENERATE=1 just gitops-beta-deploy-key-ensure\n'
fi
printf 'beta_deploy_key_boundary_check_command=just deploy-key-boundary-check\n'
printf 'remote_deploy_performed=false\n'
printf 'infrastructure_mutation_performed=false\n'
printf 'local_host_mutation_performed=false\n'
