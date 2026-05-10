#!/usr/bin/env bash
set -euo pipefail

SCRIPT_PATH="$(readlink -f "${BASH_SOURCE[0]}")"
SCRIPT_DIR="$(cd "$(dirname "$SCRIPT_PATH")" && pwd)"
source "${SCRIPT_DIR}/lib/common.sh"

state_file_arg="$(normalize_named_arg state_file "${1-data/gitops/beta-tls.staging.desired.json}")"
ca="$(normalize_named_arg ca "${2-staging}")"
mgmt_bin="$(normalize_named_arg mgmt_bin "${3-auto}")"
converged_timeout="$(normalize_named_arg converged_timeout "${4-300}")"

cd "$RECIPE_REPO_ROOT"

require_command() {
  local name="$1"
  if ! command -v "$name" >/dev/null 2>&1; then
    echo "$name is required" >&2
    exit 127
  fi
}

require_positive_int() {
  local name="$1"
  local value="$2"
  if [[ ! "$value" =~ ^[1-9][0-9]*$ ]]; then
    echo "$name must be a positive integer, got: ${value:-<empty>}" >&2
    exit 2
  fi
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
    echo "gitops-beta-reconcile-tls requires ${name}=${expected}" >&2
    exit 2
  fi
}

case "${FISHYSTUFF_OPERATOR_SECRETSPEC_PROFILE:-}" in
  production-deploy | prod-deploy | production)
    echo "gitops-beta-reconcile-tls must not run with a production SecretSpec profile" >&2
    exit 2
    ;;
esac

case "$ca" in
  staging | production) ;;
  *)
    echo "ca must be staging or production, got: $ca" >&2
    exit 2
    ;;
esac

require_command jq
require_positive_int converged_timeout "$converged_timeout"
require_env_value FISHYSTUFF_GITOPS_ENABLE_BETA_TLS_APPLY 1
require_env_value FISHYSTUFF_GITOPS_ENABLE_LOCAL_APPLY 1
if [[ "$ca" == "production" ]]; then
  require_env_value FISHYSTUFF_GITOPS_ENABLE_BETA_TLS_PRODUCTION_ACME 1
fi
if [[ -z "${CLOUDFLARE_API_TOKEN:-}" ]]; then
  echo "gitops-beta-reconcile-tls requires CLOUDFLARE_API_TOKEN from beta-deploy SecretSpec" >&2
  exit 2
fi
deployment_require_current_hostname_match beta gitops-beta-reconcile-tls

state_file="$(absolute_path "$state_file_arg")"
if [[ ! -f "$state_file" ]]; then
  echo "beta TLS desired state file does not exist: ${state_file_arg}" >&2
  exit 2
fi

packet_output="$(mktemp)"
cleanup() {
  rm -f "$packet_output"
}
trap cleanup EXIT

if ! bash scripts/recipes/gitops-beta-tls-reconcile-packet.sh "$state_file_arg" "$ca" >"$packet_output"; then
  cat "$packet_output" >&2 || true
  exit 2
fi
if ! awk -F= '$1 == "beta_tls_packet_status" && $2 == "ready" { found = 1 } END { exit(found ? 0 : 1) }' "$packet_output"; then
  cat "$packet_output" >&2
  echo "beta TLS reconcile packet is not ready" >&2
  exit 2
fi

if [[ "$mgmt_bin" == "auto" ]]; then
  mgmt_flake="${FISHYSTUFF_GITOPS_MGMT_FLAKE:-${RECIPE_REPO_ROOT}#mgmt-gitops}"
  mgmt_out="$(nix build "$mgmt_flake" --no-link --print-out-paths)"
  mgmt_bin="${mgmt_out}/bin/mgmt"
fi

if [[ "$mgmt_bin" == */* && ! -x "$mgmt_bin" ]]; then
  echo "mgmt binary is missing or not executable: $mgmt_bin" >&2
  exit 2
fi

cd "$RECIPE_REPO_ROOT/gitops"
export FISHYSTUFF_GITOPS_STATE_FILE="$state_file"

"$mgmt_bin" run --tmp-prefix --no-pgp lang --no-watch --converged-timeout "$converged_timeout" main.mcl

printf 'gitops_beta_tls_reconcile_ok=%s\n' "$state_file_arg"
printf 'beta_tls_reconcile_ca=%s\n' "$ca"
printf 'beta_tls_reconcile_state_file=%s\n' "$state_file_arg"
printf 'beta_tls_reconcile_tls_dir=/var/lib/fishystuff/gitops-beta/tls/live\n'
printf 'remote_deploy_performed=false\n'
printf 'infrastructure_mutation_performed=false\n'
printf 'local_host_mutation_performed=true\n'
