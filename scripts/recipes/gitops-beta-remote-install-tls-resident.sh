#!/usr/bin/env bash
set -euo pipefail

SCRIPT_PATH="$(readlink -f "${BASH_SOURCE[0]}")"
SCRIPT_DIR="$(cd "$(dirname "$SCRIPT_PATH")" && pwd)"
source "${SCRIPT_DIR}/lib/common.sh"

target="$(normalize_named_arg target "${1:-${FISHYSTUFF_BETA_RESIDENT_TARGET:-}}")"
expected_hostname="$(normalize_named_arg expected_hostname "${2:-site-nbg1-beta}")"
desired_state="$(normalize_named_arg desired_state "${3-data/gitops/beta-tls.desired.json}")"
unit_file="$(normalize_named_arg unit_file "${4-data/gitops/fishystuff-beta-tls-reconciler.service}")"
cloudflare_token_source="$(normalize_named_arg cloudflare_token_source "${5-${FISHYSTUFF_GITOPS_BETA_TLS_CLOUDFLARE_TOKEN_SOURCE:-env:CLOUDFLARE_API_TOKEN}}")"
ssh_bin="$(normalize_named_arg ssh_bin "${6:-${FISHYSTUFF_GITOPS_SSH_BIN:-ssh}}")"
scp_bin="$(normalize_named_arg scp_bin "${7:-${FISHYSTUFF_GITOPS_SCP_BIN:-scp}}")"

cd "$RECIPE_REPO_ROOT"

unit_name="fishystuff-beta-tls-reconciler.service"
desired_target="/var/lib/fishystuff/gitops-beta/desired/beta-tls.desired.json"
unit_target="/etc/systemd/system/${unit_name}"
cloudflare_token_target="/var/lib/fishystuff/gitops-beta/secrets/cloudflare-api-token"

fail() {
  echo "$1" >&2
  exit 2
}

require_env_value() {
  local name="$1"
  local expected="$2"
  local value="${!name-}"

  if [[ "$value" != "$expected" ]]; then
    fail "gitops-beta-remote-install-tls-resident requires ${name}=${expected}"
  fi
}

