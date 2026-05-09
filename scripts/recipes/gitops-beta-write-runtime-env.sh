#!/usr/bin/env bash
set -euo pipefail

SCRIPT_PATH="$(readlink -f "${BASH_SOURCE[0]}")"
SCRIPT_DIR="$(cd "$(dirname "$SCRIPT_PATH")" && pwd)"
source "${SCRIPT_DIR}/lib/common.sh"

service="$(normalize_named_arg service "${1-api}")"
output="$(normalize_named_arg output "${2-}")"

cd "$RECIPE_REPO_ROOT"

case "$service" in
  api)
    default_output="/var/lib/fishystuff/gitops-beta/api/runtime.env"
    enable_var="FISHYSTUFF_GITOPS_ENABLE_BETA_API_RUNTIME_ENV_WRITE"
    ;;
  dolt)
    default_output="/var/lib/fishystuff/gitops-beta/dolt/beta.env"
    enable_var="FISHYSTUFF_GITOPS_ENABLE_BETA_DOLT_RUNTIME_ENV_WRITE"
    ;;
  *)
    echo "unsupported beta runtime env service: ${service}" >&2
    exit 2
    ;;
esac

if [[ -z "$output" ]]; then
  output="$default_output"
fi

require_env_value() {
  local name="$1"
  local expected="$2"
  local value="${!name-}"

  if [[ "$value" != "$expected" ]]; then
    echo "gitops-beta-write-runtime-env requires ${name}=${expected}" >&2
    exit 2
  fi
}

require_env_nonempty() {
  local name="$1"
  local value="${!name-}"

  if [[ -z "$value" ]]; then
    echo "gitops-beta-write-runtime-env requires ${name}" >&2
    exit 2
  fi
}

