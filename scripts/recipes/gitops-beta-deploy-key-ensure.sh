#!/usr/bin/env bash
set -euo pipefail

SCRIPT_PATH="$(readlink -f "${BASH_SOURCE[0]}")"
SCRIPT_DIR="$(cd "$(dirname "$SCRIPT_PATH")" && pwd)"
source "${SCRIPT_DIR}/lib/common.sh"

key_comment="$(normalize_named_arg key_comment "${1-fishystuff-beta-deploy}")"
key_name="$(normalize_named_arg key_name "${2-fishystuff-beta-deploy}")"

cd "$RECIPE_REPO_ROOT"

require_command() {
  local command_name="$1"
  if ! command -v "$command_name" >/dev/null 2>&1; then
    echo "missing required command: ${command_name}" >&2
    exit 127
  fi
}

require_env_value() {
  local name="$1"
  local expected="$2"
  local value="${!name-}"

  if [[ "$value" != "$expected" ]]; then
    echo "gitops-beta-deploy-key-ensure requires ${name}=${expected}" >&2
    exit 2
  fi
}

packet_value() {
  local key="$1"
  local file="$2"
  awk -F= -v key="$key" '$1 == key { print substr($0, length(key) + 2); exit }' "$file"
}

active_profile="${FISHYSTUFF_OPERATOR_SECRETSPEC_PROFILE:-}"
case "$active_profile" in
  production-deploy | prod-deploy | production)
    echo "gitops-beta-deploy-key-ensure must not run with production SecretSpec profile active: ${active_profile}" >&2
    exit 2
    ;;
esac

require_command awk
require_command mktemp
require_command secretspec
require_command ssh-keygen
require_env_value FISHYSTUFF_GITOPS_ENABLE_BETA_DEPLOY_KEY_GENERATE 1

tmp_dir="$(mktemp -d)"
cleanup() {
  rm -rf "$tmp_dir"
}
trap cleanup EXIT

before_packet="${tmp_dir}/before.packet"
bash scripts/recipes/gitops-beta-deploy-credentials-packet.sh >"$before_packet"
before_credentials_status="$(packet_value beta_deploy_credentials_status "$before_packet")"
before_private_status="$(packet_value beta_deploy_ssh_private_key_status "$before_packet")"
before_public_status="$(packet_value beta_deploy_ssh_public_key_status "$before_packet")"
before_pair_match="$(packet_value beta_deploy_ssh_key_pair_match "$before_packet")"

if [[ "$before_credentials_status" == "unavailable" ]]; then
  echo "beta deploy credentials are unavailable; load or unlock the beta-deploy SecretSpec provider before generating keys" >&2
  exit 2
fi

if [[ "${FISHYSTUFF_GITOPS_BETA_DEPLOY_KEY_ROTATE:-}" != "1" \
  && "$before_private_status" == "present" \
  && "$before_public_status" == "present" \
  && "$before_pair_match" == "true" ]]; then
  printf 'gitops_beta_deploy_key_ensure_ok=already_present\n'
  printf 'beta_deploy_key_ensure_action=none\n'
  printf 'beta_deploy_key_ensure_public_key_fingerprint=%s\n' "$(packet_value beta_deploy_ssh_public_key_fingerprint "$before_packet")"
  printf 'remote_deploy_performed=false\n'
  printf 'infrastructure_mutation_performed=false\n'
  printf 'local_host_mutation_performed=false\n'
  exit 0
fi

if [[ "${FISHYSTUFF_GITOPS_BETA_DEPLOY_KEY_ROTATE:-}" != "1" \
  && "$before_private_status" == "present" \
  && "$before_public_status" == "present" \
  && "$before_pair_match" != "true" ]]; then
  echo "existing beta deploy key material is present but does not validate; set FISHYSTUFF_GITOPS_BETA_DEPLOY_KEY_ROTATE=1 to overwrite it" >&2
  exit 2
fi

key_path="${tmp_dir}/fishystuff-beta-deploy"
ssh-keygen -q -t ed25519 -a 64 -N "" -C "$key_comment" -f "$key_path"
private_key="$(<"$key_path")"
public_key="$(<"${key_path}.pub")"

secretspec set --profile beta-deploy HETZNER_SSH_PRIVATE_KEY "$private_key" >/dev/null
secretspec set --profile beta-deploy HETZNER_SSH_PUBLIC_KEY "$public_key" >/dev/null
secretspec set --profile beta-deploy HETZNER_SSH_KEY_NAME "$key_name" >/dev/null

after_packet="${tmp_dir}/after.packet"
bash scripts/recipes/gitops-beta-deploy-credentials-packet.sh >"$after_packet"
after_private_status="$(packet_value beta_deploy_ssh_private_key_status "$after_packet")"
after_public_status="$(packet_value beta_deploy_ssh_public_key_status "$after_packet")"
after_pair_match="$(packet_value beta_deploy_ssh_key_pair_match "$after_packet")"

if [[ "$after_private_status" != "present" || "$after_public_status" != "present" || "$after_pair_match" != "true" ]]; then
  echo "generated beta deploy key was not stored as a valid key pair" >&2
  cat "$after_packet" >&2
  exit 2
fi

printf 'gitops_beta_deploy_key_ensure_ok=stored\n'
printf 'beta_deploy_key_ensure_action=generated_and_stored\n'
printf 'beta_deploy_key_ensure_key_name=%s\n' "$key_name"
printf 'beta_deploy_key_ensure_public_key_fingerprint=%s\n' "$(packet_value beta_deploy_ssh_public_key_fingerprint "$after_packet")"
printf 'remote_deploy_performed=false\n'
printf 'infrastructure_mutation_performed=false\n'
printf 'local_host_mutation_performed=true\n'
printf 'hetzner_key_upload_performed=false\n'
