#!/usr/bin/env bash
set -euo pipefail

SCRIPT_PATH="$(readlink -f "${BASH_SOURCE[0]}")"
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "${SCRIPT_DIR}/lib/common.sh"

cd "$RECIPE_REPO_ROOT"

exec_with_secretspec_profile_if_needed "$(operator_secretspec_profile)" bash "$SCRIPT_PATH" "$@"

target=""
telemetry_target=""
deploy_target=""
host="site-nbg1-beta"
telemetry_host="telemetry-nbg1"
prod_host="site-nbg1-prod"
timeout="180"
remote_mgmt_bin="/usr/local/bin/mgmt"
api_gcroot="/nix/var/nix/gcroots/mgmt/fishystuff/api-current"
dolt_gcroot="/nix/var/nix/gcroots/mgmt/fishystuff/dolt-current"
edge_gcroot="/nix/var/nix/gcroots/mgmt/fishystuff/edge-current"
loki_gcroot="/nix/var/nix/gcroots/mgmt/fishystuff/loki-current"
otel_collector_gcroot="/nix/var/nix/gcroots/mgmt/fishystuff/otel-collector-current"
vector_gcroot="/nix/var/nix/gcroots/mgmt/fishystuff/vector-current"
prometheus_gcroot="/nix/var/nix/gcroots/mgmt/fishystuff/prometheus-current"
jaeger_gcroot="/nix/var/nix/gcroots/mgmt/fishystuff/jaeger-current"
grafana_gcroot="/nix/var/nix/gcroots/mgmt/fishystuff/grafana-current"
cdn_content_gcroot="/nix/var/nix/gcroots/mgmt/fishystuff/cdn-content-current"
cdn_content_mode="realise"
mgmt_modules_dir="/home/carp/code/mgmt-fishystuff-beta/modules"
remote_nix_max_jobs="0"
services_csv="api,dolt,edge,loki,otel_collector,vector,prometheus,jaeger,grafana"
deployment_environment="beta"
deployment_marker=""
api_bundle_override=""
dolt_bundle_override=""
edge_bundle_override=""
loki_bundle_override=""
otel_collector_bundle_override=""
vector_bundle_override=""
prometheus_bundle_override=""
jaeger_bundle_override=""
grafana_bundle_override=""
site_content_override=""
cdn_content_override=""
tls_enabled="${FISHYSTUFF_BETA_TLS_ENABLED:-true}"
tls_certificate_name=""
tls_acme_email="${FISHYSTUFF_BETA_TLS_ACME_EMAIL:-acme@karpfen.dev}"
tls_challenge="${FISHYSTUFF_BETA_TLS_CHALLENGE:-dns-01}"
tls_dns_provider="${FISHYSTUFF_BETA_TLS_DNS_PROVIDER:-cloudflare}"
tls_dns_env_json="$(
  jq -cn \
    --arg zone "${FISHYSTUFF_BETA_TLS_DNS_ZONE:-fishystuff.fish}" \
    '{CLOUDFLARE_ZONE_NAME: $zone}'
)"
tls_dns_env_keys_csv=""
tls_directory_url="${FISHYSTUFF_BETA_TLS_DIRECTORY_URL:-https://acme-v02.api.letsencrypt.org/directory}"
tls_domains_json=""
site_base_url_override=""
api_base_url_override=""
cdn_base_url_override=""
telemetry_base_url_override=""

