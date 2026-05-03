#!/usr/bin/env bash

RECIPE_LIB_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
RECIPE_REPO_ROOT="$(cd "${RECIPE_LIB_DIR}/../../.." && pwd)"
RECIPE_DEFAULT_DEPLOYMENT_SERVICES=(
  api
  dolt
  edge
  site
  cdn
  loki
  otel-collector
  vector
  prometheus
  jaeger
  grafana
)
RECIPE_DEFAULT_MUTATING_DEPLOY_SERVICES=(
  api
  dolt
  edge
  site
  loki
  otel-collector
  vector
  prometheus
  jaeger
  grafana
)

if [[ -n "${FISHYSTUFF_RECIPE_ENV_FILE:-}" ]]; then
  if [[ ! -f "$FISHYSTUFF_RECIPE_ENV_FILE" ]]; then
    echo "deployment config file does not exist: $FISHYSTUFF_RECIPE_ENV_FILE" >&2
    exit 2
  fi
  set -a
  # shellcheck disable=SC1090
  source "$FISHYSTUFF_RECIPE_ENV_FILE"
  set +a
fi
RECIPE_RESIDENT_BUNDLE_SERVICES=(
  api
  dolt
  edge
  loki
  otel-collector
  vector
  prometheus
  jaeger
  grafana
)

normalize_named_arg() {
  local name="$1"
  local value="${2-}"
  if [[ "$value" == "$name="* ]]; then
    printf '%s' "${value#*=}"
    return
  fi
  printf '%s' "$value"
}

require_value() {
  local value="$1"
  local message="$2"
  if [[ -z "$value" ]]; then
    echo "$message" >&2
    exit 2
  fi
}

operator_secretspec_profile() {
  printf '%s' "${FISHYSTUFF_OPERATOR_SECRETSPEC_PROFILE:-beta-deploy}"
}

exec_with_secretspec_profile_if_needed() {
  local profile="${1-}"
  local active_profile="${FISHYSTUFF_OPERATOR_SECRETSPEC_PROFILE:-}"
  shift
  if [[ -n "$profile" && ( -z "${HETZNER_SSH_PRIVATE_KEY:-}" || "$active_profile" != "$profile" ) ]]; then
    exec env FISHYSTUFF_OPERATOR_SECRETSPEC_PROFILE="$profile" secretspec run --profile "$profile" -- "$@"
  fi
}

deployment_env_var_name() {
  local deployment="$1"
  local key="$2"
  local deployment_upper=""
  local key_upper=""

  deployment_upper="$(printf '%s' "$deployment" | tr '[:lower:]-' '[:upper:]_')"
  key_upper="$(printf '%s' "$key" | tr '[:lower:]-' '[:upper:]_')"
  printf 'FISHYSTUFF_%s_%s' "$deployment_upper" "$key_upper"
}

deployment_env_value() {
  local deployment
  local key
  local env_name

  deployment="$(canonical_deployment_name "$1")"
  key="$2"
  env_name="$(deployment_env_var_name "$deployment" "$key")"
  printf '%s' "${!env_name-}"
}

deployment_env_or_default() {
  local deployment="$1"
  local key="$2"
  local default_value="${3-}"
  local value=""

  value="$(deployment_env_value "$deployment" "$key")"
  if [[ -n "$value" ]]; then
    printf '%s' "$value"
    return
  fi
  printf '%s' "$default_value"
}

ensure_trailing_slash() {
  local value="$1"
  if [[ -n "$value" && "$value" != */ ]]; then
    printf '%s/' "$value"
    return
  fi
  printf '%s' "$value"
}

trim_trailing_slash() {
  local value="$1"
  while [[ -n "$value" && "$value" == */ ]]; do
    value="${value%/}"
  done
  printf '%s' "$value"
}

canonical_deploy_service_name() {
  local value="${1-}"
  value="$(canonical_public_service_name "$value")"
  case "$value" in
    api | dolt | edge | site | cdn | loki | otel-collector | vector | prometheus | jaeger | grafana)
      printf '%s' "$value"
      ;;
    *)
      echo "service $1 is not deployable" >&2
      exit 2
      ;;
  esac
}

deploy_service_backend_name() {
  local service
  service="$(canonical_deploy_service_name "$1")"
  case "$service" in
    otel-collector) printf '%s' "otel_collector" ;;
    *)
      printf '%s' "${service//-/_}"
      ;;
  esac
}

canonical_deployment_name() {
  local value="${1-}"
  value="$(printf '%s' "$value" | tr '[:upper:]' '[:lower:]')"
  case "$value" in
    prod)
      printf '%s' "production"
      ;;
    local | beta | production)
      printf '%s' "$value"
      ;;
    *)
      echo "unknown deployment: $1" >&2
      exit 2
      ;;
  esac
}

canonical_public_service_name() {
  local value="${1-}"
  value="$(printf '%s' "$value" | tr '[:upper:]' '[:lower:]')"
  value="${value//_/-}"
  case "$value" in
    site | api | cdn | telemetry | dolt | edge | grafana | dashboard | loki | logs | loki-status | prometheus | vector | jaeger | map)
      printf '%s' "$value"
      ;;
    otel-collector | otelcollector)
      printf '%s' "otel-collector"
      ;;
    *)
      echo "unknown service: $1" >&2
      exit 2
      ;;
  esac
}

