#!/usr/bin/env bash
set -euo pipefail

SCRIPT_PATH="$(readlink -f "${BASH_SOURCE[0]}")"
SCRIPT_DIR="$(cd "$(dirname "$SCRIPT_PATH")" && pwd)"
source "${SCRIPT_DIR}/lib/common.sh"

desired_state="$(normalize_named_arg desired_state "${1-data/gitops/beta-tls.desired.json}")"
unit_file="$(normalize_named_arg unit_file "${2-data/gitops/fishystuff-beta-tls-reconciler.service}")"
cloudflare_token_source="$(normalize_named_arg cloudflare_token_source "${3-env:CLOUDFLARE_API_TOKEN}")"
install_bin="$(normalize_named_arg install_bin "${4-${FISHYSTUFF_GITOPS_INSTALL_BIN:-install}}")"
systemctl_bin="$(normalize_named_arg systemctl_bin "${5-${FISHYSTUFF_GITOPS_SYSTEMCTL_BIN:-systemctl}}")"

cd "$RECIPE_REPO_ROOT"

unit_name="fishystuff-beta-tls-reconciler.service"
desired_target="/var/lib/fishystuff/gitops-beta/desired/beta-tls.desired.json"
unit_target="/etc/systemd/system/${unit_name}"
cloudflare_token_target="/var/lib/fishystuff/gitops-beta/secrets/cloudflare-api-token"

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

require_env_value() {
  local name="$1"
  local expected="$2"
  local value="${!name-}"
  if [[ "$value" != "$expected" ]]; then
    echo "gitops-beta-install-tls-resident requires ${name}=${expected}" >&2
    exit 2
  fi
}

require_env_nonempty() {
  local name="$1"
  local value="${!name-}"
  if [[ -z "$value" ]]; then
    echo "gitops-beta-install-tls-resident requires ${name}" >&2
    exit 2
  fi
}

require_sha256_match() {
  local label="$1"
  local path="$2"
  local expected="$3"
  local actual=""
  read -r actual _ < <(sha256sum "$path")
  if [[ "$actual" != "$expected" ]]; then
    echo "${label} sha256 mismatch" >&2
    echo "actual:   ${actual}" >&2
    echo "expected: ${expected}" >&2
    exit 2
  fi
  printf '%s' "$actual"
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

case "${FISHYSTUFF_OPERATOR_SECRETSPEC_PROFILE:-}" in
  production-deploy | prod-deploy | production)
    echo "gitops-beta-install-tls-resident must not run with a production SecretSpec profile" >&2
    exit 2
    ;;
esac

require_command jq
require_command mktemp
require_command sha256sum
require_executable_or_command "$install_bin" install_bin
require_executable_or_command "$systemctl_bin" systemctl_bin
require_env_value FISHYSTUFF_GITOPS_ENABLE_BETA_TLS_RESIDENT_INSTALL 1
require_env_value FISHYSTUFF_GITOPS_ENABLE_BETA_TLS_RESIDENT_RESTART 1
require_env_nonempty FISHYSTUFF_GITOPS_BETA_TLS_DESIRED_SHA256
require_env_nonempty FISHYSTUFF_GITOPS_BETA_TLS_RESIDENT_UNIT_SHA256
require_env_nonempty FISHYSTUFF_GITOPS_BETA_TLS_CLOUDFLARE_TOKEN_SHA256
deployment_require_current_hostname_match beta gitops-beta-install-tls-resident

desired_state_path="$(absolute_path "$desired_state")"
unit_file_path="$(absolute_path "$unit_file")"
tmp_dir="$(mktemp -d)"
cleanup() {
  rm -rf "$tmp_dir"
}
trap cleanup EXIT

if [[ ! -f "$desired_state_path" ]]; then
  echo "beta TLS desired state file does not exist: ${desired_state}" >&2
  exit 2
fi
if [[ ! -f "$unit_file_path" ]]; then
  echo "beta TLS resident unit file does not exist: ${unit_file}" >&2
  exit 2
fi
case "$cloudflare_token_source" in
  env:CLOUDFLARE_API_TOKEN)
    if [[ -z "${CLOUDFLARE_API_TOKEN:-}" ]]; then
      echo "gitops-beta-install-tls-resident requires CLOUDFLARE_API_TOKEN when cloudflare_token_source=env:CLOUDFLARE_API_TOKEN" >&2
      exit 2
    fi
    cloudflare_token_source_path="${tmp_dir}/cloudflare-api-token"
    umask 077
    printf '%s\n' "$CLOUDFLARE_API_TOKEN" >"$cloudflare_token_source_path"
    chmod 600 "$cloudflare_token_source_path"
    ;;
  env:*)
    echo "unsupported beta TLS Cloudflare token env source: ${cloudflare_token_source}" >&2
    exit 2
    ;;
  *)
    cloudflare_token_source_path="$(absolute_path "$cloudflare_token_source")"
    if [[ ! -f "$cloudflare_token_source_path" ]]; then
      echo "beta TLS Cloudflare token source does not exist: ${cloudflare_token_source}" >&2
      exit 2
    fi
    ;;
esac

require_beta_tls_desired_shape "$desired_state_path"
require_beta_tls_unit_shape "$unit_file_path"
desired_sha256="$(require_sha256_match desired_state "$desired_state_path" "$FISHYSTUFF_GITOPS_BETA_TLS_DESIRED_SHA256")"
unit_sha256="$(require_sha256_match unit_file "$unit_file_path" "$FISHYSTUFF_GITOPS_BETA_TLS_RESIDENT_UNIT_SHA256")"
token_sha256="$(require_sha256_match cloudflare_token_source "$cloudflare_token_source_path" "$FISHYSTUFF_GITOPS_BETA_TLS_CLOUDFLARE_TOKEN_SHA256")"

"$install_bin" -D -m 0644 "$desired_state_path" "$desired_target"
"$install_bin" -D -m 0600 "$cloudflare_token_source_path" "$cloudflare_token_target"
"$install_bin" -D -m 0644 "$unit_file_path" "$unit_target"
"$systemctl_bin" daemon-reload
"$systemctl_bin" enable --now "$unit_name"
"$systemctl_bin" restart "$unit_name"
"$systemctl_bin" is-active --quiet "$unit_name"

printf 'gitops_beta_tls_resident_install_ok=%s\n' "$unit_name"
printf 'beta_tls_resident_desired_target=%s\n' "$desired_target"
printf 'beta_tls_resident_unit_target=%s\n' "$unit_target"
printf 'beta_tls_resident_cloudflare_token_target=%s\n' "$cloudflare_token_target"
printf 'beta_tls_resident_desired_sha256=%s\n' "$desired_sha256"
printf 'beta_tls_resident_unit_sha256=%s\n' "$unit_sha256"
printf 'beta_tls_resident_cloudflare_token_sha256=%s\n' "$token_sha256"
printf 'local_host_mutation_performed=true\n'
printf 'remote_deploy_performed=false\n'
printf 'infrastructure_mutation_performed=false\n'