raw_args="$*"
IFS=" " read -r -a overrides <<< "$raw_args"
for arg in "${overrides[@]}"; do
  [[ -n "$arg" ]] || continue
  case "$arg" in
    target=*) target="${arg#target=}" ;;
    telemetry_target=*) telemetry_target="${arg#telemetry_target=}" ;;
    deploy_target=*) deploy_target="${arg#deploy_target=}" ;;
    host=*) host="${arg#host=}" ;;
    telemetry_host=*) telemetry_host="${arg#telemetry_host=}" ;;
    prod_host=*) prod_host="${arg#prod_host=}" ;;
    timeout=*) timeout="${arg#timeout=}" ;;
    remote_mgmt_bin=*) remote_mgmt_bin="${arg#remote_mgmt_bin=}" ;;
    api_gcroot=*) api_gcroot="${arg#api_gcroot=}" ;;
    dolt_gcroot=*) dolt_gcroot="${arg#dolt_gcroot=}" ;;
    edge_gcroot=*) edge_gcroot="${arg#edge_gcroot=}" ;;
    loki_gcroot=*) loki_gcroot="${arg#loki_gcroot=}" ;;
    otel_collector_gcroot=*) otel_collector_gcroot="${arg#otel_collector_gcroot=}" ;;
    vector_gcroot=*) vector_gcroot="${arg#vector_gcroot=}" ;;
    prometheus_gcroot=*) prometheus_gcroot="${arg#prometheus_gcroot=}" ;;
    jaeger_gcroot=*) jaeger_gcroot="${arg#jaeger_gcroot=}" ;;
    grafana_gcroot=*) grafana_gcroot="${arg#grafana_gcroot=}" ;;
    cdn_content_gcroot=*) cdn_content_gcroot="${arg#cdn_content_gcroot=}" ;;
    cdn_content=*) cdn_content_override="${arg#cdn_content=}" ;;
    cdn_content_mode=*) cdn_content_mode="${arg#cdn_content_mode=}" ;;
    mgmt_modules_dir=*) mgmt_modules_dir="${arg#mgmt_modules_dir=}" ;;
    remote_nix_max_jobs=*) remote_nix_max_jobs="${arg#remote_nix_max_jobs=}" ;;
    services_csv=*) services_csv="${arg#services_csv=}" ;;
    deployment_environment=*) deployment_environment="${arg#deployment_environment=}" ;;
    deployment_marker=*) deployment_marker="${arg#deployment_marker=}" ;;
    api_bundle=*) api_bundle_override="${arg#api_bundle=}" ;;
    dolt_bundle=*) dolt_bundle_override="${arg#dolt_bundle=}" ;;
    edge_bundle=*) edge_bundle_override="${arg#edge_bundle=}" ;;
    loki_bundle=*) loki_bundle_override="${arg#loki_bundle=}" ;;
    otel_collector_bundle=*) otel_collector_bundle_override="${arg#otel_collector_bundle=}" ;;
    vector_bundle=*) vector_bundle_override="${arg#vector_bundle=}" ;;
    prometheus_bundle=*) prometheus_bundle_override="${arg#prometheus_bundle=}" ;;
    jaeger_bundle=*) jaeger_bundle_override="${arg#jaeger_bundle=}" ;;
    grafana_bundle=*) grafana_bundle_override="${arg#grafana_bundle=}" ;;
    site_content=*) site_content_override="${arg#site_content=}" ;;
    tls_enabled=*) tls_enabled="${arg#tls_enabled=}" ;;
    tls_certificate_name=*) tls_certificate_name="${arg#tls_certificate_name=}" ;;
    tls_acme_email=*) tls_acme_email="${arg#tls_acme_email=}" ;;
    tls_challenge=*) tls_challenge="${arg#tls_challenge=}" ;;
    tls_dns_provider=*) tls_dns_provider="${arg#tls_dns_provider=}" ;;
    tls_dns_env_json=*) tls_dns_env_json="${arg#tls_dns_env_json=}" ;;
    tls_dns_env_keys_csv=*) tls_dns_env_keys_csv="${arg#tls_dns_env_keys_csv=}" ;;
    tls_directory_url=*) tls_directory_url="${arg#tls_directory_url=}" ;;
    tls_domains_json=*) tls_domains_json="${arg#tls_domains_json=}" ;;
    site_base_url=*) site_base_url_override="${arg#site_base_url=}" ;;
    api_base_url=*) api_base_url_override="${arg#api_base_url=}" ;;
    cdn_base_url=*) cdn_base_url_override="${arg#cdn_base_url=}" ;;
    telemetry_base_url=*) telemetry_base_url_override="${arg#telemetry_base_url=}" ;;
    *)
      echo "unknown override for mgmt-resident-push-full-stack: $arg" >&2
      exit 2
      ;;
  esac