normalize_deployment_environment() {
  local value="$1"
  value="$(printf '%s' "$value" | tr '[:upper:]' '[:lower:]')"
  if [[ -z "$value" ]]; then
    printf '%s' "beta"
    return
  fi
  printf '%s' "$value"
}

deployment_domain() {
  local value="$1"
  if [[ "$value" == "production" ]]; then
    printf '%s' "fishystuff.fish"
    return
  fi
  printf '%s' "${value}.fishystuff.fish"
}

deployment_environment_name() {
  local deployment
  deployment="$(canonical_deployment_name "$1")"
  case "$deployment" in
    local) printf '%s' "local" ;;
    beta) printf '%s' "$(deployment_env_or_default "$deployment" "deployment_environment" "beta")" ;;
    production) printf '%s' "$(deployment_env_or_default "$deployment" "deployment_environment" "production")" ;;
  esac
}

deployment_public_base_url() {
  local deployment
  local service
  local environment
  local host

  deployment="$(canonical_deployment_name "$1")"
  service="$(canonical_public_service_name "$2")"
  if [[ "$deployment" == "local" ]]; then
    case "$service" in
      site) printf '%s' "http://127.0.0.1:1990/" ;;
      map) printf '%s' "http://127.0.0.1:1990/map/" ;;
      api) printf '%s' "http://127.0.0.1:8080/" ;;
      cdn) printf '%s' "http://127.0.0.1:4040/" ;;
      telemetry) printf '%s' "http://telemetry.localhost:1990/" ;;
      grafana | dashboard) printf '%s' "http://127.0.0.1:3000/d/fishystuff-local-observability/fishystuff-local-observability" ;;
      loki | logs) printf '%s' "http://127.0.0.1:3000/d/fishystuff-local-observability/fishystuff-local-observability?orgId=1&viewPanel=11" ;;
      loki-status) printf '%s' "http://127.0.0.1:3100/services" ;;
      prometheus) printf '%s' "http://127.0.0.1:9090/" ;;
      vector) printf '%s' "http://127.0.0.1:8686/playground" ;;
      jaeger) printf '%s' "http://127.0.0.1:16686/" ;;
      *)
        echo "service $service is not openable for deployment $deployment" >&2
        exit 2
        ;;
    esac
    return
  fi

  environment="$(deployment_environment_name "$deployment")"
  host="$(deployment_domain "$environment")"
  case "$service" in
    site)
      ensure_trailing_slash "$(deployment_env_or_default "$deployment" "site_base_url" "https://${host}/")"
      ;;
    map)
      printf '%s' "$(deployment_public_base_url "$deployment" "site")map/"
      ;;
    api)
      ensure_trailing_slash "$(deployment_env_or_default "$deployment" "api_base_url" "https://api.${host}/")"
      ;;
    cdn)
      ensure_trailing_slash "$(deployment_env_or_default "$deployment" "cdn_base_url" "https://cdn.${host}/")"
      ;;
    telemetry)
      ensure_trailing_slash "$(deployment_env_or_default "$deployment" "telemetry_base_url" "https://telemetry.${host}/")"
      ;;
    *)
      echo "service $service is not directly public for deployment $deployment" >&2
      exit 2
      ;;
  esac
}

deployment_open_url() {
  local deployment
  local service
  local base_url

  deployment="$(canonical_deployment_name "$1")"
  service="$(canonical_public_service_name "$2")"
  base_url="$(deployment_public_base_url "$deployment" "$service")"
  case "$service" in
    api) printf '%sapi/v1/meta' "$base_url" ;;
    *)
      printf '%s' "$base_url"
      ;;
  esac
}

deployment_manifest_public_url() {
  local deployment
  local service

  deployment="$(canonical_deployment_name "$1")"
  service="$(canonical_public_service_name "$2")"
  trim_trailing_slash "$(deployment_public_base_url "$deployment" "$service")"
}

deployment_resident_hostname() {
  local deployment
  deployment="$(canonical_deployment_name "$1")"
  case "$deployment" in
    beta) printf '%s' "$(deployment_env_or_default "$deployment" "resident_hostname" "site-nbg1-beta")" ;;
    production) printf '%s' "$(deployment_env_or_default "$deployment" "resident_hostname" "site-nbg1-prod")" ;;
    local) printf '%s' "" ;;
  esac
}

deployment_resident_target() {
  local deployment
  deployment="$(canonical_deployment_name "$1")"
  case "$deployment" in
    beta) printf '%s' "$(deployment_env_or_default "$deployment" "resident_target" "root@beta.fishystuff.fish")" ;;
    production) printf '%s' "$(deployment_env_value "$deployment" "resident_target")" ;;
    local) printf '%s' "" ;;
  esac
}

hetzner_server_public_ipv4() {
  local server_name="$1"
  local ipv4=""

  require_value "${HETZNER_API_TOKEN:-}" "HETZNER_API_TOKEN is required to discover production host $server_name"
  ipv4="$(
    curl -fsS \
      -H "Authorization: Bearer ${HETZNER_API_TOKEN}" \
      -H "Content-Type: application/json" \
      "https://api.hetzner.cloud/v1/servers?name=${server_name}" \
      | jq -r '.servers[0].public_net.ipv4.ip // empty'
  )"
  require_value "$ipv4" "could not discover public IPv4 for Hetzner server $server_name"
  printf '%s' "$ipv4"
}

