#!/usr/bin/env bash
set -euo pipefail

SCRIPT_PATH="$(readlink -f "${BASH_SOURCE[0]}")"
SCRIPT_DIR="$(cd "$(dirname "$SCRIPT_PATH")" && pwd)"
source "${SCRIPT_DIR}/lib/common.sh"

desired_state="$(normalize_named_arg desired_state "${1-/var/lib/fishystuff/gitops-beta/desired/beta-tls.desired.json}")"
unit_file="$(normalize_named_arg unit_file "${2-/etc/systemd/system/fishystuff-beta-tls-reconciler.service}")"
cloudflare_token="$(normalize_named_arg cloudflare_token "${3-/var/lib/fishystuff/gitops-beta/secrets/cloudflare-api-token}")"
tls_fullchain="$(normalize_named_arg tls_fullchain "${4-/var/lib/fishystuff/gitops-beta/tls/live/fullchain.pem}")"
tls_privkey="$(normalize_named_arg tls_privkey "${5-/var/lib/fishystuff/gitops-beta/tls/live/privkey.pem}")"
systemctl_bin="$(normalize_named_arg systemctl_bin "${6-${FISHYSTUFF_GITOPS_SYSTEMCTL_BIN:-systemctl}}")"
openssl_bin="$(normalize_named_arg openssl_bin "${7-${FISHYSTUFF_GITOPS_OPENSSL_BIN:-openssl}}")"

cd "$RECIPE_REPO_ROOT"

unit_name="fishystuff-beta-tls-reconciler.service"

require_command() {
  local command_name="$1"
  if ! command -v "$command_name" >/dev/null 2>&1; then
    echo "missing required command: ${command_name}" >&2
    exit 127
  fi
}

