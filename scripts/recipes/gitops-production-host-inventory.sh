#!/usr/bin/env bash
set -euo pipefail

SCRIPT_PATH="$(readlink -f "${BASH_SOURCE[0]}")"
SCRIPT_DIR="$(cd "$(dirname "$SCRIPT_PATH")" && pwd)"
source "${SCRIPT_DIR}/lib/common.sh"

state_dir="$(normalize_named_arg state_dir "${1-/var/lib/fishystuff/gitops}")"
run_dir="$(normalize_named_arg run_dir "${2-/run/fishystuff/gitops}")"
edge_bundle="$(normalize_named_arg edge_bundle "${3-auto}")"
systemd_unit_path="$(normalize_named_arg systemd_unit_path "${4-/etc/systemd/system/fishystuff-edge.service}")"
tls_fullchain_path="$(normalize_named_arg tls_fullchain_path "${5-/run/fishystuff/edge/tls/fullchain.pem}")"
tls_privkey_path="$(normalize_named_arg tls_privkey_path "${6-/run/fishystuff/edge/tls/privkey.pem}")"
environment="$(normalize_named_arg environment "${7-production}")"

cd "$RECIPE_REPO_ROOT"

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

print_file_inventory() {
  local label="$1"
  local path="$2"
  local kind="missing"
  local real_path=""
  local target=""
  local mode=""
  local owner_group=""
  local size=""
  local mtime=""
  local sha=""

  printf '%s_path=%s\n' "$label" "$path"
  if [[ -L "$path" ]]; then
    kind="symlink"
    target="$(readlink "$path")"
    printf '%s_exists=true\n' "$label"
    printf '%s_type=%s\n' "$label" "$kind"
    printf '%s_symlink_target=%s\n' "$label" "$target"
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

  real_path="$(readlink -f "$path" 2>/dev/null || true)"
  if [[ -n "$real_path" ]]; then
    printf '%s_realpath=%s\n' "$label" "$real_path"
  fi
  mode="$(stat -c '%a' "$path" 2>/dev/null || true)"
  owner_group="$(stat -c '%U:%G' "$path" 2>/dev/null || true)"
  size="$(stat -c '%s' "$path" 2>/dev/null || true)"
  mtime="$(stat -c '%Y' "$path" 2>/dev/null || true)"
  if [[ -n "$mode" ]]; then
    printf '%s_mode=%s\n' "$label" "$mode"
  fi
  if [[ -n "$owner_group" ]]; then
    printf '%s_owner_group=%s\n' "$label" "$owner_group"
  fi
  if [[ -n "$size" ]]; then
    printf '%s_size=%s\n' "$label" "$size"
  fi
  if [[ -n "$mtime" ]]; then
    printf '%s_mtime_epoch=%s\n' "$label" "$mtime"
  fi
  if [[ -f "$path" ]]; then
    read -r sha _ < <(sha256sum "$path")
    printf '%s_sha256=%s\n' "$label" "$sha"
  fi
}

print_json_field() {
  local label="$1"
  local path="$2"
  local filter="$3"
  local value=""

  if [[ ! -f "$path" ]]; then
    return
  fi
  value="$(jq -cer "$filter" "$path" 2>/dev/null || true)"
  if [[ -n "$value" && "$value" != "null" ]]; then
    printf '%s=%s\n' "$label" "$value"
  fi
}

print_certificate_inventory() {
  local label="$1"
  local path="$2"
  local subject=""
  local issuer=""
  local serial=""
  local not_before=""
  local not_after=""
  local fingerprint=""

  print_file_inventory "$label" "$path"
  if [[ ! -f "$path" ]]; then
    printf '%s_parse_ok=false\n' "$label"
    return
  fi
  if ! openssl x509 -in "$path" -noout >/dev/null 2>&1; then
    printf '%s_parse_ok=false\n' "$label"
    return
  fi

  subject="$(openssl x509 -in "$path" -noout -subject | sed 's/^subject=//')"
  issuer="$(openssl x509 -in "$path" -noout -issuer | sed 's/^issuer=//')"
  serial="$(openssl x509 -in "$path" -noout -serial | sed 's/^serial=//')"
  not_before="$(openssl x509 -in "$path" -noout -startdate | sed 's/^notBefore=//')"
  not_after="$(openssl x509 -in "$path" -noout -enddate | sed 's/^notAfter=//')"
  fingerprint="$(openssl x509 -in "$path" -noout -fingerprint -sha256 | sed 's/^sha256 Fingerprint=//;s/^SHA256 Fingerprint=//')"
  printf '%s_parse_ok=true\n' "$label"
  printf '%s_subject=%s\n' "$label" "$subject"
  printf '%s_issuer=%s\n' "$label" "$issuer"
  printf '%s_serial=%s\n' "$label" "$serial"
  printf '%s_not_before=%s\n' "$label" "$not_before"
  printf '%s_not_after=%s\n' "$label" "$not_after"
  printf '%s_sha256_fingerprint=%s\n' "$label" "$fingerprint"
}

