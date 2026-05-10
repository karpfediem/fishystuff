#!/usr/bin/env bash
set -euo pipefail

SCRIPT_PATH="$(readlink -f "${BASH_SOURCE[0]}")"
SCRIPT_DIR="$(cd "$(dirname "$SCRIPT_PATH")" && pwd)"
source "${SCRIPT_DIR}/lib/common.sh"

target="$(normalize_named_arg target "${1:-${FISHYSTUFF_BETA_RESIDENT_TARGET:-}}")"
api_source="$(normalize_named_arg api_source "${2:-}")"
dolt_source="$(normalize_named_arg dolt_source "${3:-}")"
ssh_bin="$(normalize_named_arg ssh_bin "${4:-${FISHYSTUFF_GITOPS_SSH_BIN:-ssh}}")"
scp_bin="$(normalize_named_arg scp_bin "${5:-${FISHYSTUFF_GITOPS_SCP_BIN:-scp}}")"
summary_file="$(normalize_named_arg summary_file "${6:-data/gitops/beta-current.handoff-summary.json}")"

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
    fail "gitops-beta-copy-runtime-env requires ${name}=${expected}"
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

require_beta_deploy_profile() {
  local active_profile="${FISHYSTUFF_OPERATOR_SECRETSPEC_PROFILE:-}"

  case "$active_profile" in
    beta-deploy)
      ;;
    production-deploy | prod-deploy | production)
      fail "gitops-beta-copy-runtime-env must not run with production SecretSpec profile active: ${active_profile}"
      ;;
    *)
      fail "gitops-beta-copy-runtime-env requires FISHYSTUFF_OPERATOR_SECRETSPEC_PROFILE=beta-deploy"
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
    fail "fresh beta runtime env copy currently expects root SSH, got user: ${user}"
  fi
  if [[ ! "$host" =~ ^([0-9]{1,3}\.){3}[0-9]{1,3}$ ]]; then
    fail "target host must be an IPv4 address, got: ${host}"
  fi
}

copy_source_or_generate() {
  local service="$1"
  local source="$2"
  local output="$3"

  if [[ -n "$source" ]]; then
    if [[ ! -f "$source" ]]; then
      fail "${service} runtime env source does not exist: ${source}"
    fi
    cp "$source" "$output"
    return
  fi

  case "$service" in
    api)
      FISHYSTUFF_GITOPS_ENABLE_BETA_API_RUNTIME_ENV_WRITE=1 \
        bash scripts/recipes/gitops-beta-write-runtime-env-secretspec.sh api "$output" beta-runtime >/dev/null
      ;;
    dolt)
      FISHYSTUFF_GITOPS_ENABLE_BETA_DOLT_RUNTIME_ENV_WRITE=1 \
        bash scripts/recipes/gitops-beta-write-runtime-env.sh dolt "$output" >/dev/null
      ;;
    *)
      fail "unsupported runtime env service: ${service}"
      ;;
  esac
}

release_identity_from_desired() {
  local desired_file="$1"
  local release_id="$2"

  jq -er \
    --arg release_id "$release_id" \
    '
      .releases[$release_id] as $release
      | "release=\($release_id);generation=\($release.generation);git_rev=\($release.git_rev);dolt_commit=\($release.dolt_commit);dolt_repository=\($release.dolt.repository);dolt_branch_context=\($release.dolt.branch_context);dolt_mode=\($release.dolt.mode);api=\($release.closures.api.store_path);site=\($release.closures.site.store_path);cdn_runtime=\($release.closures.cdn_runtime.store_path);dolt_service=\($release.closures.dolt_service.store_path)"
    ' \
    "$desired_file"
}

write_release_env() {
  local output="$1"
  local desired_file=""
  local release_id=""
  local release_identity=""
  local dolt_commit=""
  local dolt_release_ref=""

  if [[ ! -f "$summary_file" ]]; then
    fail "handoff summary does not exist: ${summary_file}"
  fi
  desired_file="$(jq -er '.desired_state_path | select(type == "string" and length > 0)' "$summary_file")"
  release_id="$(jq -er '.active_release.release_id | select(type == "string" and length > 0)' "$summary_file")"
  dolt_commit="$(jq -er '.active_release.dolt_commit | select(type == "string" and length > 0)' "$summary_file")"
  if [[ ! -f "$desired_file" ]]; then
    fail "desired state from handoff summary does not exist: ${desired_file}"
  fi
  release_identity="$(release_identity_from_desired "$desired_file" "$release_id")"
  dolt_release_ref="$(jq -er --arg release_id "$release_id" '.releases[$release_id].dolt.release_ref | select(type == "string" and length > 0)' "$desired_file")"
  case "$release_id$release_identity$dolt_commit$dolt_release_ref" in
    *"'"* | *$'\n'* | *$'\r'*)
      fail "release env values contain characters that cannot be safely written to a systemd env file"
      ;;
  esac

  cat >"$output" <<EOF
