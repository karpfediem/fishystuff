#!/usr/bin/env bash
set -euo pipefail

SCRIPT_PATH="$(readlink -f "${BASH_SOURCE[0]}")"
SCRIPT_DIR="$(cd "$(dirname "$SCRIPT_PATH")" && pwd)"
source "${SCRIPT_DIR}/lib/common.sh"

cd "$RECIPE_REPO_ROOT"

build_ssh_options() {
  local result_var="$1"
  local known_hosts_file="$2"
  local -n result="$result_var"
  result=(
    -o BatchMode=yes
    -o IdentitiesOnly=yes
    -o PreferredAuthentications=publickey
    -o PasswordAuthentication=no
    -o KbdInteractiveAuthentication=no
    -o ConnectTimeout=10
    -o StrictHostKeyChecking=accept-new
    -o LogLevel=ERROR
    -o "UserKnownHostsFile=$known_hosts_file"
  )
}

expect_ssh_hostname() {
  local key_path="$1"
  local known_hosts_file="$2"
  local target="$3"
  local expected_hostname="$4"
  local label="$5"
  local output=""
  local -a ssh_opts=()

  require_value "$target" "missing SSH target for $label"
  require_value "$expected_hostname" "missing expected hostname for $label"
  build_ssh_options ssh_opts "$known_hosts_file"

  if ! output="$(ssh -i "$key_path" "${ssh_opts[@]}" "$target" 'hostname -s 2>/dev/null || hostname' 2>&1)"; then
    printf '[deploy-key-boundary] %s failed: expected key access to %s\n%s\n' "$label" "$target" "$output" >&2
    exit 1
  fi
  output="${output//$'\r'/}"
  output="${output//$'\n'/}"
  if [[ "$output" != "$expected_hostname" ]]; then
    printf '[deploy-key-boundary] %s failed: expected hostname %s, got %s\n' "$label" "$expected_hostname" "$output" >&2
    exit 1
  fi
  printf '[deploy-key-boundary] pass: %s can access %s as %s\n' "$label" "$target" "$output"
}

expect_ssh_denied() {
  local key_path="$1"
  local known_hosts_file="$2"
  local target="$3"
  local label="$4"
  local output=""
  local -a ssh_opts=()

  require_value "$target" "missing SSH target for $label"
  build_ssh_options ssh_opts "$known_hosts_file"

  if output="$(ssh -i "$key_path" "${ssh_opts[@]}" "$target" true 2>&1)"; then
    printf '[deploy-key-boundary] %s failed: key unexpectedly accessed %s\n' "$label" "$target" >&2
    exit 1
  fi

  if grep -Eiq 'permission denied|publickey' <<< "$output"; then
    printf '[deploy-key-boundary] pass: %s denied by %s\n' "$label" "$target"
    return
  fi

  printf '[deploy-key-boundary] %s failed ambiguously for %s; expected public-key denial\n%s\n' "$label" "$target" "$output" >&2
  exit 1
}

with_loaded_profile() {
  local profile="$1"
  local beta_target="$2"
  local beta_telemetry_target="$3"
  local production_target="$4"
  local key_path=""
  local known_hosts_file=""

  require_value "${HETZNER_SSH_PRIVATE_KEY:-}" "HETZNER_SSH_PRIVATE_KEY is required in profile $profile"
  require_value "$beta_telemetry_target" "beta telemetry target is required for deploy key boundary check"

  key_path="$(create_temp_ssh_key_from_env /tmp/fishystuff-deploy-key-boundary.XXXXXX)"
  known_hosts_file="$(mktemp /tmp/fishystuff-deploy-key-boundary-known-hosts.XXXXXX)"
  DEPLOY_KEY_BOUNDARY_KEY_PATH="$key_path"
  DEPLOY_KEY_BOUNDARY_KNOWN_HOSTS_FILE="$known_hosts_file"
  trap 'rm -f "${DEPLOY_KEY_BOUNDARY_KEY_PATH:-}" "${DEPLOY_KEY_BOUNDARY_KNOWN_HOSTS_FILE:-}"' EXIT

  case "$profile" in
    beta-deploy)
      expect_ssh_hostname "$key_path" "$known_hosts_file" "$beta_target" "site-nbg1-beta" "beta key"
      expect_ssh_hostname "$key_path" "$known_hosts_file" "$beta_telemetry_target" "telemetry-nbg1" "beta key telemetry"
      expect_ssh_denied "$key_path" "$known_hosts_file" "$production_target" "beta key against production"
      ;;
    production-deploy)
      expect_ssh_hostname "$key_path" "$known_hosts_file" "$production_target" "site-nbg1-prod" "production key"
      expect_ssh_denied "$key_path" "$known_hosts_file" "$beta_target" "production key against beta"
      expect_ssh_denied "$key_path" "$known_hosts_file" "$beta_telemetry_target" "production key against beta telemetry"
      ;;
    *)
      echo "unknown loaded profile for deploy key boundary check: $profile" >&2
      exit 2
      ;;
  esac

  rm -f "$key_path" "$known_hosts_file"
  trap - EXIT
}

if [[ "${1-}" == "__with-profile" ]]; then
  shift
  profile="${1-}"
  beta_target="${2-}"
  beta_telemetry_target="${3-}"
  production_target="${4-}"
  with_loaded_profile "$profile" "$beta_target" "$beta_telemetry_target" "$production_target"
  exit 0
fi

beta_target="$(normalize_named_arg beta_target "${1:-root@beta.fishystuff.fish}")"
production_target="$(normalize_named_arg production_target "${2:-root@fishystuff.fish}")"
beta_telemetry_target="$(normalize_named_arg beta_telemetry_target "${3:-root@telemetry.beta.fishystuff.fish}")"

if [[ "${FISHYSTUFF_DEPLOY_KEY_BOUNDARY_DRY_RUN:-false}" == "true" ]]; then
  printf '[deploy-key-boundary] dry-run targets\n'
  printf '  beta: %s\n' "$beta_target"
  printf '  beta_telemetry: %s\n' "$beta_telemetry_target"
  printf '  production: %s\n' "$production_target"
  exit 0
fi

status=0
if ! secretspec run --profile beta-deploy -- bash "$SCRIPT_PATH" __with-profile beta-deploy "$beta_target" "$beta_telemetry_target" "$production_target"; then
  status=1
fi
if ! secretspec run --profile production-deploy -- bash "$SCRIPT_PATH" __with-profile production-deploy "$beta_target" "$beta_telemetry_target" "$production_target"; then
  status=1
fi

if (( status != 0 )); then
  printf '[deploy-key-boundary] failed\n' >&2
  exit "$status"
fi

printf '[deploy-key-boundary] passed\n'