print_private_key_inventory() {
  local label="$1"
  local path="$2"

  print_file_inventory "$label" "$path"
  if [[ ! -f "$path" ]]; then
    printf '%s_parse_ok=false\n' "$label"
    return
  fi
  if openssl pkey -in "$path" -noout -check >/dev/null 2>&1; then
    printf '%s_parse_ok=true\n' "$label"
  else
    printf '%s_parse_ok=false\n' "$label"
  fi
}

require_command jq
require_command sha256sum
require_command stat
require_command readlink
require_command openssl
require_command awk
require_command grep
require_command sed
require_command head
require_command cmp

if [[ -z "$environment" ]]; then
  echo "environment must not be empty" >&2
  exit 2
fi

state_dir="$(absolute_path "$state_dir")"
run_dir="$(absolute_path "$run_dir")"
systemd_unit_path="$(absolute_path "$systemd_unit_path")"
tls_fullchain_path="$(absolute_path "$tls_fullchain_path")"
tls_privkey_path="$(absolute_path "$tls_privkey_path")"

status_path="${state_dir%/}/status/${environment}.json"
active_path="${state_dir%/}/active/${environment}.json"
rollback_set_path="${state_dir%/}/rollback-set/${environment}.json"
rollback_path="${state_dir%/}/rollback/${environment}.json"
admission_path="${run_dir%/}/admission/${environment}.json"
route_path="${run_dir%/}/routes/${environment}.json"
roots_dir="${run_dir%/}/roots"
served_site_link="${state_dir%/}/served/${environment}/site"
served_cdn_link="${state_dir%/}/served/${environment}/cdn"

edge_output="$(mktemp)"
edge_error="$(mktemp)"
cleanup() {
  rm -f "$edge_output" "$edge_error"
}
trap cleanup EXIT

edge_bundle_check_ok="skipped"
edge_bundle_path=""
edge_caddy_validate=""
edge_caddyfile_store=""
edge_executable_store=""
edge_systemd_unit_store=""
edge_systemd_unit_artifact=""

if [[ -n "$edge_bundle" && "$edge_bundle" != "skip" ]]; then
  if bash scripts/recipes/gitops-check-edge-handoff-bundle.sh "$edge_bundle" >"$edge_output" 2>"$edge_error"; then
    edge_bundle_check_ok="true"
    edge_bundle_path="$(awk -F= '$1 == "gitops_edge_handoff_bundle_ok" { print $2 }' "$edge_output")"
    edge_caddy_validate="$(awk -F= '$1 == "gitops_edge_handoff_caddy_validate" { print $2 }' "$edge_output")"
    edge_caddyfile_store="$(awk -F= '$1 == "gitops_edge_handoff_caddyfile_store" { print $2 }' "$edge_output")"
    edge_executable_store="$(awk -F= '$1 == "gitops_edge_handoff_executable_store" { print $2 }' "$edge_output")"
    edge_systemd_unit_store="$(awk -F= '$1 == "gitops_edge_handoff_systemd_unit_store" { print $2 }' "$edge_output")"
    edge_systemd_unit_artifact="$(awk -F= '$1 == "gitops_edge_handoff_systemd_unit" { print $2 }' "$edge_output")"
  else
    echo "production GitOps edge bundle inventory check failed" >&2
    cat "$edge_error" >&2
    exit 2
  fi
fi

printf 'gitops_production_host_inventory_ok=%s\n' "$environment"
printf 'environment=%s\n' "$environment"
printf 'state_dir=%s\n' "$state_dir"
printf 'run_dir=%s\n' "$run_dir"
printf 'edge_bundle_check_ok=%s\n' "$edge_bundle_check_ok"
if [[ "$edge_bundle_check_ok" == "true" ]]; then
  printf 'edge_bundle=%s\n' "$edge_bundle_path"
  printf 'edge_caddy_validate=%s\n' "$edge_caddy_validate"
  printf 'edge_caddyfile_store=%s\n' "$edge_caddyfile_store"
  printf 'edge_executable_store=%s\n' "$edge_executable_store"
  printf 'edge_systemd_unit_store=%s\n' "$edge_systemd_unit_store"