resolve_deployment_resident_target() {
  local deployment
  local resident_target
  local resident_host
  local resident_ipv4

  deployment="$(canonical_deployment_name "$1")"
  resident_target="$(deployment_resident_target "$deployment")"
  if [[ -n "$resident_target" || "$deployment" != "production" ]]; then
    printf '%s' "$resident_target"
    return
  fi

  resident_host="$(deployment_resident_hostname "$deployment")"
  require_value "$resident_host" "deployment $deployment does not define a resident hostname"
  resident_ipv4="$(hetzner_server_public_ipv4 "$resident_host")"
  printf 'root@%s' "$resident_ipv4"
}

deployment_telemetry_target() {
  local deployment
  deployment="$(canonical_deployment_name "$1")"
  case "$deployment" in
    beta) printf '%s' "$(deployment_env_or_default "$deployment" "telemetry_target" "root@telemetry.beta.fishystuff.fish")" ;;
    production) printf '%s' "$(deployment_env_value "$deployment" "telemetry_target")" ;;
    local) printf '%s' "" ;;
  esac
}

deployment_shared_telemetry_target() {
  local deployment
  deployment="$(canonical_deployment_name "$1")"
  case "$deployment" in
    beta)
      deployment_telemetry_target "$deployment"
      ;;
    production)
      printf '%s' "$(deployment_env_or_default "$deployment" "telemetry_target" "$(deployment_telemetry_target beta)")"
      ;;
    local)
      printf '%s' ""
      ;;
  esac
}

deployment_control_target() {
  local deployment
  deployment="$(canonical_deployment_name "$1")"
  case "$deployment" in
    beta) printf '%s' "$(deployment_env_or_default "$deployment" "control_target" "mgmt-root")" ;;
    production) printf '%s' "$(deployment_env_or_default "$deployment" "control_target" "mgmt-root")" ;;
    local) printf '%s' "" ;;
  esac
}

deployment_telemetry_hostname() {
  local deployment
  deployment="$(canonical_deployment_name "$1")"
  case "$deployment" in
    beta) printf '%s' "$(deployment_env_or_default "$deployment" "telemetry_hostname" "telemetry-nbg1")" ;;
    production) printf '%s' "$(deployment_env_value "$deployment" "telemetry_hostname")" ;;
    local) printf '%s' "" ;;
  esac
}

deployment_prod_hostname() {
  local deployment
  deployment="$(canonical_deployment_name "$1")"
  case "$deployment" in
    beta) printf '%s' "" ;;
    production) printf '%s' "$(deployment_env_or_default "$deployment" "prod_hostname" "site-nbg1-prod")" ;;
    local) printf '%s' "" ;;
  esac
}

deployment_dolt_remote_branch() {
  local deployment
  deployment="$(canonical_deployment_name "$1")"
  case "$deployment" in
    beta) printf '%s' "$(deployment_env_or_default "$deployment" "dolt_remote_branch" "beta")" ;;
    production) printf '%s' "$(deployment_env_or_default "$deployment" "dolt_remote_branch" "main")" ;;
    local) printf '%s' "" ;;
  esac
}

deployment_tunnel_target() {
  local deployment
  local service="${2-}"
  local explicit_target=""
  local telemetry_tunnel_target=""

  deployment="$(canonical_deployment_name "$1")"
  if [[ -n "$service" ]]; then
    service="$(canonical_public_service_name "$service")"
    case "$service" in
      dashboard | grafana | jaeger | loki | logs | loki-status | prometheus | vector)
        telemetry_tunnel_target="$(deployment_env_value "$deployment" "telemetry_tunnel_target")"
        if [[ -n "$telemetry_tunnel_target" ]]; then
          printf '%s' "$telemetry_tunnel_target"
        else
          printf '%s' "$(deployment_shared_telemetry_target "$deployment")"
        fi
        return
        ;;
    esac
  fi
  explicit_target="$(deployment_env_value "$deployment" "tunnel_target")"
  if [[ -n "$explicit_target" ]]; then
    printf '%s' "$explicit_target"
    return
  fi
  printf '%s' "$(deployment_resident_target "$deployment")"
}

deployment_open_secretspec_profile() {
  local deployment
  deployment="$(canonical_deployment_name "$1")"
  case "$deployment" in
    beta | production) deployment_secretspec_profile "$deployment" ;;
    local) printf '%s' "" ;;
  esac
}

deployment_secretspec_profile() {
  local deployment
  deployment="$(canonical_deployment_name "$1")"
  case "$deployment" in
    beta) printf '%s' "beta-deploy" ;;
    production) printf '%s' "production-deploy" ;;
    local) printf '%s' "" ;;
  esac
}

url_host() {
  local value="$1"
  value="${value#*://}"
  value="${value%%/*}"
  value="${value%%\?*}"
  value="${value%%#*}"
  value="${value#*@}"
  value="${value%%:*}"
  printf '%s' "$value"
}

ssh_target_host() {
  local value="$1"
  value="${value#ssh://}"
  value="${value#*@}"
  value="${value%%/*}"
  value="${value%%:*}"
  printf '%s' "$value"
}

