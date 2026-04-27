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
  shift
  if [[ -n "$profile" && -z "${HETZNER_SSH_PRIVATE_KEY:-}" ]]; then
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
      grafana | loki | logs) printf '%s' "http://127.0.0.1:3000/explore" ;;
      dashboard) printf '%s' "http://127.0.0.1:3000/d/fishystuff-local-observability/fishystuff-local-observability" ;;
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
    production) printf '%s' "$(deployment_env_value "$deployment" "resident_hostname")" ;;
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

deployment_telemetry_target() {
  local deployment
  deployment="$(canonical_deployment_name "$1")"
  case "$deployment" in
    beta) printf '%s' "$(deployment_env_or_default "$deployment" "telemetry_target" "root@telemetry.beta.fishystuff.fish")" ;;
    production) printf '%s' "$(deployment_env_value "$deployment" "telemetry_target")" ;;
    local) printf '%s' "" ;;
  esac
}

deployment_control_target() {
  local deployment
  deployment="$(canonical_deployment_name "$1")"
  case "$deployment" in
    beta) printf '%s' "$(deployment_env_or_default "$deployment" "control_target" "mgmt-root")" ;;
    production) printf '%s' "$(deployment_env_or_default "$deployment" "control_target" "$(deployment_resident_target "$deployment")")" ;;
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
    beta) printf '%s' "$(deployment_env_or_default "$deployment" "prod_hostname" "site-nbg1-prod")" ;;
    production) printf '%s' "$(deployment_env_value "$deployment" "prod_hostname")" ;;
    local) printf '%s' "" ;;
  esac
}

deployment_tunnel_target() {
  local deployment
  deployment="$(canonical_deployment_name "$1")"
  printf '%s' "$(deployment_env_or_default "$deployment" "tunnel_target" "$(deployment_resident_target "$deployment")")"
}

deployment_secretspec_profile() {
  local deployment
  deployment="$(canonical_deployment_name "$1")"
  case "$deployment" in
    beta) printf '%s' "beta-deploy" ;;
    production | local) printf '%s' "" ;;
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

deployment_resident_bundle_services() {
  printf '%s\n' "${RECIPE_RESIDENT_BUNDLE_SERVICES[@]}"
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
  local service
  local local_port

  service="$(canonical_public_service_name "$1")"
  local_port="${2:-$(deployment_open_tunnel_local_port "$service")}"
  case "$service" in
    grafana | loki | logs) printf 'http://127.0.0.1:%s/explore' "$local_port" ;;
    dashboard) printf 'http://127.0.0.1:%s/d/fishystuff-operator-overview/fishystuff-operator-overview' "$local_port" ;;
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
