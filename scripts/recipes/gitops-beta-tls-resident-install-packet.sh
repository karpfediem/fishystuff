#!/usr/bin/env bash
set -euo pipefail

SCRIPT_PATH="$(readlink -f "${BASH_SOURCE[0]}")"
SCRIPT_DIR="$(cd "$(dirname "$SCRIPT_PATH")" && pwd)"
source "${SCRIPT_DIR}/lib/common.sh"

desired_state="$(normalize_named_arg desired_state "${1-data/gitops/beta-tls.desired.json}")"
unit_file="$(normalize_named_arg unit_file "${2-data/gitops/fishystuff-beta-tls-reconciler.service}")"
cloudflare_token_source="$(normalize_named_arg cloudflare_token_source "${3-${FISHYSTUFF_GITOPS_BETA_TLS_CLOUDFLARE_TOKEN_SOURCE:-env:CLOUDFLARE_API_TOKEN}}")"

cd "$RECIPE_REPO_ROOT"

unit_name="fishystuff-beta-tls-reconciler.service"
desired_target="/var/lib/fishystuff/gitops-beta/desired/beta-tls.desired.json"
cloudflare_token_target="/var/lib/fishystuff/gitops-beta/secrets/cloudflare-api-token"

require_command() {
  local command_name="$1"
  if ! command -v "$command_name" >/dev/null 2>&1; then
    echo "missing required command: ${command_name}" >&2
    exit 127
  fi
}