deployment_expected_public_host() {
  local deployment
  local service

  deployment="$(canonical_deployment_name "$1")"
  service="$(canonical_public_service_name "$2")"
  case "$deployment:$service" in
    beta:site) printf '%s' "beta.fishystuff.fish" ;;
    beta:api) printf '%s' "api.beta.fishystuff.fish" ;;
    beta:cdn) printf '%s' "cdn.beta.fishystuff.fish" ;;
    beta:telemetry) printf '%s' "telemetry.beta.fishystuff.fish" ;;
    production:site) printf '%s' "fishystuff.fish" ;;
    production:api) printf '%s' "api.fishystuff.fish" ;;
    production:cdn) printf '%s' "cdn.fishystuff.fish" ;;
    production:telemetry) printf '%s' "telemetry.fishystuff.fish" ;;
    *)
      echo "deployment $deployment does not define an expected public host for $service" >&2
      exit 2
      ;;
  esac
}

deployment_public_host_is_production() {
  case "$1" in
    fishystuff.fish | api.fishystuff.fish | cdn.fishystuff.fish | telemetry.fishystuff.fish)
      return 0
      ;;
  esac
  return 1
}

deployment_public_host_is_beta() {
  case "$1" in
    beta.fishystuff.fish | api.beta.fishystuff.fish | cdn.beta.fishystuff.fish | telemetry.beta.fishystuff.fish)
      return 0
      ;;
  esac
  return 1
}

deployment_target_mentions_production() {
  local value="$1"
  local host=""
  [[ -n "$value" ]] || return 1
  host="$(ssh_target_host "$value")"
  case "$host" in
    fishystuff.fish | api.fishystuff.fish | cdn.fishystuff.fish | telemetry.fishystuff.fish | site-nbg1-prod)
      return 0
      ;;
  esac
  case "$value" in
    *production* | *site-nbg1-prod* | *fishystuff.fish*)
      if [[ "$value" != *beta.fishystuff.fish* ]]; then
        return 0
      fi
      ;;
  esac
  return 1
}

deployment_target_mentions_beta() {
  local value="$1"
  local host=""
  [[ -n "$value" ]] || return 1
  host="$(ssh_target_host "$value")"
  case "$host" in
    beta.fishystuff.fish | api.beta.fishystuff.fish | cdn.beta.fishystuff.fish | telemetry.beta.fishystuff.fish | site-nbg1-beta | telemetry-nbg1)
      return 0
      ;;
  esac
  case "$value" in
    *beta.fishystuff.fish* | *site-nbg1-beta* | *telemetry-nbg1*)
      return 0
      ;;
  esac
  return 1
}

assert_deployment_public_urls_safe() {
  local deployment="$1"
  local service=""
  local expected_host=""
  local actual_host=""
  local url=""

  for service in site api cdn telemetry; do
    url="$(deployment_public_base_url "$deployment" "$service")"
    actual_host="$(url_host "$url")"
    expected_host="$(deployment_expected_public_host "$deployment" "$service")"
    if [[ "$actual_host" != "$expected_host" ]]; then
      echo "unsafe $deployment $service URL host: expected $expected_host, got ${actual_host:-<empty>} from $url" >&2
      exit 2
    fi
    case "$deployment" in
      beta)
        if deployment_public_host_is_production "$actual_host"; then
          echo "unsafe beta $service URL points at production host: $actual_host" >&2
          exit 2
        fi
        ;;
      production)
        if deployment_public_host_is_beta "$actual_host"; then
          echo "unsafe production $service URL points at beta host: $actual_host" >&2
          exit 2
        fi
        ;;
    esac
  done
}

assert_deployment_secret_scope_safe() {
  local deployment="$1"
  local expected_profile=""
  local active_profile="${FISHYSTUFF_OPERATOR_SECRETSPEC_PROFILE:-}"

  expected_profile="$(deployment_secretspec_profile "$deployment")"
  [[ -n "$expected_profile" ]] || return 0

  if [[ -n "${HETZNER_SSH_PRIVATE_KEY:-}" && -z "$active_profile" ]]; then
    echo "unsafe $deployment secret scope: deploy secrets are loaded but FISHYSTUFF_OPERATOR_SECRETSPEC_PROFILE is unset" >&2
    exit 2
  fi

  if [[ -n "$active_profile" && "$active_profile" != "$expected_profile" ]]; then
    echo "unsafe $deployment secret scope: expected SecretSpec profile $expected_profile, got $active_profile" >&2
    exit 2
  fi
}

assert_deployment_targets_safe() {
  local deployment="$1"
  local label=""
  local value=""

  for label in resident_target telemetry_target tunnel_target control_target; do
    case "$label" in
      resident_target) value="$(deployment_resident_target "$deployment")" ;;
      telemetry_target) value="$(deployment_telemetry_target "$deployment")" ;;
      tunnel_target) value="$(deployment_tunnel_target "$deployment")" ;;
      control_target) value="$(deployment_control_target "$deployment")" ;;
    esac
    [[ -n "$value" ]] || continue
    case "$deployment" in
      beta)
        if deployment_target_mentions_production "$value"; then
          echo "unsafe beta $label mentions production: $value" >&2
          exit 2
        fi
        ;;
      production)
        if deployment_target_mentions_beta "$value"; then
          echo "unsafe production $label mentions beta: $value" >&2
          exit 2
        fi
        ;;
    esac
  done
}

assert_deployment_branch_safe() {
  local deployment="$1"
  local branch=""

  branch="$(deployment_dolt_remote_branch "$deployment")"
  case "$deployment" in
    beta)
      if [[ "$branch" != "beta" ]]; then
        echo "unsafe beta Dolt branch: expected beta, got ${branch:-<empty>}" >&2
        exit 2
      fi
      ;;
    production)
      if [[ "$branch" != "main" ]]; then
        echo "unsafe production Dolt branch: expected main, got ${branch:-<empty>}" >&2
        exit 2
      fi
      ;;
  esac
}