require_executable_or_command() {
  local command_name="$1"
  local label="$2"
  if [[ "$command_name" == */* ]]; then
    if [[ ! -x "$command_name" ]]; then
      echo "${label} is not executable: ${command_name}" >&2
      exit 127
    fi
    return
  fi
  require_command "$command_name"
}

absolute_path() {
  local path="$1"
  if [[ "$path" == /* ]]; then
    printf '%s' "$path"
    return
  fi
  printf '%s/%s' "$RECIPE_REPO_ROOT" "$path"
}

print_file_inventory() {
  local label="$1"
  local path="$2"
  local kind="missing"
  local mode=""
  local owner_group=""
  local size=""
  local sha=""

  printf '%s_path=%s\n' "$label" "$path"
  if [[ -L "$path" ]]; then
    kind="symlink"
    printf '%s_exists=true\n' "$label"
    printf '%s_type=%s\n' "$label" "$kind"
    printf '%s_symlink_target=%s\n' "$label" "$(readlink "$path")"
  elif [[ -d "$path" ]]; then
    kind="directory"
    printf '%s_exists=true\n' "$label"
    printf '%s_type=%s\n' "$label" "$kind"
  elif [[ -f "$path" ]]; then
    kind="file"
    printf '%s_exists=true\n' "$label"
    printf '%s_type=%s\n' "$label" "$kind"
  else
    printf '%s_exists=false\n' "$label"
    printf '%s_type=%s\n' "$label" "$kind"
    return
  fi

  mode="$(stat -c '%a' "$path" 2>/dev/null || true)"
  owner_group="$(stat -c '%U:%G' "$path" 2>/dev/null || true)"
  size="$(stat -c '%s' "$path" 2>/dev/null || true)"
  if [[ -n "$mode" ]]; then
    printf '%s_mode=%s\n' "$label" "$mode"
  fi
  if [[ -n "$owner_group" ]]; then
    printf '%s_owner_group=%s\n' "$label" "$owner_group"
  fi
  if [[ -n "$size" ]]; then
    printf '%s_size=%s\n' "$label" "$size"
  fi
  if [[ -f "$path" ]]; then
    read -r sha _ < <(sha256sum "$path")
    printf '%s_sha256=%s\n' "$label" "$sha"
  fi
}

unit_property() {
  local property="$1"
  local value=""

  value="$("$systemctl_bin" show "$unit_name" -p "$property" --value 2>/dev/null || true)"
  if [[ -z "$value" ]]; then
    value="unknown"
  fi
  printf '%s' "$value"
}

print_unit_shape() {
  local path="$1"
  local expected_state="$2"
  local expected_token="$3"

  if [[ ! -f "$path" ]]; then
    printf 'beta_tls_resident_unit_shape_status=missing\n'
    return
  fi

  if grep -F "EnvironmentFile=" "$path" >/dev/null; then
    printf 'beta_tls_resident_unit_has_environment_file=true\n'
  else
    printf 'beta_tls_resident_unit_has_environment_file=false\n'
  fi
  if grep -Fx "LoadCredential=cloudflare-api-token:${expected_token}" "$path" >/dev/null; then
    printf 'beta_tls_resident_unit_has_expected_token_credential=true\n'
  else
    printf 'beta_tls_resident_unit_has_expected_token_credential=false\n'
  fi
  if grep -Fx "Environment=FISHYSTUFF_GITOPS_STATE_FILE=${expected_state}" "$path" >/dev/null; then
    printf 'beta_tls_resident_unit_has_expected_state_file=true\n'
  else
    printf 'beta_tls_resident_unit_has_expected_state_file=false\n'
  fi
  if grep -F "fishystuff.fish" "$path" | grep -v -F "beta.fishystuff" >/dev/null; then
    printf 'beta_tls_resident_unit_contains_non_beta_domain=true\n'
  else
    printf 'beta_tls_resident_unit_contains_non_beta_domain=false\n'
  fi
  printf 'beta_tls_resident_unit_shape_status=checked\n'
}

print_certificate_inventory() {
  local label="$1"
  local path="$2"
  local subject=""
  local issuer=""
  local not_after=""
  local san_text=""

  print_file_inventory "$label" "$path"
  if [[ ! -f "$path" ]]; then
    printf '%s_parse_ok=false\n' "$label"
    return
  fi
  if ! "$openssl_bin" x509 -in "$path" -noout >/dev/null 2>&1; then
    printf '%s_parse_ok=false\n' "$label"
    return
  fi

  subject="$("$openssl_bin" x509 -in "$path" -noout -subject | sed 's/^subject=//')"
  issuer="$("$openssl_bin" x509 -in "$path" -noout -issuer | sed 's/^issuer=//')"
  not_after="$("$openssl_bin" x509 -in "$path" -noout -enddate | sed 's/^notAfter=//')"
  san_text="$("$openssl_bin" x509 -in "$path" -noout -ext subjectAltName 2>/dev/null || true)"
  printf '%s_parse_ok=true\n' "$label"
  printf '%s_subject=%s\n' "$label" "$subject"
  printf '%s_issuer=%s\n' "$label" "$issuer"
  printf '%s_not_after=%s\n' "$label" "$not_after"
  if "$openssl_bin" x509 -checkend 604800 -noout -in "$path" >/dev/null 2>&1; then
    printf '%s_valid_more_than_7d=true\n' "$label"
  else
    printf '%s_valid_more_than_7d=false\n' "$label"
  fi
  if "$openssl_bin" x509 -checkend 2592000 -noout -in "$path" >/dev/null 2>&1; then
    printf '%s_valid_more_than_30d=true\n' "$label"
  else
    printf '%s_valid_more_than_30d=false\n' "$label"
  fi
  for host in beta.fishystuff.fish api.beta.fishystuff.fish cdn.beta.fishystuff.fish telemetry.beta.fishystuff.fish; do
    if grep -F "DNS:${host}" <<<"$san_text" >/dev/null; then
      printf '%s_san_%s=true\n' "$label" "${host//./_}"
    else
      printf '%s_san_%s=false\n' "$label" "${host//./_}"
    fi
  done
}

print_private_key_inventory() {
  local label="$1"
  local path="$2"

  print_file_inventory "$label" "$path"
  if [[ ! -f "$path" ]]; then
    printf '%s_parse_ok=false\n' "$label"
    return
  fi
  if "$openssl_bin" pkey -in "$path" -noout -check >/dev/null 2>&1; then
    printf '%s_parse_ok=true\n' "$label"
  else
    printf '%s_parse_ok=false\n' "$label"
  fi
}

print_cert_key_match() {
  local fullchain="$1"
  local privkey="$2"
  local cert_pub_hash=""
  local key_pub_hash=""

  if [[ ! -f "$fullchain" || ! -f "$privkey" ]]; then
    printf 'beta_tls_resident_cert_key_match=unknown\n'
    return
  fi
  if ! cert_pub_hash="$("$openssl_bin" x509 -in "$fullchain" -pubkey -noout 2>/dev/null | "$openssl_bin" sha256 2>/dev/null | awk '{ print $NF }')"; then
    printf 'beta_tls_resident_cert_key_match=false\n'
    return
  fi
  if ! key_pub_hash="$("$openssl_bin" pkey -in "$privkey" -pubout 2>/dev/null | "$openssl_bin" sha256 2>/dev/null | awk '{ print $NF }')"; then
    printf 'beta_tls_resident_cert_key_match=false\n'
    return
  fi
  if [[ -n "$cert_pub_hash" && "$cert_pub_hash" == "$key_pub_hash" ]]; then
    printf 'beta_tls_resident_cert_key_match=true\n'
  else
    printf 'beta_tls_resident_cert_key_match=false\n'
  fi
}

status_summary() {
  local hostname_match="$1"
  local load_state="$2"
  local active_state="$3"
  local desired_path="$4"
  local unit_path="$5"
  local token_path="$6"
  local cert_path="$7"
  local key_path="$8"

  if [[ "$hostname_match" == "false" ]]; then
    printf 'wrong_host'
  elif [[ ! -f "$desired_path" || ! -f "$unit_path" || ! -f "$token_path" ]]; then
    printf 'pending_install'
  elif [[ "$load_state" != "loaded" ]]; then
    printf 'unit_not_loaded'
  elif [[ "$active_state" != "active" ]]; then
    printf 'unit_not_active'
  elif [[ ! -f "$cert_path" || ! -f "$key_path" ]]; then
    printf 'active_waiting_for_tls_material'
  else
    printf 'active_with_tls_material'
  fi
}

require_command sha256sum
require_command stat
require_command readlink
require_command sed
require_command grep
require_command awk
require_executable_or_command "$systemctl_bin" systemctl_bin
require_executable_or_command "$openssl_bin" openssl_bin

desired_state="$(absolute_path "$desired_state")"
unit_file="$(absolute_path "$unit_file")"
cloudflare_token="$(absolute_path "$cloudflare_token")"
tls_fullchain="$(absolute_path "$tls_fullchain")"
tls_privkey="$(absolute_path "$tls_privkey")"

active_profile="${FISHYSTUFF_OPERATOR_SECRETSPEC_PROFILE:-<not-loaded>}"
current_hostname="$(deployment_current_hostname)"
expected_hostname="$(deployment_resident_hostname beta)"
hostname_match="$(deployment_hostname_match_status "$current_hostname" "$expected_hostname")"
load_state="$(unit_property LoadState)"
active_state="$(unit_property ActiveState)"
sub_state="$(unit_property SubState)"
unit_file_state="$(unit_property UnitFileState)"
main_pid="$(unit_property MainPID)"
unit_result="$(unit_property Result)"
exec_main_status="$(unit_property ExecMainStatus)"
n_restarts="$(unit_property NRestarts)"
summary="$(status_summary "$hostname_match" "$load_state" "$active_state" "$desired_state" "$unit_file" "$cloudflare_token" "$tls_fullchain" "$tls_privkey")"

printf 'gitops_beta_tls_resident_status_packet_ok=true\n'
printf 'beta_tls_resident_status=%s\n' "$summary"
printf 'beta_tls_resident_unit_name=%s\n' "$unit_name"
printf 'beta_tls_resident_active_secretspec_profile=%s\n' "$active_profile"
printf 'beta_tls_resident_current_hostname=%s\n' "$current_hostname"
printf 'beta_tls_resident_expected_hostname=%s\n' "$expected_hostname"
printf 'beta_tls_resident_hostname_match=%s\n' "$hostname_match"
printf 'beta_tls_resident_unit_load_state=%s\n' "$load_state"
printf 'beta_tls_resident_unit_active_state=%s\n' "$active_state"
printf 'beta_tls_resident_unit_sub_state=%s\n' "$sub_state"
printf 'beta_tls_resident_unit_file_state=%s\n' "$unit_file_state"
printf 'beta_tls_resident_unit_main_pid=%s\n' "$main_pid"
printf 'beta_tls_resident_unit_result=%s\n' "$unit_result"
printf 'beta_tls_resident_unit_exec_main_status=%s\n' "$exec_main_status"
printf 'beta_tls_resident_unit_n_restarts=%s\n' "$n_restarts"

print_file_inventory beta_tls_resident_desired_state "$desired_state"
print_file_inventory beta_tls_resident_unit_file "$unit_file"
print_file_inventory beta_tls_resident_cloudflare_token "$cloudflare_token"
print_unit_shape "$unit_file" "$desired_state" "$cloudflare_token"
print_certificate_inventory beta_tls_resident_fullchain "$tls_fullchain"
print_private_key_inventory beta_tls_resident_privkey "$tls_privkey"
print_cert_key_match "$tls_fullchain" "$tls_privkey"

printf 'remote_deploy_performed=false\n'
printf 'infrastructure_mutation_performed=false\n'
printf 'local_host_mutation_performed=false\n'