done

build_release_map_runtime() {
  local site_content_path="$1"
  local runtime_config_path="$site_content_path/runtime-config.js"
  local map_asset_cache_key=""

  if [[ ! -f "$runtime_config_path" ]]; then
    echo "runtime-config.js missing from site content: $runtime_config_path" >&2
    exit 2
  fi

  map_asset_cache_key="$(
    node ./tools/scripts/print_runtime_map_asset_cache_key.mjs "$runtime_config_path"
  )"
  if [[ -z "$map_asset_cache_key" ]]; then
    echo "failed to resolve mapAssetCacheKey from $runtime_config_path" >&2
    exit 2
  fi

  echo "rebuilding map runtime for cache key: $map_asset_cache_key" >&2
  FISHYSTUFF_RUNTIME_MAP_ASSET_CACHE_KEY="$map_asset_cache_key" \
    ./tools/scripts/build_map.sh
  ./tools/scripts/stage_cdn_assets.sh --map-only
}

deployment_environment="$(normalize_deployment_environment "$deployment_environment")"
if [[ -z "$deployment_marker" ]]; then
  deployment_marker="$(date -u +%Y%m%dT%H%M%SZ)-$deployment_environment-$RANDOM-$RANDOM"
fi
deployment_domain_name="$(deployment_domain "$deployment_environment")"
site_base_url="${site_base_url_override:-https://$deployment_domain_name}"
api_base_url="${api_base_url_override:-https://api.$deployment_domain_name}"
cdn_base_url="${cdn_base_url_override:-https://cdn.$deployment_domain_name}"
telemetry_base_url="${telemetry_base_url_override:-https://telemetry.$deployment_domain_name}"
if [[ "$tls_challenge" == "dns-01" && "$tls_dns_provider" == "cloudflare" && -z "$tls_dns_env_keys_csv" && -n "${CLOUDFLARE_API_TOKEN:-}" ]]; then
  tls_dns_env_keys_csv="CLOUDFLARE_API_TOKEN"
fi
tls_dns_env_json="$(merge_json_env_from_keys "$tls_dns_env_json" "$tls_dns_env_keys_csv")"
if [[ -z "$tls_domains_json" ]]; then
  site_tls_domain="${site_base_url#https://}"
  site_tls_domain="${site_tls_domain%/}"
  api_tls_domain="${api_base_url#https://}"
  api_tls_domain="${api_tls_domain%/}"
  cdn_tls_domain="${cdn_base_url#https://}"
  cdn_tls_domain="${cdn_tls_domain%/}"
  telemetry_tls_domain="${telemetry_base_url#https://}"
  telemetry_tls_domain="${telemetry_tls_domain%/}"
  tls_domains_json="$(
    jq -cn \
      --arg site "$site_tls_domain" \
      --arg api "$api_tls_domain" \
      --arg cdn "$cdn_tls_domain" \
      --arg telemetry "$telemetry_tls_domain" \
      '[$site, $api, $cdn, $telemetry]'
  )"
fi

services_csv="${services_csv//[[:space:]]/}"
require_value "$target" "missing target=... for mgmt-resident-push-full-stack"
deploy_target="${deploy_target:-$target}"

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

site_public_ipv4_hint="$(extract_ipv4_from_ssh_target "$target")"
telemetry_public_ipv4_hint=""
if [[ -n "$telemetry_target" ]]; then
  telemetry_public_ipv4_hint="$(extract_ipv4_from_ssh_target "$telemetry_target")"
fi