assert_deployment_environment_safe() {
  local deployment="$1"
  local environment=""

  environment="$(deployment_environment_name "$deployment")"
  case "$deployment" in
    beta)
      if [[ "$environment" != "beta" ]]; then
        echo "unsafe beta deployment environment: expected beta, got ${environment:-<empty>}" >&2
        exit 2
      fi
      ;;
    production)
      if [[ "$environment" != "production" ]]; then
        echo "unsafe production deployment environment: expected production, got ${environment:-<empty>}" >&2
        exit 2
      fi
      ;;
  esac
}

assert_deployment_prod_hostname_safe() {
  local deployment="$1"
  local prod_hostname=""

  prod_hostname="$(deployment_prod_hostname "$deployment")"
  case "$deployment" in
    beta)
      if [[ -n "$prod_hostname" ]]; then
        echo "unsafe beta production hostname setting: $prod_hostname" >&2
        exit 2
      fi
      ;;
    production)
      if [[ -n "$prod_hostname" && ( "$prod_hostname" == *beta* || "$prod_hostname" == "site-nbg1-beta" ) ]]; then
        echo "unsafe production prod hostname setting: $prod_hostname" >&2
        exit 2
      fi
      ;;
  esac
}

assert_deployment_configuration_safe() {
  local deployment
  deployment="$(canonical_deployment_name "$1")"
  case "$deployment" in
    local) return 0 ;;
  esac

  assert_deployment_secret_scope_safe "$deployment"
  assert_deployment_environment_safe "$deployment"
  assert_deployment_prod_hostname_safe "$deployment"
  assert_deployment_public_urls_safe "$deployment"
  assert_deployment_targets_safe "$deployment"
  assert_deployment_branch_safe "$deployment"
}

assert_resident_push_scope_safe() {
  local environment="$1"
  local target="$2"
  local telemetry_target="$3"
  local host="$4"
  local telemetry_host="$5"
  local prod_host="$6"
  local site_base_url="$7"
  local api_base_url="$8"
  local cdn_base_url="$9"
  local telemetry_base_url="${10}"
  local dolt_remote_branch="${11}"
  local deployment=""
  local service=""
  local url=""
  local actual_host=""
  local expected_host=""

  case "$environment" in
    beta) deployment="beta" ;;
    production) deployment="production" ;;
    *)
      echo "resident push supports only beta or production deployment_environment, got: ${environment:-<empty>}" >&2
      exit 2
      ;;
  esac

  for service in site api cdn telemetry; do
    case "$service" in
      site) url="$site_base_url" ;;
      api) url="$api_base_url" ;;
      cdn) url="$cdn_base_url" ;;
      telemetry) url="$telemetry_base_url" ;;
    esac
    actual_host="$(url_host "$url")"
    expected_host="$(deployment_expected_public_host "$deployment" "$service")"
    if [[ "$actual_host" != "$expected_host" ]]; then
      echo "unsafe resident push $environment $service URL host: expected $expected_host, got ${actual_host:-<empty>} from $url" >&2
      exit 2
    fi
  done

  case "$deployment" in
    beta)
      if [[ "$host" == "site-nbg1-prod" || "$host" == *production* ]]; then
        echo "unsafe beta resident hostname: $host" >&2
        exit 2
      fi
      if [[ -n "$prod_host" ]]; then
        echo "unsafe beta resident manifest carries production hostname: $prod_host" >&2
        exit 2
      fi
      if deployment_target_mentions_production "$target"; then
        echo "unsafe beta resident target mentions production: $target" >&2
        exit 2
      fi
      if deployment_target_mentions_production "$telemetry_target"; then
        echo "unsafe beta telemetry target mentions production: $telemetry_target" >&2
        exit 2
      fi
      if [[ "$dolt_remote_branch" != "beta" ]]; then
        echo "unsafe beta Dolt branch: expected beta, got ${dolt_remote_branch:-<empty>}" >&2
        exit 2
      fi
      ;;
    production)
      if [[ "$host" == "site-nbg1-beta" || "$host" == *beta* ]]; then
        echo "unsafe production resident hostname: $host" >&2
        exit 2
      fi
      if deployment_target_mentions_beta "$target"; then
        echo "unsafe production resident target mentions beta: $target" >&2
        exit 2
      fi
      if deployment_target_mentions_beta "$telemetry_target"; then
        echo "unsafe production telemetry target mentions beta: $telemetry_target" >&2
        exit 2
      fi
      if [[ "$dolt_remote_branch" != "main" ]]; then
        echo "unsafe production Dolt branch: expected main, got ${dolt_remote_branch:-<empty>}" >&2
        exit 2
      fi
      ;;
  esac
}

deployment_tls_enabled() {
  local deployment
  deployment="$(canonical_deployment_name "$1")"
  case "$deployment" in
    beta) printf '%s' "$(deployment_env_or_default "$deployment" "tls_enabled" "true")" ;;
    production) printf '%s' "$(deployment_env_or_default "$deployment" "tls_enabled" "true")" ;;
    local) printf '%s' "false" ;;
  esac
}