absolute_path() {
  local path="$1"
  if [[ "$path" == /* ]]; then
    printf '%s' "$path"
    return
  fi
  printf '%s/%s' "$RECIPE_REPO_ROOT" "$path"
}

kv_command_arg() {
  local name="$1"
  local value="$2"
  printf '%s=%q' "$name" "$value"
}

sha256_file() {
  local path="$1"
  local value=""
  read -r value _ < <(sha256sum "$path")
  printf '%s' "$value"
}

sha256_line() {
  local value="$1"
  printf '%s\n' "$value" | sha256sum | awk '{ print $1 }'
}

require_beta_tls_desired_shape() {
  local path="$1"
  jq -e '
    .cluster == "beta"
    and .mode == "local-apply"
    and (.tls["beta-edge"].enabled == true)
    and (.tls["beta-edge"].materialize == true)
    and (.tls["beta-edge"].solve == true)
    and (.tls["beta-edge"].present_dns == true)
    and (.tls["beta-edge"].certificate_name == "fishystuff-beta-edge")
    and (.tls["beta-edge"].dns_provider == "cloudflare")
    and (.tls["beta-edge"].dns_zone == "fishystuff.fish")
    and ((.tls["beta-edge"].domains | sort) == [
      "api.beta.fishystuff.fish",
      "beta.fishystuff.fish",
      "cdn.beta.fishystuff.fish",
      "telemetry.beta.fishystuff.fish"
    ])
    and (.tls["beta-edge"].request_namespace == "acme/cert-requests/fishystuff-beta")
    and (.tls["beta-edge"].tls_dir == "/var/lib/fishystuff/gitops-beta/tls/live")
    and (.tls["beta-edge"].fullchain_path == "/var/lib/fishystuff/gitops-beta/tls/live/fullchain.pem")
    and (.tls["beta-edge"].cloudflare_token_env == "CLOUDFLARE_API_TOKEN")
    and (.tls["beta-edge"].reload_service == "fishystuff-beta-edge")
    and (.tls["beta-edge"].reload_service_action == "reload-or-try-restart")
  ' "$path" >/dev/null
}

require_beta_tls_unit_shape() {
  local path="$1"
  if grep -F "EnvironmentFile=" "$path" >/dev/null; then
    echo "beta TLS resident unit must use LoadCredential, not EnvironmentFile" >&2
    exit 2
  fi
  if grep -F "fishystuff.fish" "$path" | grep -v -F "beta.fishystuff" >/dev/null; then
    echo "beta TLS resident unit contains a non-beta production hostname" >&2
    exit 2
  fi
  grep -Fx "Description=FishyStuff beta GitOps TLS ACME reconciler" "$path" >/dev/null
  grep -Fx "Environment=FISHYSTUFF_GITOPS_ENABLE_LOCAL_APPLY=1" "$path" >/dev/null
  grep -Fx "Environment=FISHYSTUFF_GITOPS_STATE_FILE=${desired_target}" "$path" >/dev/null
  grep -Fx "LoadCredential=cloudflare-api-token:${cloudflare_token_target}" "$path" >/dev/null
  grep -F "CLOUDFLARE_API_TOKEN=\"\$(cat \"\$CREDENTIALS_DIRECTORY/cloudflare-api-token\")\"" "$path" >/dev/null
  grep -Fx "ReadWritePaths=/var/lib/fishystuff/gitops-beta" "$path" >/dev/null
  grep -Fx "WantedBy=multi-user.target" "$path" >/dev/null
}

require_command jq
require_command sha256sum

active_profile="${FISHYSTUFF_OPERATOR_SECRETSPEC_PROFILE:-<not-loaded>}"
current_hostname="$(deployment_current_hostname)"
expected_hostname="$(deployment_resident_hostname beta)"
hostname_match="$(deployment_hostname_match_status "$current_hostname" "$expected_hostname")"
desired_state_path="$(absolute_path "$desired_state")"
unit_file_path="$(absolute_path "$unit_file")"
cloudflare_token_source_path=""
if [[ -n "$cloudflare_token_source" && "$cloudflare_token_source" != env:* ]]; then
  cloudflare_token_source_path="$(absolute_path "$cloudflare_token_source")"
fi

printf 'gitops_beta_tls_resident_install_packet_ok=true\n'
printf 'beta_tls_resident_install_packet_unit_name=%s\n' "$unit_name"
printf 'beta_tls_resident_install_packet_desired_state=%s\n' "$desired_state"
printf 'beta_tls_resident_install_packet_unit_file=%s\n' "$unit_file"
printf 'beta_tls_resident_install_packet_cloudflare_token_source=%s\n' "${cloudflare_token_source:-<missing>}"
printf 'beta_tls_resident_install_packet_desired_target=%s\n' "$desired_target"
printf 'beta_tls_resident_install_packet_cloudflare_token_target=%s\n' "$cloudflare_token_target"
printf 'beta_tls_resident_install_packet_active_secretspec_profile=%s\n' "$active_profile"
printf 'beta_tls_resident_install_packet_current_hostname=%s\n' "$current_hostname"
printf 'beta_tls_resident_install_packet_expected_hostname=%s\n' "$expected_hostname"
printf 'beta_tls_resident_install_packet_hostname_match=%s\n' "$hostname_match"
printf 'remote_deploy_performed=false\n'
printf 'infrastructure_mutation_performed=false\n'
printf 'local_host_mutation_performed=false\n'

case "$active_profile" in
  production-deploy | prod-deploy | production)
    printf 'beta_tls_resident_install_packet_status=blocked_profile\n'
    printf 'beta_tls_resident_install_packet_next_required_action=load_beta_deploy_or_no_operator_profile\n'
    exit 0
    ;;
esac

missing_inputs=0
if [[ ! -f "$desired_state_path" ]]; then
  printf 'beta_tls_resident_install_packet_desired_state_status=missing\n'
  printf 'beta_tls_resident_install_packet_next_command_01=just gitops-beta-tls-desired output=%s\n' "$desired_state"
  missing_inputs=1
else
  require_beta_tls_desired_shape "$desired_state_path"
  desired_sha256="$(sha256_file "$desired_state_path")"
  printf 'beta_tls_resident_install_packet_desired_state_status=ready\n'
  printf 'beta_tls_resident_install_packet_desired_sha256=%s\n' "$desired_sha256"
fi

if [[ ! -f "$unit_file_path" ]]; then
  printf 'beta_tls_resident_install_packet_unit_file_status=missing\n'
  printf 'beta_tls_resident_install_packet_next_command_02=just gitops-beta-tls-resident-unit output=%s\n' "$unit_file"
  missing_inputs=1
else
  require_beta_tls_unit_shape "$unit_file_path"
  unit_sha256="$(sha256_file "$unit_file_path")"
  printf 'beta_tls_resident_install_packet_unit_file_status=ready\n'
  printf 'beta_tls_resident_install_packet_unit_sha256=%s\n' "$unit_sha256"
fi

case "$cloudflare_token_source" in
  env:CLOUDFLARE_API_TOKEN)
    if [[ -z "${CLOUDFLARE_API_TOKEN:-}" ]]; then
      printf 'beta_tls_resident_install_packet_cloudflare_token_status=missing_env\n'
      printf 'beta_tls_resident_install_packet_next_required_action=load_beta_deploy_cloudflare_token\n'
      printf 'beta_tls_resident_install_packet_next_command_03=secretspec run --profile beta-deploy -- just gitops-beta-tls-resident-install-packet desired_state=%s unit_file=%s cloudflare_token_source=env:CLOUDFLARE_API_TOKEN\n' "$desired_state" "$unit_file"
      missing_inputs=1
    else
      token_sha256="$(sha256_line "$CLOUDFLARE_API_TOKEN")"
      printf 'beta_tls_resident_install_packet_cloudflare_token_status=ready_env\n'
      printf 'beta_tls_resident_install_packet_cloudflare_token_sha256=%s\n' "$token_sha256"
    fi
    ;;
  env:*)
    echo "unsupported beta TLS Cloudflare token env source: ${cloudflare_token_source}" >&2
    exit 2
    ;;
  "")
    printf 'beta_tls_resident_install_packet_cloudflare_token_status=missing_source\n'
    printf 'beta_tls_resident_install_packet_next_required_action=choose_cloudflare_token_source\n'
    missing_inputs=1
    ;;
  *)
    if [[ ! -f "$cloudflare_token_source_path" ]]; then
      printf 'beta_tls_resident_install_packet_cloudflare_token_status=missing_file\n'
      printf 'beta_tls_resident_install_packet_next_required_action=write_or_select_cloudflare_token_source_file\n'
      missing_inputs=1
    else
      token_sha256="$(sha256_file "$cloudflare_token_source_path")"
      printf 'beta_tls_resident_install_packet_cloudflare_token_status=ready_file\n'
      printf 'beta_tls_resident_install_packet_cloudflare_token_sha256=%s\n' "$token_sha256"
    fi
    ;;
esac

if [[ "$missing_inputs" != "0" ]]; then
  printf 'beta_tls_resident_install_packet_status=pending_inputs\n'
  exit 0
fi

if [[ "$hostname_match" != "true" ]]; then
  printf 'beta_tls_resident_install_packet_status=blocked_host\n'
  printf 'beta_tls_resident_install_packet_next_required_action=run_on_beta_resident_host\n'
  exit 0
fi

desired_arg="$(kv_command_arg desired_state "$desired_state")"
unit_arg="$(kv_command_arg unit_file "$unit_file")"
token_arg="$(kv_command_arg cloudflare_token_source "$cloudflare_token_source")"
printf 'beta_tls_resident_install_packet_status=ready\n'
printf 'beta_tls_resident_install_packet_next_required_action=run_guarded_install\n'
printf 'beta_tls_resident_install_packet_next_command_01=FISHYSTUFF_GITOPS_ENABLE_BETA_TLS_RESIDENT_INSTALL=1 FISHYSTUFF_GITOPS_ENABLE_BETA_TLS_RESIDENT_RESTART=1 FISHYSTUFF_GITOPS_BETA_TLS_DESIRED_SHA256=%s FISHYSTUFF_GITOPS_BETA_TLS_RESIDENT_UNIT_SHA256=%s FISHYSTUFF_GITOPS_BETA_TLS_CLOUDFLARE_TOKEN_SHA256=%s just gitops-beta-install-tls-resident %s %s %s\n' \
  "$desired_sha256" \
  "$unit_sha256" \
  "$token_sha256" \
  "$desired_arg" \
  "$unit_arg" \
  "$token_arg"