operator_repo_root="$PWD"
declare -A selected_services=()
IFS=',' read -r -a requested_services <<< "$services_csv"
for service_name in "${requested_services[@]}"; do
  [[ -n "$service_name" ]] || continue
  case "$service_name" in
    api | dolt | edge | loki | otel_collector | vector | prometheus | jaeger | grafana)
      selected_services["$service_name"]=1
      ;;
    *)
      echo "unknown service name in services_csv: $service_name" >&2
      exit 2
      ;;
  esac
done

service_selected() {
  [[ -n "${selected_services[$1]:-}" ]]
}

bundle_is_remote_only() {
  local bundle_path="$1"
  local override_path=""
  for override_path in \
    "$api_bundle_override" \
    "$dolt_bundle_override" \
    "$edge_bundle_override" \
    "$loki_bundle_override" \
    "$otel_collector_bundle_override" \
    "$vector_bundle_override" \
    "$prometheus_bundle_override" \
    "$jaeger_bundle_override" \
    "$grafana_bundle_override"; do
    if [[ -n "$override_path" && "$bundle_path" == "$override_path" && ! -e "$bundle_path" ]]; then
      return 0
    fi
  done
  return 1
}

content_is_remote_only() {
  local content_path="$1"
  local override_path=""
  for override_path in "$site_content_override" "$cdn_content_override"; do
    if [[ -n "$override_path" && "$content_path" == "$override_path" && ! -e "$content_path" ]]; then
      return 0
    fi
  done
  return 1
}

if ! service_selected api || ! service_selected dolt; then
  echo "services_csv must include both api and dolt for mgmt-resident-push-full-stack" >&2
  exit 2
fi

api_bundle="$api_bundle_override"
dolt_bundle="$dolt_bundle_override"
edge_bundle="$edge_bundle_override"
loki_bundle="$loki_bundle_override"
otel_collector_bundle="$otel_collector_bundle_override"
vector_bundle="$vector_bundle_override"
prometheus_bundle="$prometheus_bundle_override"
jaeger_bundle="$jaeger_bundle_override"
grafana_bundle="$grafana_bundle_override"
site_content=""
cdn_base_content=""
cdn_content=""
cdn_content_drv=""
minimap_display_tiles=""
minimap_source_tiles=""

if service_selected api && [[ -z "$api_bundle" ]]; then
  api_bundle="$(nix build .#api-service-bundle --no-link --print-out-paths)"
fi
if service_selected dolt && [[ -z "$dolt_bundle" ]]; then
  dolt_bundle="$(nix build .#dolt-service-bundle --no-link --print-out-paths)"