deployment_tls_directory_url() {
  local deployment
  deployment="$(canonical_deployment_name "$1")"
  case "$deployment" in
    beta) printf '%s' "$(deployment_env_or_default "$deployment" "tls_directory_url" "https://acme-v02.api.letsencrypt.org/directory")" ;;
    production) printf '%s' "$(deployment_env_or_default "$deployment" "tls_directory_url" "https://acme-v02.api.letsencrypt.org/directory")" ;;
    local) printf '%s' "https://acme-v02.api.letsencrypt.org/directory" ;;
  esac
}

deployment_tls_challenge() {
  local deployment
  deployment="$(canonical_deployment_name "$1")"
  case "$deployment" in
    beta) printf '%s' "$(deployment_env_or_default "$deployment" "tls_challenge" "dns-01")" ;;
    production) printf '%s' "$(deployment_env_or_default "$deployment" "tls_challenge" "http-01")" ;;
    local) printf '%s' "" ;;
  esac
}

deployment_tls_dns_provider() {
  local deployment
  deployment="$(canonical_deployment_name "$1")"
  case "$deployment" in
    beta) printf '%s' "$(deployment_env_or_default "$deployment" "tls_dns_provider" "cloudflare")" ;;
    production) printf '%s' "$(deployment_env_value "$deployment" "tls_dns_provider")" ;;
    local) printf '%s' "" ;;
  esac
}

deployment_tls_dns_zone() {
  local deployment
  deployment="$(canonical_deployment_name "$1")"
  case "$deployment" in
    beta) printf '%s' "$(deployment_env_or_default "$deployment" "tls_dns_zone" "fishystuff.fish")" ;;
    production) printf '%s' "$(deployment_env_value "$deployment" "tls_dns_zone")" ;;
    local) printf '%s' "" ;;
  esac
}

deployment_tls_acme_email() {
  local deployment
  deployment="$(canonical_deployment_name "$1")"
  case "$deployment" in
    local) printf '%s' "acme@karpfen.dev" ;;
    beta | production) printf '%s' "$(deployment_env_or_default "$deployment" "tls_acme_email" "acme@karpfen.dev")" ;;
  esac
}

deployment_default_services() {
  printf '%s\n' "${RECIPE_DEFAULT_DEPLOYMENT_SERVICES[@]}"
}

deployment_default_mutating_services() {
  printf '%s\n' "${RECIPE_DEFAULT_MUTATING_DEPLOY_SERVICES[@]}"
}

deployment_resident_bundle_services() {
  printf '%s\n' "${RECIPE_RESIDENT_BUNDLE_SERVICES[@]}"
}

extract_ipv4_from_ssh_target() {
  local ssh_target="$1"
  local host_part=""

  host_part="${ssh_target#ssh://}"
  host_part="${host_part#*@}"
  host_part="${host_part%%/*}"
  host_part="${host_part%%:*}"
  if [[ "$host_part" =~ ^([0-9]{1,3}\.){3}[0-9]{1,3}$ ]]; then
    printf '%s' "$host_part"
  fi
}

deploy_service_gcroot_path() {
  local service
  service="$(canonical_public_service_name "$1")"
  case "$service" in
    api) printf '%s' "/nix/var/nix/gcroots/mgmt/fishystuff/api-current" ;;
    dolt) printf '%s' "/nix/var/nix/gcroots/mgmt/fishystuff/dolt-current" ;;
    edge) printf '%s' "/nix/var/nix/gcroots/mgmt/fishystuff/edge-current" ;;
    loki) printf '%s' "/nix/var/nix/gcroots/mgmt/fishystuff/loki-current" ;;
    otel-collector) printf '%s' "/nix/var/nix/gcroots/mgmt/fishystuff/otel-collector-current" ;;
    vector) printf '%s' "/nix/var/nix/gcroots/mgmt/fishystuff/vector-current" ;;
    prometheus) printf '%s' "/nix/var/nix/gcroots/mgmt/fishystuff/prometheus-current" ;;
    jaeger) printf '%s' "/nix/var/nix/gcroots/mgmt/fishystuff/jaeger-current" ;;
    grafana) printf '%s' "/nix/var/nix/gcroots/mgmt/fishystuff/grafana-current" ;;
    site) printf '%s' "/nix/var/nix/gcroots/mgmt/fishystuff/site-content-current" ;;
    cdn) printf '%s' "/nix/var/nix/gcroots/mgmt/fishystuff/cdn-content-current" ;;
    *)
      echo "service $service does not have a deploy gcroot" >&2
      exit 2
      ;;
  esac
}

deploy_service_override_arg_name() {
  local service
  service="$(canonical_public_service_name "$1")"
  case "$service" in
    api) printf '%s' "api_bundle" ;;
    dolt) printf '%s' "dolt_bundle" ;;
    edge) printf '%s' "edge_bundle" ;;
    loki) printf '%s' "loki_bundle" ;;
    otel-collector) printf '%s' "otel_collector_bundle" ;;
    vector) printf '%s' "vector_bundle" ;;
    prometheus) printf '%s' "prometheus_bundle" ;;
    jaeger) printf '%s' "jaeger_bundle" ;;
    grafana) printf '%s' "grafana_bundle" ;;
    site) printf '%s' "site_content" ;;
    cdn) printf '%s' "cdn_content" ;;
    *)
      echo "service $service does not have a deploy override argument" >&2
      exit 2
      ;;
  esac
}

