#!/usr/bin/env bash
set -euo pipefail

SCRIPT_PATH="$(readlink -f "${BASH_SOURCE[0]}")"
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "${SCRIPT_DIR}/lib/common.sh"

cd "$RECIPE_REPO_ROOT"

exec_with_secretspec_profile_if_needed "$(operator_secretspec_profile)" bash "$SCRIPT_PATH" "$@"

target=""
deploy_target=""
host="site-nbg1-beta"
telemetry_host="telemetry-nbg1"
prod_host="site-nbg1-prod"
timeout="${FISHYSTUFF_RESIDENT_DEPLOY_TIMEOUT:-120}"
remote_mgmt_bin="/usr/local/bin/mgmt"
api_gcroot="/nix/var/nix/gcroots/mgmt/fishystuff/api-current"
dolt_gcroot="/nix/var/nix/gcroots/mgmt/fishystuff/dolt-current"
mgmt_modules_dir="${FISHYSTUFF_MGMT_MODULES_DIR:-/home/carp/code/mgmt-fishystuff-beta/modules}"
deployment_environment="beta"
dolt_remote_branch=""
tls_enabled="false"
tls_certificate_name=""
tls_acme_email="acme@karpfen.dev"
tls_challenge="http-01"
tls_dns_provider=""
tls_dns_env_json="{}"
tls_dns_env_keys_csv=""
tls_directory_url="https://acme-v02.api.letsencrypt.org/directory"
tls_domains_json=""

raw_args="$*"
IFS=" " read -r -a overrides <<< "$raw_args"
for arg in "${overrides[@]}"; do
  [[ -n "$arg" ]] || continue
  case "$arg" in
    target=*) target="${arg#target=}" ;;
    deploy_target=*) deploy_target="${arg#deploy_target=}" ;;
    host=*) host="${arg#host=}" ;;
    telemetry_host=*) telemetry_host="${arg#telemetry_host=}" ;;
    prod_host=*) prod_host="${arg#prod_host=}" ;;
    timeout=*) timeout="${arg#timeout=}" ;;
    remote_mgmt_bin=*) remote_mgmt_bin="${arg#remote_mgmt_bin=}" ;;
    api_gcroot=*) api_gcroot="${arg#api_gcroot=}" ;;
    dolt_gcroot=*) dolt_gcroot="${arg#dolt_gcroot=}" ;;
    mgmt_modules_dir=*) mgmt_modules_dir="${arg#mgmt_modules_dir=}" ;;
    deployment_environment=*) deployment_environment="${arg#deployment_environment=}" ;;
    dolt_remote_branch=*) dolt_remote_branch="${arg#dolt_remote_branch=}" ;;
    tls_enabled=*) tls_enabled="${arg#tls_enabled=}" ;;
    tls_certificate_name=*) tls_certificate_name="${arg#tls_certificate_name=}" ;;
    tls_acme_email=*) tls_acme_email="${arg#tls_acme_email=}" ;;
    tls_challenge=*) tls_challenge="${arg#tls_challenge=}" ;;
    tls_dns_provider=*) tls_dns_provider="${arg#tls_dns_provider=}" ;;
    tls_dns_env_json=*) tls_dns_env_json="${arg#tls_dns_env_json=}" ;;
    tls_dns_env_keys_csv=*) tls_dns_env_keys_csv="${arg#tls_dns_env_keys_csv=}" ;;
    tls_directory_url=*) tls_directory_url="${arg#tls_directory_url=}" ;;
    tls_domains_json=*) tls_domains_json="${arg#tls_domains_json=}" ;;
    *)
      echo "unknown override for mgmt-resident-push-api-db: $arg" >&2
      exit 2
      ;;
  esac
done

deployment_environment="$(normalize_deployment_environment "$deployment_environment")"
deployment_domain_name="$(deployment_domain "$deployment_environment")"
site_base_url="https://$deployment_domain_name"
api_base_url="https://api.$deployment_domain_name"
cdn_base_url="https://cdn.$deployment_domain_name"
telemetry_base_url="https://telemetry.$deployment_domain_name"
tls_dns_env_json="$(merge_json_env_from_keys "$tls_dns_env_json" "$tls_dns_env_keys_csv")"
if [[ -z "$tls_domains_json" ]]; then
  tls_domains_json="$(
    jq -cn \
      --arg site "${site_base_url#https://}" \
      --arg api "${api_base_url#https://}" \
      --arg cdn "${cdn_base_url#https://}" \
      --arg telemetry "${telemetry_base_url#https://}" \
      '[$site, $api, $cdn, $telemetry]'
  )"
fi

require_value "$target" "missing target=... for mgmt-resident-push-api-db"
deploy_target="${deploy_target:-$target}"

api_bundle="$(nix build .#api-service-bundle --no-link --print-out-paths)"
dolt_bundle="$(nix build .#dolt-service-bundle --no-link --print-out-paths)"
deploy_dir="$(mktemp -d /tmp/fishystuff-resident-beta.XXXXXX)"
trap 'rm -rf "$deploy_dir"' EXIT
cp -a mgmt/resident-beta/. "$deploy_dir/"
mkdir -p "$deploy_dir/files"
copy_resident_common_modules "$deploy_dir" "$mgmt_modules_dir"