fi
if service_selected edge; then
  if [[ -z "$edge_bundle" ]]; then
    edge_bundle="$(nix build .#edge-service-bundle --no-link --print-out-paths)"
  fi
  if [[ -n "$site_content_override" ]]; then
    case "$site_content_override" in
      /nix/store/*)
        site_content="$site_content_override"
        ;;
      *)
        if [[ ! -e "$site_content_override" ]]; then
          echo "site_content path does not exist locally: $site_content_override" >&2
          exit 2
        fi
        site_content="$(nix store add-path --name "fishystuff-site-content-$deployment_environment" "$site_content_override")"
        ;;
    esac
  else
    case "$deployment_environment" in
      beta) site_content_package="site-content-beta" ;;
      production) site_content_package="site-content" ;;
      *)
        echo "site_content must be provided explicitly for deployment_environment=$deployment_environment" >&2
        exit 2
        ;;
    esac
    site_content="$(nix build ".#$site_content_package" --no-link --print-out-paths)"
  fi
  if [[ -n "$cdn_content_override" ]]; then
    case "$cdn_content_override" in
      /nix/store/*)
        cdn_content="$cdn_content_override"
        cdn_content_drv=""
        ;;
      *)
        if [[ ! -e "$cdn_content_override" ]]; then
          echo "cdn_content path does not exist locally: $cdn_content_override" >&2
          exit 2
        fi
        cdn_content="$(nix store add-path --name "fishystuff-cdn-content-$deployment_environment" "$cdn_content_override")"
        cdn_content_drv=""
        ;;
    esac
  else
    build_release_map_runtime "$site_content"
    case "$cdn_content_mode" in
      local | substitute)
        cdn_content="$(
          FISHYSTUFF_OPERATOR_ROOT="$operator_repo_root" \
            nix build --impure .#cdn-content --no-link --print-out-paths
        )"
        cdn_content_drv=""
        ;;
      realise)
        minimap_display_tiles="$(nix build .#minimap-display-tiles --no-link --print-out-paths)"
        readarray -t cdn_operator_paths < <(
          FISHYSTUFF_OPERATOR_ROOT="$operator_repo_root" \
            nix build --impure \
              .#cdn-base-content \
              .#minimap-source-tiles \
              --no-link \
              --print-out-paths
        )
        cdn_base_content="${cdn_operator_paths[0]:-}"
        minimap_source_tiles="${cdn_operator_paths[1]:-}"
        cdn_content_drv="$(
          FISHYSTUFF_OPERATOR_ROOT="$operator_repo_root" \
            nix path-info --impure .#cdn-content --derivation
        )"
        cdn_content="$(nix derivation show "$cdn_content_drv" | jq -r 'to_entries[0].value.outputs.out.path')"
        ;;
      *)
        echo "unknown cdn_content_mode for mgmt-resident-push-full-stack: $cdn_content_mode" >&2
        exit 2
        ;;
    esac
  fi
fi
if service_selected loki && [[ -z "$loki_bundle" ]]; then
  loki_bundle="$(nix build .#loki-service-bundle --no-link --print-out-paths)"
fi
if service_selected otel_collector && [[ -z "$otel_collector_bundle" ]]; then
  otel_collector_bundle="$(nix build .#otel-collector-service-bundle --no-link --print-out-paths)"
fi
if service_selected vector && [[ -z "$vector_bundle" ]]; then
  vector_bundle="$(nix build .#vector-service-bundle --no-link --print-out-paths)"
fi
if service_selected prometheus && [[ -z "$prometheus_bundle" ]]; then
  prometheus_bundle="$(nix build .#prometheus-service-bundle --no-link --print-out-paths)"
fi
if service_selected jaeger && [[ -z "$jaeger_bundle" ]]; then
  jaeger_bundle="$(nix build .#jaeger-service-bundle --no-link --print-out-paths)"
fi
if service_selected grafana && [[ -z "$grafana_bundle" ]]; then
  grafana_bundle="$(nix build .#grafana-service-bundle --no-link --print-out-paths)"
fi

site_push_paths=()
telemetry_push_paths=()
declare -A site_push_seen=()
declare -A telemetry_push_seen=()

add_push_path() {
  local array_name="$1"
  local seen_name="$2"
  local path="$3"
  local -n push_array="$array_name"
  local -n seen_paths="$seen_name"

  [[ -n "$path" ]] || return 0
  if [[ -n "${seen_paths[$path]+x}" ]]; then
    return 0
  fi
  seen_paths["$path"]=1
  push_array+=("$path")
}

add_bundle_push_path() {
  local array_name="$1"
  local seen_name="$2"
  local bundle_path="$3"

  [[ -n "$bundle_path" ]] || return 0
  if bundle_is_remote_only "$bundle_path"; then
    echo "[resident-push] reusing existing remote bundle path without local push: $bundle_path"
    return 0
  fi
  if [[ -e "$bundle_path" ]]; then
    add_push_path "$array_name" "$seen_name" "$bundle_path"
    return 0
  fi
  echo "bundle path does not exist locally: $bundle_path" >&2
  exit 2
}

add_content_push_path() {
  local array_name="$1"
  local seen_name="$2"
  local path_to_push="$3"

  [[ -n "$path_to_push" ]] || return 0
  if content_is_remote_only "$path_to_push"; then
    echo "[resident-push] reusing existing remote content path without local push: $path_to_push"
    return 0
  fi
  if [[ ! -e "$path_to_push" ]]; then
    echo "content path does not exist locally: $path_to_push" >&2
    exit 2
  fi
  add_push_path "$array_name" "$seen_name" "$path_to_push"
}

add_bundle_push_path site_push_paths site_push_seen "$api_bundle"
add_bundle_push_path site_push_paths site_push_seen "$dolt_bundle"
add_bundle_push_path site_push_paths site_push_seen "$edge_bundle"
add_content_push_path site_push_paths site_push_seen "$site_content"
add_content_push_path site_push_paths site_push_seen "$cdn_base_content"
add_content_push_path site_push_paths site_push_seen "$minimap_display_tiles"
add_content_push_path site_push_paths site_push_seen "$minimap_source_tiles"
add_content_push_path site_push_paths site_push_seen "$cdn_content_drv"
if [[ -n "$cdn_content" && -z "$cdn_content_drv" ]]; then
  if content_is_remote_only "$cdn_content"; then
    echo "[resident-push] reusing existing remote content path without local push: $cdn_content"
  elif [[ -e "$cdn_content" ]]; then
    add_push_path site_push_paths site_push_seen "$cdn_content"
  else
    echo "content path does not exist locally: $cdn_content" >&2
    exit 2
  fi
fi

if [[ -n "$telemetry_target" ]]; then
  add_bundle_push_path telemetry_push_paths telemetry_push_seen "$edge_bundle"
  add_bundle_push_path telemetry_push_paths telemetry_push_seen "$loki_bundle"
  add_bundle_push_path telemetry_push_paths telemetry_push_seen "$otel_collector_bundle"
  add_bundle_push_path telemetry_push_paths telemetry_push_seen "$vector_bundle"
  add_bundle_push_path telemetry_push_paths telemetry_push_seen "$prometheus_bundle"
  add_bundle_push_path telemetry_push_paths telemetry_push_seen "$jaeger_bundle"
  add_bundle_push_path telemetry_push_paths telemetry_push_seen "$grafana_bundle"
fi

deploy_dir="$(mktemp -d /tmp/fishystuff-resident-full-stack.XXXXXX)"
trap 'rm -rf "$deploy_dir"' EXIT
cp -a mgmt/resident-beta/. "$deploy_dir/"
mkdir -p "$deploy_dir/files"
copy_resident_common_modules "$deploy_dir" "$mgmt_modules_dir"

jq -n \
  --arg cluster "beta" \
  --arg deployment_marker "$deployment_marker" \
  --arg hostname "$host" \
  --arg telemetry_hostname "$telemetry_host" \
  --arg prod_hostname "$prod_host" \
  --arg site_base_url "$site_base_url" \
  --arg api_base_url "$api_base_url" \
  --arg cdn_base_url "$cdn_base_url" \
  --arg telemetry_base_url "$telemetry_base_url" \
  --arg site_public_ipv4_hint "$site_public_ipv4_hint" \
  --arg telemetry_public_ipv4_hint "$telemetry_public_ipv4_hint" \
  --arg deployment_environment "$deployment_environment" \
  --arg startup_mode "enabled" \
  --arg dolt_data_dir "/var/lib/fishystuff/dolt" \
  --arg dolt_cfg_dir "/var/lib/fishystuff/dolt/.doltcfg" \
  --arg dolt_database_name "fishystuff" \
  --arg dolt_remote_url "fishystuff/fishystuff" \
  --arg dolt_clone_depth "1" \
  --arg dolt_volume_device "" \
  --arg dolt_volume_fs_type "ext4" \
  --arg dolt_port "3306" \
  --arg site_root_dir "/srv/fishystuff/site" \
  --arg cdn_root_dir "/srv/fishystuff/cdn" \
  --arg site_content "$site_content" \
  --arg site_content_gcroot "/nix/var/nix/gcroots/mgmt/fishystuff/site-content-current" \
  --argjson tls_enabled "$tls_enabled" \
  --arg tls_certificate_name "$tls_certificate_name" \
  --arg tls_acme_email "$tls_acme_email" \
  --arg tls_challenge "$tls_challenge" \
  --arg tls_dns_provider "$tls_dns_provider" \
  --argjson tls_dns_env "$tls_dns_env_json" \
  --arg tls_directory_url "$tls_directory_url" \
  --argjson tls_domains "$tls_domains_json" \
  --arg api_bundle "$api_bundle" \
  --arg api_gcroot "$api_gcroot" \
  --arg dolt_bundle "$dolt_bundle" \
  --arg dolt_gcroot "$dolt_gcroot" \
  --arg edge_bundle "$edge_bundle" \
  --arg edge_gcroot "$edge_gcroot" \
  --arg loki_bundle "$loki_bundle" \
  --arg loki_gcroot "$loki_gcroot" \
  --arg otel_collector_bundle "$otel_collector_bundle" \
  --arg otel_collector_gcroot "$otel_collector_gcroot" \
  --arg vector_bundle "$vector_bundle" \
  --arg vector_gcroot "$vector_gcroot" \
  --arg prometheus_bundle "$prometheus_bundle" \
  --arg prometheus_gcroot "$prometheus_gcroot" \
  --arg jaeger_bundle "$jaeger_bundle" \
  --arg jaeger_gcroot "$jaeger_gcroot" \
  --arg grafana_bundle "$grafana_bundle" \
  --arg grafana_gcroot "$grafana_gcroot" \
  --arg cdn_content "$cdn_content" \
  --arg cdn_content_drv "$cdn_content_drv" \
  --arg cdn_content_gcroot "$cdn_content_gcroot" \
  '{
    cluster: $cluster,
    deployment_marker: $deployment_marker,
    hostname: $hostname,
    telemetry_hostname: $telemetry_hostname,
    prod_hostname: $prod_hostname,
    public_urls: {
      site_base_url: $site_base_url,
      api_base_url: $api_base_url,
      cdn_base_url: $cdn_base_url,
      telemetry_base_url: $telemetry_base_url
    },
    host_hints: {
      site_public_ipv4: $site_public_ipv4_hint,
      telemetry_public_ipv4: $telemetry_public_ipv4_hint
    },
    deployment_environment: $deployment_environment,
    startup_mode: $startup_mode,
    dolt: {
      data_dir: $dolt_data_dir,
      cfg_dir: $dolt_cfg_dir,
      database_name: $dolt_database_name,
      remote_url: $dolt_remote_url,
      clone_depth: $dolt_clone_depth,
      volume_device: $dolt_volume_device,
      volume_fs_type: $dolt_volume_fs_type,
      port: $dolt_port
    },
    content_roots: {
      site_root_dir: $site_root_dir,
      cdn_root_dir: $cdn_root_dir
    },
    content: {
      site: {
        store_path: $site_content,
        drv_path: "",
        gcroot_path: $site_content_gcroot
      },
      cdn: {
        store_path: $cdn_content,
        drv_path: $cdn_content_drv,
        gcroot_path: $cdn_content_gcroot
      }
    },
    tls: {
      enabled: $tls_enabled,
      certificate_name: $tls_certificate_name,
      acme_email: $tls_acme_email,
      challenge: $tls_challenge,
      dns_provider: $tls_dns_provider,
      dns_env: $tls_dns_env,
      directory_url: $tls_directory_url,
      domains: $tls_domains
    },
    services: {
      api: {bundle_path: $api_bundle, gcroot_path: $api_gcroot},
      dolt: {bundle_path: $dolt_bundle, gcroot_path: $dolt_gcroot},
      edge: {bundle_path: $edge_bundle, gcroot_path: $edge_gcroot},
      loki: {bundle_path: $loki_bundle, gcroot_path: $loki_gcroot},
      otel_collector: {bundle_path: $otel_collector_bundle, gcroot_path: $otel_collector_gcroot},
      vector: {bundle_path: $vector_bundle, gcroot_path: $vector_gcroot},
      prometheus: {bundle_path: $prometheus_bundle, gcroot_path: $prometheus_gcroot},
      jaeger: {bundle_path: $jaeger_bundle, gcroot_path: $jaeger_gcroot},
      grafana: {bundle_path: $grafana_bundle, gcroot_path: $grafana_gcroot}
    }
  }' > "$deploy_dir/files/resident-manifest.json"

if [[ -n "${FISHYSTUFF_DEPLOY_EXPECTED_MANIFEST:-}" ]]; then
  cp "$deploy_dir/files/resident-manifest.json" "$FISHYSTUFF_DEPLOY_EXPECTED_MANIFEST"
fi

bash -lc '
  set -euo pipefail
  source "$1/scripts/recipes/lib/common.sh"
  site_ssh_target="${2:?}"
  telemetry_ssh_target="${3:-}"
  deploy_dir="${4:?}"
  deploy_timeout="${5:?}"
  remote_mgmt_bin="${6:-/usr/local/bin/mgmt}"
  if [[ "$remote_mgmt_bin" != /* ]]; then
    remote_mgmt_bin=/usr/local/bin/mgmt
  fi
  remote_nix_max_jobs="${7:?}"
  deploy_target="${8:?}"
  site_push_count="${9:?}"
  telemetry_push_count="${10:?}"
  shift 10
  site_push_paths=()
  for (( idx = 0; idx < site_push_count; idx++ )); do
    site_push_paths+=("${1:?missing site push path}")
    shift
  done
  telemetry_push_paths=()
  for (( idx = 0; idx < telemetry_push_count; idx++ )); do
    telemetry_push_paths+=("${1:?missing telemetry push path}")
    shift
  done
  tmp_key="$(create_temp_ssh_key_from_env /tmp/fishystuff-mgmt-ssh.XXXXXX)"
  trap '\''rm -f "$tmp_key"'\'' EXIT

  push_paths_to_target() {
    local ssh_target="${1:?missing ssh target}"
    shift
    if (( $# == 0 )); then
      echo "[resident-push] no local paths to push for $ssh_target"
      return 0
    fi
    remote_nix_daemon_path="$(detect_remote_nix_daemon_path "$ssh_target" "$tmp_key")"
    if [[ -z "$remote_nix_daemon_path" ]]; then
      echo "could not detect remote nix-daemon path on $ssh_target" >&2
      exit 1
    fi
    SSH_OPTS="-i $tmp_key -o IdentitiesOnly=yes -o StrictHostKeyChecking=accept-new" \
    NIX_SSH_KEY_PATH="$tmp_key" \
    NIX_REMOTE_PROGRAM_PATH="$remote_nix_daemon_path" \
    FISHYSTUFF_REMOTE_NIX_MAX_JOBS="$remote_nix_max_jobs" \
    bash mgmt/scripts/push-fishystuff-bundles-remote.sh \
        "$ssh_target" \
        "$@"
  }

  push_paths_to_target "$site_ssh_target" "${site_push_paths[@]}"
  if [[ -n "$telemetry_ssh_target" && "$telemetry_ssh_target" != "$site_ssh_target" ]]; then
    push_paths_to_target "$telemetry_ssh_target" "${telemetry_push_paths[@]}"
  elif [[ -n "$telemetry_ssh_target" ]]; then
    push_paths_to_target "$telemetry_ssh_target" "${telemetry_push_paths[@]}"
  fi

  SSH_OPTS="-i $tmp_key -o IdentitiesOnly=yes -o StrictHostKeyChecking=accept-new" \
      bash mgmt/scripts/deploy-fishystuff-resident-remote.sh \
      "$deploy_dir" \
      "$deploy_target" \
      "$deploy_timeout" \
      "$remote_mgmt_bin"
' \
-- "$RECIPE_REPO_ROOT" "$target" "$telemetry_target" "$deploy_dir" "$timeout" "$remote_mgmt_bin" "$remote_nix_max_jobs" "$deploy_target" "${#site_push_paths[@]}" "${#telemetry_push_paths[@]}" "${site_push_paths[@]}" "${telemetry_push_paths[@]}"