status_service_backing_gcroot_service() {
  local service
  service="$(canonical_public_service_name "$1")"
  case "$service" in
    api | dolt | edge | loki | otel-collector | vector | prometheus | jaeger | grafana)
      printf '%s' "$service"
      ;;
    site | cdn | map | telemetry)
      printf '%s' "edge"
      ;;
    dashboard)
      printf '%s' "grafana"
      ;;
    logs | loki-status)
      printf '%s' "loki"
      ;;
    *)
      printf '%s' ""
      ;;
  esac
}

status_service_bundle_gcroot_path() {
  local backing_service
  backing_service="$(status_service_backing_gcroot_service "$1")"
  if [[ -z "$backing_service" ]]; then
    printf '%s' ""
    return
  fi
  deploy_service_gcroot_path "$backing_service"
}

status_service_content_gcroot_path() {
  local service
  service="$(canonical_public_service_name "$1")"
  case "$service" in
    site) deploy_service_gcroot_path site ;;
    cdn) deploy_service_gcroot_path cdn ;;
    *)
      printf '%s' ""
      ;;
  esac
}

status_service_previous_content_gcroot_path() {
  local current_gcroot=""
  current_gcroot="$(status_service_content_gcroot_path "$1")"
  if [[ -z "$current_gcroot" ]]; then
    printf '%s' ""
    return
  fi
  case "$current_gcroot" in
    *-current) printf '%s-previous' "${current_gcroot%-current}" ;;
    *) printf '%s-previous' "$current_gcroot" ;;
  esac
}

status_service_remote_unit_name() {
  local service
  service="$(canonical_public_service_name "$1")"
  case "$service" in
    api) printf '%s' "fishystuff-api.service" ;;
    dolt) printf '%s' "fishystuff-dolt.service" ;;
    edge | site | cdn | map | telemetry) printf '%s' "fishystuff-edge.service" ;;
    loki | logs | loki-status) printf '%s' "fishystuff-loki.service" ;;
    otel-collector) printf '%s' "fishystuff-otel-collector.service" ;;
    vector) printf '%s' "fishystuff-vector.service" ;;
    prometheus) printf '%s' "fishystuff-prometheus.service" ;;
    jaeger) printf '%s' "fishystuff-jaeger.service" ;;
    grafana | dashboard) printf '%s' "fishystuff-grafana.service" ;;
    *)
      printf '%s' ""
      ;;
  esac
}

status_service_direct_url() {
  local deployment
  local service

  deployment="$(canonical_deployment_name "$1")"
  service="$(canonical_public_service_name "$2")"
  if [[ "$deployment" == "local" ]]; then
    case "$service" in
      site | map | api | cdn | telemetry | grafana | dashboard | loki | logs | loki-status | prometheus | vector | jaeger)
        deployment_open_url "$deployment" "$service"
        ;;
      *)
        printf '%s' ""
        ;;
    esac
    return
  fi

  case "$service" in
    site | map | api | cdn | telemetry)
      deployment_open_url "$deployment" "$service"
      ;;
    *)
      printf '%s' ""
      ;;
  esac
}

status_service_local_probe_port() {
  local service
  service="$(canonical_public_service_name "$1")"
  case "$service" in
    site | map | telemetry | edge) printf '%s' "1990" ;;
    api) printf '%s' "8080" ;;
    cdn) printf '%s' "4040" ;;
    dolt) printf '%s' "3306" ;;
    grafana | dashboard | loki | logs) printf '%s' "3000" ;;
    loki-status) printf '%s' "3100" ;;
    prometheus) printf '%s' "9090" ;;
    vector) printf '%s' "8686" ;;
    jaeger) printf '%s' "16686" ;;
    *)
      printf '%s' ""
      ;;
  esac
}

deployment_open_tunnel_remote_port() {
  local service
  service="$(canonical_public_service_name "$1")"
  case "$service" in
    grafana | dashboard | loki | logs) printf '%s' "3000" ;;
    loki-status) printf '%s' "3100" ;;
    prometheus) printf '%s' "9090" ;;
    vector) printf '%s' "8686" ;;
    jaeger) printf '%s' "16686" ;;
    *)
      printf '%s' ""
      ;;
  esac
}

deployment_open_tunnel_local_port() {
  local service
  service="$(canonical_public_service_name "$1")"
  case "$service" in
    grafana) printf '%s' "3300" ;;
    dashboard) printf '%s' "3301" ;;
    loki | logs) printf '%s' "3302" ;;
    loki-status) printf '%s' "3310" ;;
    prometheus) printf '%s' "3909" ;;
    vector) printf '%s' "3868" ;;
    jaeger) printf '%s' "3368" ;;
    *)
      printf '%s' ""
      ;;
  esac
}

deployment_open_tunnel_ttl_seconds() {
  local deployment
  deployment="$(canonical_deployment_name "$1")"
  case "$deployment" in
    local) printf '%s' "0" ;;
    beta | production) printf '%s' "$(deployment_env_or_default "$deployment" "open_tunnel_ttl_seconds" "900")" ;;
  esac
}