fi

print_file_inventory status "$status_path"
print_json_field status_desired_generation "$status_path" '.desired_generation'
print_json_field status_host "$status_path" '.host'
print_json_field status_release_id "$status_path" '.release_id'
print_json_field status_phase "$status_path" '.phase'
print_json_field status_admission_state "$status_path" '.admission_state'
print_json_field status_rollback_available "$status_path" '.rollback_available'

print_file_inventory active "$active_path"
print_json_field active_desired_generation "$active_path" '.desired_generation'
print_json_field active_host "$active_path" '.host'
print_json_field active_release_id "$active_path" '.release_id'
print_json_field active_api_upstream "$active_path" '.api_upstream'

print_file_inventory rollback_set "$rollback_set_path"
print_json_field rollback_set_current_release_id "$rollback_set_path" '.current_release_id'
print_json_field rollback_set_retained_release_count "$rollback_set_path" '.retained_release_count'
print_json_field rollback_set_retained_release_ids "$rollback_set_path" '.retained_release_ids'

print_file_inventory rollback "$rollback_path"
print_json_field rollback_release_id "$rollback_path" '.rollback_release_id'
print_json_field rollback_available "$rollback_path" '.rollback_available'

print_file_inventory admission "$admission_path"
print_json_field admission_release_id "$admission_path" '.release_id'
print_json_field admission_state "$admission_path" '.admission_state'
print_json_field admission_url "$admission_path" '.url'

print_file_inventory route "$route_path"
print_json_field route_release_id "$route_path" '.release_id'
print_json_field route_api_upstream "$route_path" '.api_upstream'
print_json_field route_state "$route_path" '.state'

print_file_inventory roots_dir "$roots_dir"
print_file_inventory served_site_link "$served_site_link"
print_file_inventory served_cdn_link "$served_cdn_link"

print_file_inventory installed_edge_unit "$systemd_unit_path"
if [[ -f "$systemd_unit_path" ]]; then
  installed_execstart="$(grep -E '^ExecStart=' "$systemd_unit_path" | head -n 1 || true)"
  installed_execreload="$(grep -E '^ExecReload=' "$systemd_unit_path" | head -n 1 || true)"
  installed_fullchain_credential="$(grep -E '^LoadCredential=fullchain.pem:' "$systemd_unit_path" | head -n 1 || true)"
  installed_privkey_credential="$(grep -E '^LoadCredential=privkey.pem:' "$systemd_unit_path" | head -n 1 || true)"
  printf 'installed_edge_unit_execstart=%s\n' "$installed_execstart"
  printf 'installed_edge_unit_execreload=%s\n' "$installed_execreload"
  printf 'installed_edge_unit_fullchain_credential=%s\n' "$installed_fullchain_credential"
  printf 'installed_edge_unit_privkey_credential=%s\n' "$installed_privkey_credential"
  if [[ -n "$edge_systemd_unit_artifact" && -f "$edge_systemd_unit_artifact" ]]; then
    if cmp -s "$systemd_unit_path" "$edge_systemd_unit_artifact"; then
      printf 'installed_edge_unit_matches_bundle=true\n'
    else
      printf 'installed_edge_unit_matches_bundle=false\n'
    fi
  else
    printf 'installed_edge_unit_matches_bundle=unknown\n'
  fi
  if [[ -n "$edge_executable_store" && -n "$edge_caddyfile_store" ]]; then
    expected_execstart="ExecStart=${edge_executable_store} run --config ${edge_caddyfile_store} --adapter caddyfile"
    expected_execreload="ExecReload=${edge_executable_store} reload --config ${edge_caddyfile_store} --adapter caddyfile --address 127.0.0.1:2019 --force"
    if [[ "$installed_execstart" == "$expected_execstart" ]]; then
      printf 'installed_edge_unit_execstart_matches_bundle=true\n'
    else
      printf 'installed_edge_unit_execstart_matches_bundle=false\n'
    fi
    if [[ "$installed_execreload" == "$expected_execreload" ]]; then
      printf 'installed_edge_unit_execreload_matches_bundle=true\n'
    else
      printf 'installed_edge_unit_execreload_matches_bundle=false\n'
    fi
  fi
fi

print_certificate_inventory tls_fullchain "$tls_fullchain_path"
print_private_key_inventory tls_privkey "$tls_privkey_path"

printf 'remote_deploy_performed=false\n'
printf 'infrastructure_mutation_performed=false\n'