require_env_nonempty() {
  local name="$1"
  local value="${!name-}"

  if [[ -z "$value" ]]; then
    fail "gitops-beta-remote-install-tls-resident requires ${name}"
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

absolute_path() {
  local path="$1"
  if [[ "$path" == /* ]]; then
    printf '%s' "$path"
    return
  fi
  printf '%s/%s' "$RECIPE_REPO_ROOT" "$path"
}

sha256_file() {
  local path="$1"
  local value=""
  read -r value _ < <(sha256sum "$path")
  printf '%s' "$value"
}

require_sha256_match() {
  local label="$1"
  local path="$2"
  local expected="$3"
  local actual=""

  actual="$(sha256_file "$path")"
  if [[ "$actual" != "$expected" ]]; then
    fail "${label} sha256 mismatch: expected ${expected}, got ${actual}"
  fi
  printf '%s' "$actual"
}

require_beta_deploy_profile() {
  local active_profile="${FISHYSTUFF_OPERATOR_SECRETSPEC_PROFILE:-}"

  case "$active_profile" in
    beta-deploy)
      ;;
    production-deploy | prod-deploy | production)
      fail "gitops-beta-remote-install-tls-resident must not run with production SecretSpec profile active: ${active_profile}"
      ;;
    *)
      fail "gitops-beta-remote-install-tls-resident requires FISHYSTUFF_OPERATOR_SECRETSPEC_PROFILE=beta-deploy"
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
    fail "fresh beta TLS resident install currently expects root SSH, got user: ${user}"
  fi
  if [[ ! "$host" =~ ^([0-9]{1,3}\.){3}[0-9]{1,3}$ ]]; then
    fail "target host must be an IPv4 address, got: ${host}"
  fi
  if [[ "$host" == "178.104.230.121" ]]; then
    fail "target points at the previous beta host; use the fresh replacement IP"
  fi
}

require_expected_hostname() {
  local value="$1"

  if [[ -z "$value" ]]; then
    fail "expected_hostname is required"
  fi
  if [[ "$value" == *=* ]]; then
    fail "expected_hostname must be a hostname, got ${value}; pass arguments in order: target expected_hostname desired_state unit_file"
  fi
  if [[ ! "$value" =~ ^[a-zA-Z0-9][a-zA-Z0-9.-]*$ ]]; then
    fail "expected_hostname must be a simple hostname, got: ${value}"
  fi
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
    fail "beta TLS resident unit must use LoadCredential, not EnvironmentFile"
  fi
  if grep -F "fishystuff.fish" "$path" | grep -v -F "beta.fishystuff" >/dev/null; then
    fail "beta TLS resident unit contains a non-beta production hostname"
  fi
  grep -Fx "Description=FishyStuff beta GitOps TLS ACME reconciler" "$path" >/dev/null
  grep -Fx "Environment=FISHYSTUFF_GITOPS_ENABLE_LOCAL_APPLY=1" "$path" >/dev/null
  grep -Fx "Environment=FISHYSTUFF_GITOPS_STATE_FILE=${desired_target}" "$path" >/dev/null
  grep -Fx "LoadCredential=cloudflare-api-token:${cloudflare_token_target}" "$path" >/dev/null
  grep -F "CLOUDFLARE_API_TOKEN=\"\$(cat \"\$CREDENTIALS_DIRECTORY/cloudflare-api-token\")\"" "$path" >/dev/null
  grep -Fx "ReadWritePaths=/var/lib/fishystuff/gitops-beta" "$path" >/dev/null
  grep -Fx "WantedBy=multi-user.target" "$path" >/dev/null
}

require_command_or_executable jq jq
require_command_or_executable sha256sum sha256sum
require_command_or_executable mktemp mktemp
require_command_or_executable "$ssh_bin" ssh_bin
require_command_or_executable "$scp_bin" scp_bin
require_env_value FISHYSTUFF_GITOPS_ENABLE_BETA_REMOTE_TLS_RESIDENT_INSTALL 1
require_env_value FISHYSTUFF_GITOPS_ENABLE_BETA_REMOTE_TLS_RESIDENT_RESTART 1
require_env_value FISHYSTUFF_GITOPS_BETA_REMOTE_TLS_RESIDENT_TARGET "$target"
require_env_nonempty FISHYSTUFF_GITOPS_BETA_TLS_DESIRED_SHA256
require_env_nonempty FISHYSTUFF_GITOPS_BETA_TLS_RESIDENT_UNIT_SHA256
require_env_nonempty FISHYSTUFF_GITOPS_BETA_TLS_CLOUDFLARE_TOKEN_SHA256
require_beta_deploy_profile
assert_deployment_configuration_safe beta
assert_beta_infra_cluster_dns_scope_safe
require_safe_target "$target"
require_expected_hostname "$expected_hostname"

desired_state_path="$(absolute_path "$desired_state")"
unit_file_path="$(absolute_path "$unit_file")"
tmp_token_dir="$(mktemp -d)"
cleanup_token() {
  rm -rf "$tmp_token_dir"
}
trap cleanup_token EXIT

if [[ ! -f "$desired_state_path" ]]; then
  fail "beta TLS desired state file does not exist: ${desired_state}"
fi
if [[ ! -f "$unit_file_path" ]]; then
  fail "beta TLS resident unit file does not exist: ${unit_file}"
fi

case "$cloudflare_token_source" in
  env:CLOUDFLARE_API_TOKEN)
    if [[ -z "${CLOUDFLARE_API_TOKEN:-}" ]]; then
      fail "gitops-beta-remote-install-tls-resident requires CLOUDFLARE_API_TOKEN when cloudflare_token_source=env:CLOUDFLARE_API_TOKEN"
    fi
    cloudflare_token_source_path="${tmp_token_dir}/cloudflare-api-token"
    umask 077
    printf '%s\n' "$CLOUDFLARE_API_TOKEN" >"$cloudflare_token_source_path"
    chmod 600 "$cloudflare_token_source_path"
    ;;
  env:*)
    fail "unsupported beta TLS Cloudflare token env source: ${cloudflare_token_source}"
    ;;
  *)
    cloudflare_token_source_path="$(absolute_path "$cloudflare_token_source")"
    if [[ ! -f "$cloudflare_token_source_path" ]]; then
      fail "beta TLS Cloudflare token source does not exist: ${cloudflare_token_source}"
    fi
    ;;
esac

require_beta_tls_desired_shape "$desired_state_path"
require_beta_tls_unit_shape "$unit_file_path"
desired_sha256="$(require_sha256_match desired_state "$desired_state_path" "$FISHYSTUFF_GITOPS_BETA_TLS_DESIRED_SHA256")"
unit_sha256="$(require_sha256_match unit_file "$unit_file_path" "$FISHYSTUFF_GITOPS_BETA_TLS_RESIDENT_UNIT_SHA256")"
token_sha256="$(require_sha256_match cloudflare_token_source "$cloudflare_token_source_path" "$FISHYSTUFF_GITOPS_BETA_TLS_CLOUDFLARE_TOKEN_SHA256")"

tmp_key="$(create_temp_ssh_key_from_env /tmp/fishystuff-beta-tls-resident-key.XXXXXX)"
known_hosts="$(mktemp /tmp/fishystuff-beta-tls-resident-known-hosts.XXXXXX)"
cleanup() {
  rm -f "$tmp_key" "$known_hosts"
  rm -rf "$tmp_token_dir"
}
trap cleanup EXIT

printf 'gitops_beta_remote_install_tls_resident_checked=true\n'
printf 'deployment=beta\n'
printf 'resident_target=%s\n' "$target"
printf 'desired_sha256=%s\n' "$desired_sha256"
printf 'unit_sha256=%s\n' "$unit_sha256"
printf 'cloudflare_token_sha256=%s\n' "$token_sha256"

ssh_common=(
  -i "$tmp_key"
  -o IdentitiesOnly=yes
  -o StrictHostKeyChecking=accept-new
  -o UserKnownHostsFile="$known_hosts"
)

remote_desired="/tmp/fishystuff-beta-tls-resident-desired.json"
remote_unit="/tmp/fishystuff-beta-tls-resident.service"
remote_token="/tmp/fishystuff-beta-tls-resident-cloudflare-api-token"
"$scp_bin" "${ssh_common[@]}" "$desired_state_path" "${target}:${remote_desired}"
"$scp_bin" "${ssh_common[@]}" "$unit_file_path" "${target}:${remote_unit}"
"$scp_bin" "${ssh_common[@]}" "$cloudflare_token_source_path" "${target}:${remote_token}"

"$ssh_bin" "${ssh_common[@]}" "$target" bash -s -- \
  "$expected_hostname" \
  "$remote_desired" \
  "$remote_unit" \
  "$remote_token" \
  "$desired_sha256" \
  "$unit_sha256" \
  "$token_sha256" \
  "$desired_target" \
  "$unit_target" \
  "$cloudflare_token_target" \
  "$unit_name" <<'REMOTE'
set -euo pipefail

expected_hostname="$1"
remote_desired="$2"
remote_unit="$3"
remote_token="$4"
desired_sha256="$5"
unit_sha256="$6"
token_sha256="$7"
desired_target="$8"
unit_target="$9"
cloudflare_token_target="${10}"
unit_name="${11}"

fail() {
  echo "$1" >&2
  exit 2
}

require_file() {
  local label="$1"
  local path="$2"

  if [[ ! -f "$path" ]]; then
    fail "${label} missing: ${path}"
  fi
}

require_sha256() {
  local label="$1"
  local path="$2"
  local expected="$3"
  local actual=""

  read -r actual _ < <(sha256sum "$path")
  if [[ "$actual" != "$expected" ]]; then
    fail "${label} sha256 mismatch: expected ${expected}, got ${actual}"
  fi
}

require_unit_shape() {
  local path="$1"

  if grep -F "EnvironmentFile=" "$path" >/dev/null; then
    fail "beta TLS resident unit must use LoadCredential, not EnvironmentFile"
  fi
  if grep -F "fishystuff.fish" "$path" | grep -v -F "beta.fishystuff" >/dev/null; then
    fail "beta TLS resident unit contains a non-beta production hostname"
  fi
  grep -Fx "Description=FishyStuff beta GitOps TLS ACME reconciler" "$path" >/dev/null
  grep -Fx "Environment=FISHYSTUFF_GITOPS_ENABLE_LOCAL_APPLY=1" "$path" >/dev/null
  grep -Fx "Environment=FISHYSTUFF_GITOPS_STATE_FILE=${desired_target}" "$path" >/dev/null
  grep -Fx "LoadCredential=cloudflare-api-token:${cloudflare_token_target}" "$path" >/dev/null
  grep -F "CLOUDFLARE_API_TOKEN=\"\$(cat \"\$CREDENTIALS_DIRECTORY/cloudflare-api-token\")\"" "$path" >/dev/null
  grep -Fx "ReadWritePaths=/var/lib/fishystuff/gitops-beta" "$path" >/dev/null
  grep -Fx "WantedBy=multi-user.target" "$path" >/dev/null
}

if [[ "$(hostname)" != "$expected_hostname" ]]; then
  fail "remote hostname mismatch: expected ${expected_hostname}, got $(hostname)"
fi

require_file "beta TLS desired" "$remote_desired"
require_file "beta TLS resident unit" "$remote_unit"
require_file "beta TLS Cloudflare token" "$remote_token"
require_sha256 "beta TLS desired" "$remote_desired" "$desired_sha256"
require_sha256 "beta TLS resident unit" "$remote_unit" "$unit_sha256"
require_sha256 "beta TLS Cloudflare token" "$remote_token" "$token_sha256"
require_unit_shape "$remote_unit"

install -D -m 0644 "$remote_desired" "$desired_target"
install -D -m 0600 "$remote_token" "$cloudflare_token_target"
install -D -m 0644 "$remote_unit" "$unit_target"
rm -f "$remote_desired" "$remote_unit" "$remote_token"
systemctl daemon-reload
systemctl enable --now "$unit_name"
systemctl restart "$unit_name"
systemctl is-active --quiet "$unit_name"

printf 'remote_hostname=%s\n' "$(hostname)"
printf 'remote_tls_resident_install_ok=%s\n' "$unit_name"
printf 'remote_tls_resident_desired_target=%s\n' "$desired_target"
printf 'remote_tls_resident_unit_target=%s\n' "$unit_target"
printf 'remote_tls_resident_cloudflare_token_target=%s\n' "$cloudflare_token_target"
printf 'remote_host_mutation_performed=true\n'
printf 'remote_deploy_performed=false\n'
printf 'infrastructure_mutation_performed=false\n'
printf 'local_host_mutation_performed=false\n'
REMOTE

printf 'gitops_beta_remote_install_tls_resident_ok=true\n'
printf 'remote_host_mutation_performed=true\n'
printf 'remote_deploy_performed=false\n'
printf 'infrastructure_mutation_performed=false\n'
printf 'local_host_mutation_performed=false\n'