jq -n \
  --arg cluster "beta" \
  --arg hostname "$host" \
  --arg telemetry_hostname "$telemetry_host" \
  --arg prod_hostname "$prod_host" \
  --arg site_base_url "$site_base_url" \
  --arg api_base_url "$api_base_url" \
  --arg cdn_base_url "$cdn_base_url" \
  --arg telemetry_base_url "$telemetry_base_url" \
  --arg deployment_environment "$deployment_environment" \
  --arg startup_mode "enabled" \
  --arg dolt_data_dir "/var/lib/fishystuff/dolt" \
  --arg dolt_cfg_dir "/var/lib/fishystuff/dolt/.doltcfg" \
  --arg dolt_database_name "fishystuff" \
  --arg dolt_remote_url "fishystuff/fishystuff" \
  --arg dolt_remote_branch "$dolt_remote_branch" \
  --arg dolt_clone_depth "1" \
  --arg dolt_volume_device "" \
  --arg dolt_volume_fs_type "ext4" \
  --arg dolt_port "3306" \
  --arg site_root_dir "/srv/fishystuff/site" \
  --arg cdn_root_dir "/srv/fishystuff/cdn" \
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
  --arg edge_bundle "" \
  --arg edge_gcroot "/nix/var/nix/gcroots/mgmt/fishystuff/edge-current" \
  --arg loki_bundle "" \
  --arg loki_gcroot "/nix/var/nix/gcroots/mgmt/fishystuff/loki-current" \
  --arg otel_collector_bundle "" \
  --arg otel_collector_gcroot "/nix/var/nix/gcroots/mgmt/fishystuff/otel-collector-current" \
  --arg vector_bundle "" \
  --arg vector_gcroot "/nix/var/nix/gcroots/mgmt/fishystuff/vector-current" \
  --arg prometheus_bundle "" \
  --arg prometheus_gcroot "/nix/var/nix/gcroots/mgmt/fishystuff/prometheus-current" \
  --arg jaeger_bundle "" \
  --arg jaeger_gcroot "/nix/var/nix/gcroots/mgmt/fishystuff/jaeger-current" \
  --arg grafana_bundle "" \
  --arg grafana_gcroot "/nix/var/nix/gcroots/mgmt/fishystuff/grafana-current" \
  '{
    cluster: $cluster,
    hostname: $hostname,
    telemetry_hostname: $telemetry_hostname,
    prod_hostname: $prod_hostname,
    public_urls: {
      site_base_url: $site_base_url,
      api_base_url: $api_base_url,
      cdn_base_url: $cdn_base_url,
      telemetry_base_url: $telemetry_base_url
    },
    deployment_environment: $deployment_environment,
    startup_mode: $startup_mode,
    dolt: {
      data_dir: $dolt_data_dir,
      cfg_dir: $dolt_cfg_dir,
      database_name: $dolt_database_name,
      remote_url: $dolt_remote_url,
      remote_branch: $dolt_remote_branch,
      clone_depth: $dolt_clone_depth,
      volume_device: $dolt_volume_device,
      volume_fs_type: $dolt_volume_fs_type,
      port: $dolt_port
    },
    content_roots: {
      site_root_dir: $site_root_dir,
      cdn_root_dir: $cdn_root_dir
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

bash -lc '
  set -euo pipefail
  source "$1/scripts/recipes/lib/common.sh"
  ssh_target="${2:?}"
  deploy_dir="${3:?}"
  deploy_timeout="${4:?}"
  remote_mgmt_bin="${5:-/usr/local/bin/mgmt}"
  if [[ "$remote_mgmt_bin" != /* ]]; then
    remote_mgmt_bin=/usr/local/bin/mgmt
  fi
  deploy_target="${6:?}"
  shift 6
  tmp_key="$(create_temp_ssh_key_from_env /tmp/fishystuff-mgmt-ssh.XXXXXX)"
  trap '\''rm -f "$tmp_key"'\'' EXIT
  remote_nix_daemon_path="$(detect_remote_nix_daemon_path "$ssh_target" "$tmp_key")"
  if [[ -z "$remote_nix_daemon_path" ]]; then
    echo "could not detect remote nix-daemon path on $ssh_target" >&2
    exit 1
  fi
  SSH_OPTS="-i $tmp_key -o IdentitiesOnly=yes -o StrictHostKeyChecking=accept-new" \
  NIX_SSH_KEY_PATH="$tmp_key" \
  NIX_REMOTE_PROGRAM_PATH="$remote_nix_daemon_path" \
  bash mgmt/scripts/push-fishystuff-bundles-remote.sh \
      "$ssh_target" \
      "$@"
  SSH_OPTS="-i $tmp_key -o IdentitiesOnly=yes -o StrictHostKeyChecking=accept-new" \
  bash mgmt/scripts/deploy-fishystuff-resident-remote.sh \
      "$deploy_dir" \
      "$deploy_target" \
      "$deploy_timeout" \
      "$remote_mgmt_bin"
' \
-- "$RECIPE_REPO_ROOT" "$target" "$deploy_dir" "$timeout" "$remote_mgmt_bin" "$deploy_target" "$api_bundle" "$dolt_bundle"
