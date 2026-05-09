#!/usr/bin/env bash
set -euo pipefail

SCRIPT_PATH="$(readlink -f "${BASH_SOURCE[0]}")"
SCRIPT_DIR="$(cd "$(dirname "$SCRIPT_PATH")" && pwd)"
source "${SCRIPT_DIR}/lib/common.sh"

target="$(normalize_named_arg target "${1:-${FISHYSTUFF_BETA_RESIDENT_TARGET:-}}")"
expected_hostname="$(normalize_named_arg expected_hostname "${2:-site-nbg1-beta}")"
edge_bundle="$(normalize_named_arg edge_bundle "${3:-auto}")"
summary_file="$(normalize_named_arg summary_file "${4:-data/gitops/beta-current.handoff-summary.json}")"
push_bin="$(normalize_named_arg push_bin "${5:-scripts/recipes/push-closure.sh}")"
ssh_bin="$(normalize_named_arg ssh_bin "${6:-${FISHYSTUFF_GITOPS_SSH_BIN:-ssh}}")"
scp_bin="$(normalize_named_arg scp_bin "${7:-${FISHYSTUFF_GITOPS_SCP_BIN:-scp}}")"

cd "$RECIPE_REPO_ROOT"

fail() {
  echo "$1" >&2
  exit 2
}

require_env_value() {
  local name="$1"
  local expected="$2"
  local value="${!name-}"

  if [[ "$value" != "$expected" ]]; then
    fail "gitops-beta-remote-start-edge requires ${name}=${expected}"
  fi
}

