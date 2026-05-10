#!/usr/bin/env bash
set -euo pipefail

SCRIPT_PATH="$(readlink -f "${BASH_SOURCE[0]}")"
SCRIPT_DIR="$(cd "$(dirname "$SCRIPT_PATH")" && pwd)"
source "${SCRIPT_DIR}/lib/common.sh"

state_file_arg="$(normalize_named_arg state_file "${1-data/gitops/beta-tls.staging.desired.json}")"
ca="$(normalize_named_arg ca "${2-staging}")"
contact_email="$(normalize_named_arg contact_email "${3-${FISHYSTUFF_GITOPS_BETA_ACME_CONTACT_EMAIL:-}}")"
if [[ -z "$contact_email" ]]; then
  contact_email="${FISHYSTUFF_GITOPS_BETA_ACME_CONTACT_EMAIL:-}"
fi

cd "$RECIPE_REPO_ROOT"

require_command() {
  local name="$1"
  if ! command -v "$name" >/dev/null 2>&1; then
    echo "$name is required" >&2
    exit 127
  fi
}

state_file_path() {
  local path="$1"
  if [[ "$path" == /* ]]; then
    printf '%s' "$path"
    return
  fi
  printf '%s/%s' "$RECIPE_REPO_ROOT" "$path"
}

acme_directory_url() {
  case "$1" in
    staging)
      printf '%s' "https://acme-staging-v02.api.letsencrypt.org/directory"
      ;;
    production)
      printf '%s' "https://acme-v02.api.letsencrypt.org/directory"
      ;;
    *)
      echo "ca must be staging or production, got: $1" >&2
      exit 2
      ;;
  esac
}

secret_status() {
  local name="$1"
  local value="$2"
  if [[ -e "${tmp_dir}/secret-${name}.unavailable" ]]; then
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

require_beta_tls_shape() {
  local state_file="$1"
  local directory_url="$2"

  jq -e \
    --arg directory_url "$directory_url" \
    '
      .cluster == "beta"
      and .mode == "local-apply"
      and (.tls["beta-edge"].enabled == true)
      and (.tls["beta-edge"].materialize == true)
      and (.tls["beta-edge"].solve == true)
      and (.tls["beta-edge"].present_dns == true)
      and (.tls["beta-edge"].certificate_name == "fishystuff-beta-edge")
      and (.tls["beta-edge"].account_name == "fishystuff-beta-edge-account")
      and (.tls["beta-edge"].directory_url == $directory_url)
      and (.tls["beta-edge"].challenge == "dns-01")
      and (.tls["beta-edge"].dns_provider == "cloudflare")
      and (.tls["beta-edge"].dns_zone == "fishystuff.fish")
      and ((.tls["beta-edge"].domains | sort) == [
        "api.beta.fishystuff.fish",
        "beta.fishystuff.fish",
        "cdn.beta.fishystuff.fish",
        "telemetry.beta.fishystuff.fish"
      ])
      and all(.tls["beta-edge"].domains[]; . == "beta.fishystuff.fish" or endswith(".beta.fishystuff.fish"))
      and (.tls["beta-edge"].request_namespace == "acme/cert-requests/fishystuff-beta")
      and (.tls["beta-edge"].account_key_path == "/var/lib/fishystuff/gitops-beta/acme/fishystuff-beta-edge-account/account.key")
      and (.tls["beta-edge"].account_cache_dir == "/var/lib/fishystuff/gitops-beta/acme/fishystuff-beta-edge-account")
      and (.tls["beta-edge"].tls_dir == "/var/lib/fishystuff/gitops-beta/tls/live")
      and (.tls["beta-edge"].key_path == "/var/lib/fishystuff/gitops-beta/tls/live/privkey.pem")
      and (.tls["beta-edge"].cert_path == "/var/lib/fishystuff/gitops-beta/tls/live/cert.pem")
      and (.tls["beta-edge"].chain_path == "/var/lib/fishystuff/gitops-beta/tls/live/chain.pem")
      and (.tls["beta-edge"].fullchain_path == "/var/lib/fishystuff/gitops-beta/tls/live/fullchain.pem")
      and (.tls["beta-edge"].cloudflare_token_env == "CLOUDFLARE_API_TOKEN")
    ' "$state_file" >/dev/null
}

active_profile="${FISHYSTUFF_OPERATOR_SECRETSPEC_PROFILE:-<not-loaded>}"
case "$active_profile" in
  production-deploy | prod-deploy | production)
    echo "gitops-beta-tls-reconcile-packet must not run with production SecretSpec profile active: ${active_profile}" >&2
    exit 2
    ;;
esac

require_command awk
require_command jq
require_command mktemp

tmp_dir="$(mktemp -d)"
cleanup() {
  rm -rf "$tmp_dir"
}
trap cleanup EXIT

state_file="$(state_file_path "$state_file_arg")"
directory_url="$(acme_directory_url "$ca")"

printf 'gitops_beta_tls_reconcile_packet_ok=true\n'
printf 'beta_tls_packet_state_file=%s\n' "$state_file_arg"
printf 'beta_tls_packet_ca=%s\n' "$ca"
printf 'beta_tls_packet_directory_url=%s\n' "$directory_url"
printf 'beta_tls_packet_tls_dir=/var/lib/fishystuff/gitops-beta/tls/live\n'
printf 'beta_tls_packet_fullchain_path=/var/lib/fishystuff/gitops-beta/tls/live/fullchain.pem\n'
printf 'beta_tls_packet_privkey_path=/var/lib/fishystuff/gitops-beta/tls/live/privkey.pem\n'
printf 'beta_tls_packet_domains=beta.fishystuff.fish,api.beta.fishystuff.fish,cdn.beta.fishystuff.fish,telemetry.beta.fishystuff.fish\n'
printf 'beta_tls_packet_mgmt_flake=%s\n' "${FISHYSTUFF_GITOPS_MGMT_FLAKE:-${RECIPE_REPO_ROOT}#mgmt-gitops}"

if [[ ! -f "$state_file" ]]; then
  printf 'beta_tls_packet_status=missing_desired_state\n'
  if [[ -n "$contact_email" ]]; then
    printf 'beta_tls_packet_contact_email_status=present\n'
    printf 'beta_tls_packet_next_required_action=write_tls_desired_state\n'
    printf 'beta_tls_packet_next_command_01=just gitops-beta-tls-desired output=%s ca=%s contact_email=%s\n' "$state_file_arg" "$ca" "$contact_email"
  else
    printf 'beta_tls_packet_contact_email_status=missing\n'
    printf 'beta_tls_packet_next_required_action=choose_acme_contact_email\n'
    printf 'beta_tls_packet_next_command_01=FISHYSTUFF_GITOPS_BETA_ACME_CONTACT_EMAIL=<email> just gitops-beta-tls-desired output=%s ca=%s\n' "$state_file_arg" "$ca"
  fi
  printf 'remote_deploy_performed=false\n'
  printf 'infrastructure_mutation_performed=false\n'
  printf 'local_host_mutation_performed=false\n'
  exit 0
fi

if ! require_beta_tls_shape "$state_file" "$directory_url"; then
  echo "beta TLS desired state does not match the guarded beta ${ca} ACME shape: ${state_file_arg}" >&2
  exit 2
fi

require_command secretspec

check_status="missing"
if secretspec check --profile beta-deploy --no-prompt >"${tmp_dir}/secretspec-check.out" 2>"${tmp_dir}/secretspec-check.err"; then
  check_status="present"
elif grep -E 'DBus error|secure storage|Operation not permitted|permission denied' "${tmp_dir}/secretspec-check.err" >/dev/null; then
  check_status="unavailable"
fi

cloudflare_token="${CLOUDFLARE_API_TOKEN:-}"
if [[ -z "$cloudflare_token" ]]; then
  cloudflare_token="$(secret_value CLOUDFLARE_API_TOKEN)"
fi
cloudflare_token_status="$(secret_status CLOUDFLARE_API_TOKEN "$cloudflare_token")"
if [[ "$check_status" == "unavailable" ]]; then
  cloudflare_token_status="unavailable"
fi

printf 'beta_tls_packet_status=%s\n' "$(if [[ "$cloudflare_token_status" == "present" ]]; then printf 'ready'; else printf 'blocked_credentials'; fi)"
printf 'beta_tls_packet_desired_state_status=ready\n'
printf 'beta_tls_packet_secretspec_profile=beta-deploy\n'
printf 'beta_tls_packet_active_secretspec_profile=%s\n' "$active_profile"
printf 'beta_tls_packet_secretspec_status=%s\n' "$check_status"
printf 'beta_tls_packet_cloudflare_api_token_status=%s\n' "$cloudflare_token_status"
printf 'beta_tls_packet_unify_command=just gitops-unify auto %s\n' "$state_file_arg"

if [[ "$cloudflare_token_status" != "present" ]]; then
  printf 'beta_tls_packet_next_required_action=load_or_unlock_beta_deploy_secrets\n'
  printf 'beta_tls_packet_next_command_01=secretspec run --profile beta-deploy -- just gitops-beta-tls-reconcile-packet state_file=%s ca=%s\n' "$state_file_arg" "$ca"
elif [[ "$ca" == "production" ]]; then
  printf 'beta_tls_packet_next_required_action=review_and_run_production_acme_reconcile\n'
  printf 'beta_tls_packet_next_command_01=FISHYSTUFF_OPERATOR_SECRETSPEC_PROFILE=beta-deploy FISHYSTUFF_GITOPS_ENABLE_BETA_TLS_APPLY=1 FISHYSTUFF_GITOPS_ENABLE_LOCAL_APPLY=1 FISHYSTUFF_GITOPS_ENABLE_BETA_TLS_PRODUCTION_ACME=1 secretspec run --profile beta-deploy -- just gitops-beta-reconcile-tls state_file=%s ca=production\n' "$state_file_arg"
else
  printf 'beta_tls_packet_next_required_action=run_staging_acme_reconcile\n'
  printf 'beta_tls_packet_next_command_01=FISHYSTUFF_OPERATOR_SECRETSPEC_PROFILE=beta-deploy FISHYSTUFF_GITOPS_ENABLE_BETA_TLS_APPLY=1 FISHYSTUFF_GITOPS_ENABLE_LOCAL_APPLY=1 secretspec run --profile beta-deploy -- just gitops-beta-reconcile-tls state_file=%s ca=staging\n' "$state_file_arg"
fi

printf 'remote_deploy_performed=false\n'
printf 'infrastructure_mutation_performed=false\n'
printf 'local_host_mutation_performed=false\n'
