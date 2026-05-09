#!/usr/bin/env bash
set -euo pipefail

SCRIPT_PATH="$(readlink -f "${BASH_SOURCE[0]}")"
SCRIPT_DIR="$(cd "$(dirname "$SCRIPT_PATH")" && pwd)"
source "${SCRIPT_DIR}/lib/common.sh"

service="$(normalize_named_arg service "${1-api}")"
env_file="$(normalize_named_arg env_file "${2-}")"

cd "$RECIPE_REPO_ROOT"

case "$service" in
  api)
    default_env_file="/var/lib/fishystuff/gitops-beta/api/runtime.env"
    ;;
  dolt)
    default_env_file="/var/lib/fishystuff/gitops-beta/dolt/beta.env"
    ;;
  *)
    echo "unsupported beta runtime env service: ${service}" >&2
    exit 2
    ;;
esac

if [[ -z "$env_file" ]]; then
  env_file="$default_env_file"
fi

if [[ "$env_file" == "/run/fishystuff/api/env" ]]; then
  echo "beta runtime env must not use the shared API env path: ${env_file}" >&2
  exit 2
fi
if [[ "$env_file" == /var/lib/fishystuff/gitops/* ]]; then
  echo "beta runtime env must not use production GitOps state: ${env_file}" >&2
  exit 2
fi

if [[ ! -f "$env_file" ]]; then
  echo "beta ${service} runtime env file does not exist: ${env_file}" >&2
  exit 2
fi

fail() {
  echo "$1" >&2
  exit 2
}

trim() {
  local value="$1"
  value="${value#"${value%%[![:space:]]*}"}"
  value="${value%"${value##*[![:space:]]}"}"
  printf '%s' "$value"
}

strip_env_quotes() {
  local value="$1"
  if [[ "${value:0:1}" == "'" && "${value: -1}" == "'" && "${#value}" -ge 2 ]]; then
    printf '%s' "${value:1:${#value}-2}"
    return
  fi
  if [[ "${value:0:1}" == '"' && "${value: -1}" == '"' && "${#value}" -ge 2 ]]; then
    printf '%s' "${value:1:${#value}-2}"
    return
  fi
  printf '%s' "$value"
}

env_value() {
  local key="$1"
  local line=""
  local raw=""

  while IFS= read -r line || [[ -n "$line" ]]; do
    [[ "$line" =~ ^[[:space:]]*$ ]] && continue
    [[ "$line" =~ ^[[:space:]]*# ]] && continue
    case "$line" in
      "${key}="*)
        raw="${line#*=}"
        strip_env_quotes "$raw"
        return
        ;;
    esac
  done <"$env_file"
}

require_key() {
  local key="$1"
  local value=""
  value="$(env_value "$key")"
  if [[ -z "$value" ]]; then
    fail "beta ${service} runtime env is missing ${key}"
  fi
  printf '%s' "$value"
}

require_exact() {
  local key="$1"
  local expected="$2"
  local value=""
  value="$(require_key "$key")"
  if [[ "$value" != "$expected" ]]; then
    fail "${key} must be ${expected}, got: ${value}"
  fi
}

validate_format() {
  local line=""
  local line_no=0

  while IFS= read -r line || [[ -n "$line" ]]; do
    line_no="$((line_no + 1))"
    [[ "$line" =~ ^[[:space:]]*$ ]] && continue
    [[ "$line" =~ ^[[:space:]]*# ]] && continue
    if [[ ! "$line" =~ ^[A-Za-z_][A-Za-z0-9_]*= ]]; then
      fail "invalid env assignment in ${env_file}:${line_no}"
    fi
  done <"$env_file"
}

reject_production_fragments() {
  local content=""
  content="$(<"$env_file")"
  case "$content" in
    *"https://fishystuff.fish"* | \
    *"https://www.fishystuff.fish"* | \
    *"https://api.fishystuff.fish"* | \
    *"https://cdn.fishystuff.fish"* | \
    *"/var/lib/fishystuff/gitops/"* | \
    *"/run/fishystuff/api/env"* | \
    *"fishystuff-api.service"* | \
    *"fishystuff-dolt.service"*)
      fail "beta ${service} runtime env contains production or shared deployment material"
      ;;
  esac
}

validate_api_env() {
  local database_url=""
  local cors_allowed_origins=""
  local public_site_base_url=""
  local public_cdn_base_url=""
  local runtime_cdn_base_url=""
  local seen_beta_origin="false"
  local origin=""

  database_url="$(require_key FISHYSTUFF_DATABASE_URL)"
  case "$database_url" in
    *"@127.0.0.1:3316/"* | *"@localhost:3316/"*)
      ;;
    *)
      fail "FISHYSTUFF_DATABASE_URL must point at the beta loopback Dolt SQL port 3316"
      ;;
  esac

  cors_allowed_origins="$(require_key FISHYSTUFF_CORS_ALLOWED_ORIGINS)"
  IFS=',' read -r -a origins <<<"$cors_allowed_origins"
  for origin in "${origins[@]}"; do
    origin="$(trim "$origin")"
    case "$origin" in
      "https://beta.fishystuff.fish")
        seen_beta_origin="true"
        ;;
      "https://fishystuff.fish" | "https://www.fishystuff.fish" | "https://api.fishystuff.fish" | "https://cdn.fishystuff.fish")
        fail "FISHYSTUFF_CORS_ALLOWED_ORIGINS contains a production origin: ${origin}"
        ;;
    esac
  done
  if [[ "$seen_beta_origin" != "true" ]]; then
    fail "FISHYSTUFF_CORS_ALLOWED_ORIGINS must include https://beta.fishystuff.fish"
  fi

  require_exact FISHYSTUFF_PUBLIC_SITE_BASE_URL "https://beta.fishystuff.fish"
  require_exact FISHYSTUFF_PUBLIC_CDN_BASE_URL "https://cdn.beta.fishystuff.fish"
  require_exact FISHYSTUFF_RUNTIME_CDN_BASE_URL "https://cdn.beta.fishystuff.fish"

  public_site_base_url="$(require_key FISHYSTUFF_PUBLIC_SITE_BASE_URL)"
  public_cdn_base_url="$(require_key FISHYSTUFF_PUBLIC_CDN_BASE_URL)"
  runtime_cdn_base_url="$(require_key FISHYSTUFF_RUNTIME_CDN_BASE_URL)"

  printf 'gitops_beta_runtime_env_database=loopback-dolt-beta\n'
  printf 'gitops_beta_runtime_env_cors_allowed_origins=%s\n' "$cors_allowed_origins"
  printf 'gitops_beta_runtime_env_public_site_base_url=%s\n' "$public_site_base_url"
  printf 'gitops_beta_runtime_env_public_cdn_base_url=%s\n' "$public_cdn_base_url"
  printf 'gitops_beta_runtime_env_runtime_cdn_base_url=%s\n' "$runtime_cdn_base_url"
}

validate_dolt_env() {
  local dolt_remote_branch=""
  local deployment_environment=""

  dolt_remote_branch="$(env_value DOLT_REMOTE_BRANCH)"
  if [[ -n "$dolt_remote_branch" && "$dolt_remote_branch" != "beta" ]]; then
    fail "DOLT_REMOTE_BRANCH must be beta when set for the beta Dolt service"
  fi

  deployment_environment="$(env_value FISHYSTUFF_DEPLOYMENT_ENVIRONMENT)"
  if [[ -n "$deployment_environment" && "$deployment_environment" != "beta" ]]; then
    fail "FISHYSTUFF_DEPLOYMENT_ENVIRONMENT must be beta when set for the beta Dolt service"
  fi
}

validate_format
reject_production_fragments

case "$service" in
  api)
    validate_api_env
    ;;
  dolt)
    validate_dolt_env
    ;;
esac

printf 'gitops_beta_runtime_env_ok=%s\n' "$env_file"
printf 'gitops_beta_runtime_env_service=%s\n' "$service"
printf 'remote_deploy_performed=false\n'
printf 'infrastructure_mutation_performed=false\n'
printf 'local_host_mutation_performed=false\n'