FISHYSTUFF_RELEASE_ID='${release_id}'
FISHYSTUFF_RELEASE_IDENTITY='${release_identity}'
FISHYSTUFF_DOLT_COMMIT='${dolt_commit}'
FISHYSTUFF_DEPLOYMENT_ENVIRONMENT='beta'
FISHYSTUFF_DEFAULT_DOLT_REF='${dolt_release_ref}'
EOF
}

require_env_value FISHYSTUFF_GITOPS_ENABLE_BETA_REMOTE_RUNTIME_ENV_COPY 1
require_env_value FISHYSTUFF_GITOPS_BETA_REMOTE_RUNTIME_ENV_TARGET "$target"
require_beta_deploy_profile
assert_deployment_configuration_safe beta
assert_beta_infra_cluster_dns_scope_safe
require_safe_target "$target"
require_command_or_executable "$ssh_bin" ssh_bin
require_command_or_executable "$scp_bin" scp_bin

tmp_dir="$(mktemp -d)"
tmp_key="$(create_temp_ssh_key_from_env /tmp/fishystuff-beta-runtime-env-key.XXXXXX)"
known_hosts="$(mktemp /tmp/fishystuff-beta-runtime-env-known-hosts.XXXXXX)"
cleanup() {
  rm -rf "$tmp_dir"
  rm -f "$tmp_key" "$known_hosts"
}
trap cleanup EXIT

api_env="${tmp_dir}/api.runtime.env"
dolt_env="${tmp_dir}/dolt.beta.env"
release_env="${tmp_dir}/api.beta.env"
copy_source_or_generate api "$api_source" "$api_env"
copy_source_or_generate dolt "$dolt_source" "$dolt_env"
write_release_env "$release_env"
bash scripts/recipes/gitops-check-beta-runtime-env.sh api "$api_env" >/dev/null
bash scripts/recipes/gitops-check-beta-runtime-env.sh dolt "$dolt_env" >/dev/null
chmod 0640 "$api_env" "$dolt_env" "$release_env"

remote_tmp="/tmp/fishystuff-beta-runtime-env.$$"
ssh_common=(
  -i "$tmp_key"
  -o IdentitiesOnly=yes
  -o StrictHostKeyChecking=accept-new
  -o UserKnownHostsFile="$known_hosts"
)

"$ssh_bin" "${ssh_common[@]}" "$target" "set -eu; test \"\$(hostname)\" = site-nbg1-beta; rm -rf '$remote_tmp'; install -d -m 0700 '$remote_tmp'"
"$scp_bin" "${ssh_common[@]}" "$api_env" "$target:${remote_tmp}/api.runtime.env"
"$scp_bin" "${ssh_common[@]}" "$dolt_env" "$target:${remote_tmp}/dolt.beta.env"
"$scp_bin" "${ssh_common[@]}" "$release_env" "$target:${remote_tmp}/api.beta.env"
"$ssh_bin" "${ssh_common[@]}" "$target" "set -eu; install -m 0640 -o root -g root '$remote_tmp/api.runtime.env' /var/lib/fishystuff/gitops-beta/api/runtime.env; install -m 0640 -o root -g root '$remote_tmp/dolt.beta.env' /var/lib/fishystuff/gitops-beta/dolt/beta.env; install -m 0640 -o root -g root '$remote_tmp/api.beta.env' /var/lib/fishystuff/gitops-beta/api/beta.env; rm -rf '$remote_tmp'"

printf 'gitops_beta_copy_runtime_env_ok=true\n'
printf 'deployment=beta\n'
printf 'resident_target=%s\n' "$target"
printf 'api_runtime_env_path=/var/lib/fishystuff/gitops-beta/api/runtime.env\n'
printf 'api_release_env_path=/var/lib/fishystuff/gitops-beta/api/beta.env\n'
printf 'dolt_runtime_env_path=/var/lib/fishystuff/gitops-beta/dolt/beta.env\n'
printf 'remote_host_mutation_performed=true\n'
printf 'remote_deploy_performed=false\n'
printf 'infrastructure_mutation_performed=false\n'
printf 'local_host_mutation_performed=false\n'