deployment_open_tunnel_url() {
  local deployment
  local service
  local local_port
  local environment
  local encoded_environment

  deployment="$(canonical_deployment_name "$1")"
  service="$(canonical_public_service_name "$2")"
  local_port="${3:-$(deployment_open_tunnel_local_port "$service")}"
  case "$service" in
    grafana | dashboard)
      environment="$(deployment_environment_name "$deployment")"
      encoded_environment="${environment// /%20}"
      printf 'http://127.0.0.1:%s/d/fishystuff-operator-overview/fishystuff-operator-overview?orgId=1&var-env=%s' "$local_port" "$encoded_environment"
      ;;
    loki | logs)
      environment="$(deployment_environment_name "$deployment")"
      encoded_environment="${environment// /%20}"
      printf 'http://127.0.0.1:%s/d/fishystuff-operator-overview/fishystuff-operator-overview?orgId=1&var-env=%s&viewPanel=17' "$local_port" "$encoded_environment"
      ;;
    loki-status) printf 'http://127.0.0.1:%s/services' "$local_port" ;;
    prometheus | vector | jaeger) printf 'http://127.0.0.1:%s/' "$local_port" ;;
    *)
      printf '%s' ""
      ;;
  esac
}

local_port_is_listening() {
  local port="$1"
  ss -ltnH "( sport = :${port} )" 2>/dev/null | grep -q .
}

merge_json_env_from_keys() {
  local base_json="$1"
  local pairs_csv="$2"
  local merged_json="$base_json"
  local -a env_entries=()
  local entry=""
  local key=""
  local env_name=""
  local value=""

  [[ -n "$pairs_csv" ]] || {
    printf '%s' "$merged_json"
    return
  }

  IFS=',' read -r -a env_entries <<< "$pairs_csv"
  for entry in "${env_entries[@]}"; do
    [[ -n "$entry" ]] || continue
    key="${entry%%=*}"
    env_name="${entry#*=}"
    if [[ "$entry" != *=* ]]; then
      env_name="$entry"
    fi
    if [[ -z "$key" || -z "$env_name" ]]; then
      echo "invalid key/env entry: $entry" >&2
      exit 2
    fi
    value="${!env_name:-}"
    if [[ -z "$value" ]]; then
      echo "missing environment variable for entry: $entry" >&2
      exit 2
    fi
    merged_json="$(
      jq -cn \
        --argjson current "$merged_json" \
        --arg key "$key" \
        --arg value "$value" \
        '$current + {($key): $value}'
    )"
  done

  printf '%s' "$merged_json"
}

create_temp_ssh_key_from_env() {
  local prefix="${1:-/tmp/fishystuff-ssh.XXXXXX}"
  local tmp_key=""

  tmp_key="$(mktemp "$prefix")"
  umask 077
  printf '%s\n' "${HETZNER_SSH_PRIVATE_KEY:?}" > "$tmp_key"
  chmod 600 "$tmp_key"
  printf '%s' "$tmp_key"
}

detect_remote_nix_probe() {
  local ssh_target="$1"
  local tmp_key="$2"

  ssh \
    -i "$tmp_key" \
    -o IdentitiesOnly=yes \
    -o StrictHostKeyChecking=accept-new \
    "$ssh_target" \
    '
      nix_path=""
      nix_daemon_path=""
      if test -x /nix/var/nix/profiles/default/bin/nix; then
        nix_path=/nix/var/nix/profiles/default/bin/nix
      elif command -v nix >/dev/null 2>&1; then
        nix_path="$(command -v nix)"
      fi
      if test -x /nix/var/nix/profiles/default/bin/nix-daemon; then
        nix_daemon_path=/nix/var/nix/profiles/default/bin/nix-daemon
      elif command -v nix-daemon >/dev/null 2>&1; then
        nix_daemon_path="$(command -v nix-daemon)"
      fi
      printf "%s\t%s\n" "$nix_path" "$nix_daemon_path"
    ' \
    2>/dev/null || true
}

read_remote_nix_paths() {
  local ssh_target="$1"
  local tmp_key="$2"
  local probe=""
  local nix_path=""
  local nix_daemon_path=""

  probe="$(detect_remote_nix_probe "$ssh_target" "$tmp_key")"
  if [[ -n "$probe" ]]; then
    IFS=$'\t' read -r nix_path nix_daemon_path <<< "$probe"
  fi
  printf '%s\t%s\n' "$nix_path" "$nix_daemon_path"
}

detect_remote_nix_daemon_path() {
  local ssh_target="$1"
  local tmp_key="$2"
  local nix_probe=""
  local nix_daemon_path=""

  nix_probe="$(read_remote_nix_paths "$ssh_target" "$tmp_key")"
  if [[ -n "$nix_probe" ]]; then
    IFS=$'\t' read -r _nix_path nix_daemon_path <<< "$nix_probe"
  fi
  printf '%s' "$nix_daemon_path"
}

build_nix_copy_target() {
  local ssh_target="$1"
  local tmp_key="$2"
  local remote_program="${3-}"
  local target="ssh-ng://$ssh_target?ssh-key=$tmp_key"

  if [[ -n "$remote_program" ]]; then
    target="${target}&remote-program=$remote_program"
  fi
  printf '%s' "$target"
}

copy_resident_common_modules() {
  local deploy_dir="$1"
  local mgmt_modules_dir="$2"
  local module_name=""

  mkdir -p "$deploy_dir/modules/lib" "$deploy_dir/modules/providers"
  for module_name in fishystuff-beta-access systemd-daemon-reload; do
    cp -a "$RECIPE_REPO_ROOT/mgmt/modules/lib/$module_name" "$deploy_dir/modules/lib/"
  done
  cp -a "$RECIPE_REPO_ROOT/mgmt/modules/providers/cloudflare-dnsmanager" "$deploy_dir/modules/providers/"
  cp -a "$RECIPE_REPO_ROOT/mgmt/modules/providers/hetzner-firewall" "$deploy_dir/modules/providers/"
}