require_safe_output_path() {
  local service_name="$1"
  local path="$2"

  case "$service_name:$path" in
    api:/var/lib/fishystuff/gitops-beta/api/runtime.env | \
    dolt:/var/lib/fishystuff/gitops-beta/dolt/beta.env | \
    api:/tmp/* | \
    dolt:/tmp/*)
      ;;
    *)
      echo "refusing to write beta ${service_name} runtime env outside the beta runtime path or /tmp: ${path}" >&2
      exit 2
      ;;
  esac
}

require_env_file_value_safe() {
  local name="$1"
  local value="$2"

  case "$value" in
    *$'\n'* | *$'\r'* | *"'"*)
      echo "${name} contains a character that cannot be safely written to a systemd env file" >&2
      exit 2
      ;;
  esac
}

require_beta_api_database_url() {
  local value="$1"

  case "$value" in
    *"@127.0.0.1:3316/"* | *"@localhost:3316/"*)
      ;;
    *)
      echo "FISHYSTUFF_GITOPS_BETA_API_DATABASE_URL must point at the beta loopback Dolt SQL port 3316" >&2
      exit 2
      ;;
  esac
}

require_exact_value() {
  local name="$1"
  local value="$2"
  local expected="$3"

  if [[ "$value" != "$expected" ]]; then
    echo "${name} must be ${expected}, got: ${value}" >&2
    exit 2
  fi
}

reject_production_value() {
  local name="$1"
  local value="$2"

  case "$value" in
    *"https://fishystuff.fish"* | \
    *"https://www.fishystuff.fish"* | \
    *"https://api.fishystuff.fish"* | \
    *"https://cdn.fishystuff.fish"* | \
    *"/var/lib/fishystuff/gitops/"* | \
    *"/run/fishystuff/api/env"*)
      echo "${name} contains production or shared deployment material" >&2
      exit 2
      ;;
  esac
}

write_api_env() {
  local database_url="${FISHYSTUFF_GITOPS_BETA_API_DATABASE_URL-}"
  local cors_allowed_origins="${FISHYSTUFF_GITOPS_BETA_API_CORS_ALLOWED_ORIGINS:-https://beta.fishystuff.fish}"
  local public_site_base_url="${FISHYSTUFF_GITOPS_BETA_PUBLIC_SITE_BASE_URL:-https://beta.fishystuff.fish}"
  local public_cdn_base_url="${FISHYSTUFF_GITOPS_BETA_PUBLIC_CDN_BASE_URL:-https://cdn.beta.fishystuff.fish}"
  local runtime_cdn_base_url="${FISHYSTUFF_GITOPS_BETA_RUNTIME_CDN_BASE_URL:-https://cdn.beta.fishystuff.fish}"

  require_env_nonempty FISHYSTUFF_GITOPS_BETA_API_DATABASE_URL
  require_beta_api_database_url "$database_url"
  require_exact_value FISHYSTUFF_GITOPS_BETA_PUBLIC_SITE_BASE_URL "$public_site_base_url" "https://beta.fishystuff.fish"
  require_exact_value FISHYSTUFF_GITOPS_BETA_PUBLIC_CDN_BASE_URL "$public_cdn_base_url" "https://cdn.beta.fishystuff.fish"
  require_exact_value FISHYSTUFF_GITOPS_BETA_RUNTIME_CDN_BASE_URL "$runtime_cdn_base_url" "https://cdn.beta.fishystuff.fish"

  require_env_file_value_safe FISHYSTUFF_GITOPS_BETA_API_DATABASE_URL "$database_url"
  require_env_file_value_safe FISHYSTUFF_GITOPS_BETA_API_CORS_ALLOWED_ORIGINS "$cors_allowed_origins"
  require_env_file_value_safe FISHYSTUFF_GITOPS_BETA_PUBLIC_SITE_BASE_URL "$public_site_base_url"
  require_env_file_value_safe FISHYSTUFF_GITOPS_BETA_PUBLIC_CDN_BASE_URL "$public_cdn_base_url"
  require_env_file_value_safe FISHYSTUFF_GITOPS_BETA_RUNTIME_CDN_BASE_URL "$runtime_cdn_base_url"

  reject_production_value FISHYSTUFF_GITOPS_BETA_API_DATABASE_URL "$database_url"
  reject_production_value FISHYSTUFF_GITOPS_BETA_API_CORS_ALLOWED_ORIGINS "$cors_allowed_origins"
  reject_production_value FISHYSTUFF_GITOPS_BETA_PUBLIC_SITE_BASE_URL "$public_site_base_url"
  reject_production_value FISHYSTUFF_GITOPS_BETA_PUBLIC_CDN_BASE_URL "$public_cdn_base_url"
  reject_production_value FISHYSTUFF_GITOPS_BETA_RUNTIME_CDN_BASE_URL "$runtime_cdn_base_url"

  cat >"$tmp_file" <<EOF
# FishyStuff beta API runtime configuration.
# Operator-owned. GitOps writes release identity to /var/lib/fishystuff/gitops-beta/api/beta.env.
FISHYSTUFF_DATABASE_URL='${database_url}'
FISHYSTUFF_CORS_ALLOWED_ORIGINS='${cors_allowed_origins}'
FISHYSTUFF_PUBLIC_SITE_BASE_URL='${public_site_base_url}'
FISHYSTUFF_PUBLIC_CDN_BASE_URL='${public_cdn_base_url}'
FISHYSTUFF_RUNTIME_CDN_BASE_URL='${runtime_cdn_base_url}'
EOF
}

write_dolt_env() {
  cat >"$tmp_file" <<'EOF'
# FishyStuff beta Dolt runtime configuration.
# Intentionally empty for now. The beta Dolt unit pins deployment identity through static unit env.
EOF
}

require_env_value "$enable_var" 1
require_safe_output_path "$service" "$output"

output_dir="$(dirname "$output")"
output_base="$(basename "$output")"
mkdir -p "$output_dir"
tmp_file="$(mktemp "${output_dir}/.${output_base}.tmp.XXXXXX")"
cleanup() {
  rm -f "$tmp_file"
}
trap cleanup EXIT

case "$service" in
  api)
    write_api_env
    ;;
  dolt)
    write_dolt_env
    ;;
esac

chmod 0640 "$tmp_file"
bash scripts/recipes/gitops-check-beta-runtime-env.sh "$service" "$tmp_file" >/dev/null
mv -f "$tmp_file" "$output"
trap - EXIT

printf 'gitops_beta_runtime_env_write_ok=%s\n' "$output"
printf 'gitops_beta_runtime_env_service=%s\n' "$service"
printf 'remote_deploy_performed=false\n'
printf 'infrastructure_mutation_performed=false\n'
printf 'local_host_mutation_performed=true\n'