require_env_nonempty() {
  local name="$1"
  local value="${!name-}"

  if [[ -z "$value" ]]; then
    fail "gitops-beta-remote-start-edge requires ${name}"
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

resolve_push_command() {
  local command_name="$1"

  if [[ "$command_name" == */* && "$command_name" == *.sh && -f "$command_name" ]]; then
    printf 'bash\0%s\0' "$command_name"
    return
  fi
  require_command_or_executable "$command_name" push_bin
  printf '%s\0' "$command_name"
}

require_beta_deploy_profile() {
  local active_profile="${FISHYSTUFF_OPERATOR_SECRETSPEC_PROFILE:-}"

  case "$active_profile" in
    beta-deploy)
      ;;
    production-deploy | prod-deploy | production)
      fail "gitops-beta-remote-start-edge must not run with production SecretSpec profile active: ${active_profile}"
      ;;
    *)
      fail "gitops-beta-remote-start-edge requires FISHYSTUFF_OPERATOR_SECRETSPEC_PROFILE=beta-deploy"
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
    fail "fresh beta edge start currently expects root SSH, got user: ${user}"
  fi
  if [[ ! "$host" =~ ^([0-9]{1,3}\.){3}[0-9]{1,3}$ ]]; then
    fail "target host must be an IPv4 address, got: ${host}"
  fi
  if [[ "$host" == "178.104.230.121" ]]; then
    fail "target points at the previous beta host; use the fresh replacement IP"
  fi
}

summary_value() {
  local query="$1"
  jq -er "$query" "$summary_file"
}

require_summary_equals() {
  local label="$1"
  local query="$2"
  local expected="$3"
  local value=""

  value="$(summary_value "$query")"
  if [[ "$value" != "$expected" ]]; then
    fail "handoff summary ${label} must be ${expected}, got: ${value}"
  fi
}

allow_fixture_paths() {
  [[ "${FISHYSTUFF_GITOPS_BETA_REMOTE_EDGE_ALLOW_FIXTURE_PATHS:-}" == "1" ]]
}

require_store_path() {
  local label="$1"
  local value="$2"

  if allow_fixture_paths && [[ "$value" == /tmp/* ]]; then
    if [[ ! -e "$value" ]]; then
      fail "${label} fixture path does not exist locally: ${value}"
    fi
    return
  fi

  if [[ "$value" != /nix/store/* ]]; then
    fail "${label} must be a /nix/store path, got: ${value}"
  fi
  if [[ ! -e "$value" ]]; then
    fail "${label} does not exist locally: ${value}"
  fi
}

require_store_dir() {
  local label="$1"
  local value="$2"

  if allow_fixture_paths && [[ "$value" == /tmp/* ]]; then
    if [[ ! -d "$value" ]]; then
      fail "${label} fixture directory does not exist locally: ${value}"
    fi
    return
  fi

  if [[ "$value" != /nix/store/* ]]; then
    fail "${label} must be a /nix/store path, got: ${value}"
  fi
  if [[ ! -d "$value" ]]; then
    fail "${label} does not exist locally: ${value}"
  fi
}

kv_value() {
  local key="$1"
  local file="$2"
  awk -F= -v key="$key" '$1 == key { print substr($0, length(key) + 2); exit }' "$file"
}

require_kv_value() {
  local key="$1"
  local file="$2"
  local message="$3"
  local value=""

  value="$(kv_value "$key" "$file")"
  require_value "$value" "$message"
  printf '%s' "$value"
}

generate_placeholder_tls() {
  local cert_path="$1"
  local key_path="$2"
  local log_path="$3"

  if ! openssl req \
    -x509 \
    -newkey rsa:2048 \
    -nodes \
    -keyout "$key_path" \
    -out "$cert_path" \
    -days 7 \
    -subj "/CN=beta.fishystuff.fish" \
    -addext "subjectAltName=DNS:beta.fishystuff.fish,DNS:api.beta.fishystuff.fish,DNS:cdn.beta.fishystuff.fish,DNS:telemetry.beta.fishystuff.fish" \
    >"$log_path" 2>&1; then
    echo "placeholder beta edge TLS generation failed" >&2
    cat "$log_path" >&2 || true
    exit 2
  fi
}

require_env_value FISHYSTUFF_GITOPS_ENABLE_BETA_REMOTE_EDGE_START 1
require_env_value FISHYSTUFF_GITOPS_ENABLE_BETA_REMOTE_EDGE_CLOSURE_COPY 1
require_env_value FISHYSTUFF_GITOPS_ENABLE_BETA_REMOTE_EDGE_SERVED_LINKS 1
require_env_value FISHYSTUFF_GITOPS_ENABLE_BETA_REMOTE_EDGE_PLACEHOLDER_TLS 1
require_env_value FISHYSTUFF_GITOPS_ENABLE_BETA_REMOTE_EDGE_INSTALL 1
require_env_value FISHYSTUFF_GITOPS_ENABLE_BETA_REMOTE_EDGE_RESTART 1
require_env_value FISHYSTUFF_GITOPS_BETA_REMOTE_EDGE_TARGET "$target"
require_env_nonempty FISHYSTUFF_GITOPS_BETA_EDGE_UNIT_SHA256
require_beta_deploy_profile
assert_deployment_configuration_safe beta
assert_beta_infra_cluster_dns_scope_safe
require_safe_target "$target"
require_command_or_executable jq jq
require_command_or_executable awk awk
require_command_or_executable openssl openssl
require_command_or_executable "$ssh_bin" ssh_bin
require_command_or_executable "$scp_bin" scp_bin
push_command=()
while IFS= read -r -d '' part; do
  push_command+=("$part")
done < <(resolve_push_command "$push_bin")

if [[ ! -f "$summary_file" ]]; then
  fail "handoff summary does not exist: ${summary_file}"
fi

require_summary_equals schema '.schema' fishystuff.gitops.current-handoff.v1
require_summary_equals cluster '.cluster' beta
require_summary_equals environment '.environment.name' beta
require_summary_equals mode '.mode' validate
require_summary_equals closure_paths_verified '.checks.closure_paths_verified | tostring' true
require_summary_equals gitops_unify_passed '.checks.gitops_unify_passed | tostring' true
require_summary_equals summary_remote_deploy_performed '.checks.remote_deploy_performed | tostring' false
require_summary_equals summary_infrastructure_mutation_performed '.checks.infrastructure_mutation_performed | tostring' false

release_id="$(summary_value '.active_release.release_id')"
git_rev="$(summary_value '.active_release.git_rev')"
dolt_commit="$(summary_value '.active_release.dolt_commit')"
site_closure="$(summary_value '.active_release.closures.site')"
cdn_runtime_closure="$(summary_value '.active_release.closures.cdn_runtime')"

require_store_path site_closure "$site_closure"
require_store_path cdn_runtime_closure "$cdn_runtime_closure"

tmp_dir="$(mktemp -d)"
tmp_key="$(create_temp_ssh_key_from_env /tmp/fishystuff-beta-edge-start-key.XXXXXX)"
known_hosts="$(mktemp /tmp/fishystuff-beta-edge-start-known-hosts.XXXXXX)"
cleanup() {
  rm -rf "$tmp_dir"
  rm -f "$tmp_key" "$known_hosts"
}
trap cleanup EXIT

edge_output="${tmp_dir}/edge-bundle.out"
if ! bash scripts/recipes/gitops-check-edge-handoff-bundle.sh "$edge_bundle" beta >"$edge_output"; then
  echo "beta edge service bundle check failed" >&2
  cat "$edge_output" >&2 || true
  exit 2
fi

edge_bundle_resolved="$(require_kv_value gitops_edge_handoff_bundle_ok "$edge_output" "edge bundle check did not report a bundle path")"
edge_unit_name="$(require_kv_value gitops_edge_handoff_unit_name "$edge_output" "edge bundle check did not report a unit name")"
edge_unit_source="$(require_kv_value gitops_edge_handoff_systemd_unit_store "$edge_output" "edge bundle check did not report a unit source")"
edge_unit_install_path="/etc/systemd/system/${edge_unit_name}"
edge_site_root="$(require_kv_value gitops_edge_handoff_site_root "$edge_output" "edge bundle check did not report a site root")"
edge_cdn_root="$(require_kv_value gitops_edge_handoff_cdn_root "$edge_output" "edge bundle check did not report a CDN root")"
edge_tls_dir="$(require_kv_value gitops_edge_handoff_tls_dir "$edge_output" "edge bundle check did not report a TLS dir")"
edge_unit_sha256=""
read -r edge_unit_sha256 _ < <(sha256sum "$edge_unit_source")

if [[ "$edge_unit_name" != "fishystuff-beta-edge.service" ]]; then
  fail "edge bundle reported a non-beta unit: ${edge_unit_name}"
fi
if [[ "$edge_site_root" != "/var/lib/fishystuff/gitops-beta/served/beta/site" ]]; then
  fail "edge bundle reported an unexpected beta site root: ${edge_site_root}"
fi
if [[ "$edge_cdn_root" != "/var/lib/fishystuff/gitops-beta/served/beta/cdn" ]]; then
  fail "edge bundle reported an unexpected beta CDN root: ${edge_cdn_root}"
fi
if [[ "$edge_tls_dir" != "/run/fishystuff/beta-edge/tls" ]]; then
  fail "edge bundle reported an unexpected beta TLS dir: ${edge_tls_dir}"
fi
if [[ "$edge_unit_sha256" != "$FISHYSTUFF_GITOPS_BETA_EDGE_UNIT_SHA256" ]]; then
  fail "FISHYSTUFF_GITOPS_BETA_EDGE_UNIT_SHA256 does not match checked beta edge unit"
fi

require_store_dir edge_bundle "$edge_bundle_resolved"
require_store_path edge_unit_source "$edge_unit_source"

placeholder_fullchain="${tmp_dir}/fullchain.pem"
placeholder_privkey="${tmp_dir}/privkey.pem"
generate_placeholder_tls "$placeholder_fullchain" "$placeholder_privkey" "${tmp_dir}/openssl.log"

printf 'gitops_beta_remote_start_edge_checked=true\n'
printf 'deployment=beta\n'
printf 'resident_target=%s\n' "$target"
printf 'handoff_summary=%s\n' "$summary_file"
printf 'release_id=%s\n' "$release_id"
printf 'git_rev=%s\n' "$git_rev"
printf 'dolt_commit=%s\n' "$dolt_commit"
printf 'edge_bundle=%s\n' "$edge_bundle_resolved"
printf 'edge_unit_source=%s\n' "$edge_unit_source"
printf 'edge_unit_sha256=%s\n' "$edge_unit_sha256"
printf 'site_closure=%s\n' "$site_closure"
printf 'cdn_runtime_closure=%s\n' "$cdn_runtime_closure"
printf 'tls_mode=placeholder_self_signed\n'

"${push_command[@]}" "$target" "$edge_bundle_resolved"

ssh_common=(
  -i "$tmp_key"
  -o IdentitiesOnly=yes
  -o StrictHostKeyChecking=accept-new
  -o UserKnownHostsFile="$known_hosts"
)

remote_fullchain="/tmp/fishystuff-beta-edge-fullchain.pem"
remote_privkey="/tmp/fishystuff-beta-edge-privkey.pem"
"$scp_bin" "${ssh_common[@]}" "$placeholder_fullchain" "${target}:${remote_fullchain}"
"$scp_bin" "${ssh_common[@]}" "$placeholder_privkey" "${target}:${remote_privkey}"

"$ssh_bin" "${ssh_common[@]}" "$target" bash -s -- \
  "$expected_hostname" \
  "$edge_bundle_resolved" \
  "$edge_unit_source" \
  "$edge_unit_sha256" \
  "$edge_unit_install_path" \
  "$edge_site_root" \
  "$edge_cdn_root" \
  "$edge_tls_dir" \
  "$site_closure" \
  "$cdn_runtime_closure" \
  "$remote_fullchain" \
  "$remote_privkey" \
  "$release_id" \
  "$dolt_commit" <<'REMOTE'
set -euo pipefail

expected_hostname="$1"
edge_bundle="$2"
edge_unit_source="$3"
edge_unit_sha256="$4"
edge_unit_install_path="$5"
edge_site_root="$6"
edge_cdn_root="$7"
edge_tls_dir="$8"
site_closure="$9"
cdn_runtime_closure="${10}"
remote_fullchain="${11}"
remote_privkey="${12}"
release_id="${13}"
dolt_commit="${14}"

fail() {
  echo "$1" >&2
  exit 2
}

require_path() {
  local label="$1"
  local path="$2"

  if [[ ! -e "$path" ]]; then
    fail "${label} missing: ${path}"
  fi
}

require_file() {
  local label="$1"
  local path="$2"

  if [[ ! -f "$path" ]]; then
    fail "${label} missing: ${path}"
  fi
}

require_store_path() {
  local label="$1"
  local path="$2"

  case "$path" in
    /nix/store/*)
      ;;
    *)
      fail "${label} must be a /nix/store path, got: ${path}"
      ;;
  esac
  require_path "$label" "$path"
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

publish_symlink() {
  local label="$1"
  local target_path="$2"
  local link_path="$3"
  local parent=""

  parent="$(dirname "$link_path")"
  install -d -m 0755 "$parent"
  ln -sfn "$target_path" "${link_path}.next"
  mv -Tf "${link_path}.next" "$link_path"
  printf '%s_symlink=%s->%s\n' "$label" "$link_path" "$target_path"
}

wait_edge_url() {
  local label="$1"
  local host="$2"
  local path="$3"
  local expected="$4"
  local attempts="$5"
  local n=0
  local body=""

  if ! command -v curl >/dev/null 2>&1; then
    fail "curl is required for beta edge readiness"
  fi

  while (( n < attempts )); do
    body="$(curl -kfsS --max-time 3 --resolve "${host}:443:127.0.0.1" "https://${host}${path}" 2>/dev/null || true)"
    if [[ -n "$body" && ( -z "$expected" || "$body" == *"$expected"* ) ]]; then
      printf '%s_ready=true\n' "$label"
      return 0
    fi
    if ! systemctl is-active --quiet fishystuff-beta-edge.service; then
      systemctl status fishystuff-beta-edge.service --no-pager || true
      journalctl -u fishystuff-beta-edge.service --no-pager -n 120 || true
      fail "beta edge unit stopped before ${host}${path} became ready"
    fi
    n="$((n + 1))"
    sleep 1
  done
  journalctl -u fishystuff-beta-edge.service --no-pager -n 120 || true
  fail "beta edge ${host}${path} did not become ready"
}

if [[ "$(hostname)" != "$expected_hostname" ]]; then
  fail "remote hostname mismatch: expected ${expected_hostname}, got $(hostname)"
fi

require_store_path "edge bundle" "$edge_bundle"
require_store_path "edge unit source" "$edge_unit_source"
require_store_path "site closure" "$site_closure"
require_store_path "CDN runtime closure" "$cdn_runtime_closure"
require_file "placeholder TLS fullchain" "$remote_fullchain"
require_file "placeholder TLS private key" "$remote_privkey"
require_sha256 "edge unit source" "$edge_unit_source" "$edge_unit_sha256"

if [[ "$edge_unit_install_path" != "/etc/systemd/system/fishystuff-beta-edge.service" ]]; then
  fail "edge unit install path must be /etc/systemd/system/fishystuff-beta-edge.service, got: ${edge_unit_install_path}"
fi
if [[ "$edge_site_root" != "/var/lib/fishystuff/gitops-beta/served/beta/site" ]]; then
  fail "edge site root must be /var/lib/fishystuff/gitops-beta/served/beta/site, got: ${edge_site_root}"
fi
if [[ "$edge_cdn_root" != "/var/lib/fishystuff/gitops-beta/served/beta/cdn" ]]; then
  fail "edge CDN root must be /var/lib/fishystuff/gitops-beta/served/beta/cdn, got: ${edge_cdn_root}"
fi
if [[ "$edge_tls_dir" != "/run/fishystuff/beta-edge/tls" ]]; then
  fail "edge TLS dir must be /run/fishystuff/beta-edge/tls, got: ${edge_tls_dir}"
fi

publish_symlink beta_site "$site_closure" "$edge_site_root"
publish_symlink beta_cdn "$cdn_runtime_closure" "$edge_cdn_root"
install -d -m 0700 "$edge_tls_dir"
install -m 0644 "$remote_fullchain" "${edge_tls_dir}/fullchain.pem"
install -m 0600 "$remote_privkey" "${edge_tls_dir}/privkey.pem"
rm -f "$remote_fullchain" "$remote_privkey"
install -D -m 0644 "$edge_unit_source" "$edge_unit_install_path"
systemctl daemon-reload
systemctl restart fishystuff-beta-edge.service
systemctl is-active --quiet fishystuff-beta-edge.service
wait_edge_url edge_site beta.fishystuff.fish / '' 120
wait_edge_url edge_api_meta api.beta.fishystuff.fish /api/v1/meta "$release_id" 120
wait_edge_url edge_cdn_runtime cdn.beta.fishystuff.fish /map/runtime-manifest.json fishystuff_ui_bevy 120

api_meta_body="$(curl -kfsS --max-time 3 --resolve api.beta.fishystuff.fish:443:127.0.0.1 https://api.beta.fishystuff.fish/api/v1/meta)"
if [[ "$api_meta_body" != *"$dolt_commit"* ]]; then
  fail "beta edge API meta did not report expected Dolt commit"
fi

printf 'remote_hostname=%s\n' "$(hostname)"
printf 'remote_edge_served_links_ok=true\n'
printf 'remote_edge_placeholder_tls_installed=true\n'
printf 'remote_edge_service_install_ok=fishystuff-beta-edge.service\n'
printf 'remote_edge_service_restart_ok=fishystuff-beta-edge.service\n'
printf 'remote_edge_api_meta_contains_dolt_commit=true\n'
printf 'remote_host_mutation_performed=true\n'
printf 'remote_deploy_performed=false\n'
printf 'infrastructure_mutation_performed=false\n'
printf 'local_host_mutation_performed=false\n'
REMOTE

printf 'gitops_beta_remote_start_edge_ok=true\n'
printf 'remote_store_mutation_performed=true\n'
printf 'remote_host_mutation_performed=true\n'
printf 'remote_deploy_performed=false\n'
printf 'infrastructure_mutation_performed=false\n'
printf 'local_host_mutation_performed=false\n'
