#!/usr/bin/env bash
set -euo pipefail

SCRIPT_PATH="$(readlink -f "${BASH_SOURCE[0]}")"
SCRIPT_DIR="$(cd "$(dirname "$SCRIPT_PATH")" && pwd)"
source "${SCRIPT_DIR}/lib/common.sh"

target="$(normalize_named_arg target "${1:-${FISHYSTUFF_BETA_RESIDENT_TARGET:-}}")"
expected_hostname="$(normalize_named_arg expected_hostname "${2:-site-nbg1-beta}")"
summary_file="$(normalize_named_arg summary_file "${3:-data/gitops/beta-current.handoff-summary.json}")"
ssh_bin="$(normalize_named_arg ssh_bin "${4:-${FISHYSTUFF_GITOPS_SSH_BIN:-ssh}}")"

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
    fail "gitops-beta-remote-materialize-dolt-ref requires ${name}=${expected}"
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
      fail "gitops-beta-remote-materialize-dolt-ref must not run with production SecretSpec profile active: ${active_profile}"
      ;;
    *)
      fail "gitops-beta-remote-materialize-dolt-ref requires FISHYSTUFF_OPERATOR_SECRETSPEC_PROFILE=beta-deploy"
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
    fail "fresh beta Dolt ref materialization currently expects root SSH, got user: ${user}"
  fi
  if [[ ! "$host" =~ ^([0-9]{1,3}\.){3}[0-9]{1,3}$ ]]; then
    fail "target host must be an IPv4 address, got: ${host}"
  fi
  if [[ "$host" == "178.104.230.121" ]]; then
    fail "target points at the previous beta host; use the fresh replacement IP"
  fi
}

require_safe_ref_name() {
  local name="$1"
  local value="$2"

  require_value "$value" "$name must not be empty"
  if [[ ! "$value" =~ ^[A-Za-z0-9._/-]+$ ]]; then
    fail "$name contains unsupported characters: $value"
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

require_store_path() {
  local label="$1"
  local value="$2"
  local fixture_override="${FISHYSTUFF_GITOPS_BETA_REMOTE_DOLT_REF_ALLOW_BUNDLE_FIXTURE:-}"

  if [[ "$fixture_override" == "1" && "$value" == /tmp/* ]]; then
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

extract_dolt_bin() {
  local bundle="$1"
  local script="${bundle}/artifacts/exe/main"
  local dolt_bin=""

  if [[ ! -f "$script" ]]; then
    fail "Dolt service bundle executable script is missing: ${script}"
  fi

  dolt_bin="$(
    sed -n 's|^export PATH="\(/nix/store/[^:"]*-dolt-[^:"]*/bin\):.*$|\1/dolt|p' "$script" | head -n 1
  )"
  if [[ -z "$dolt_bin" ]]; then
    fail "could not resolve Dolt binary from beta Dolt service bundle: ${script}"
  fi
  if [[ "$dolt_bin" != /nix/store/*/bin/dolt ]]; then
    fail "resolved Dolt binary is not a Nix store Dolt binary: ${dolt_bin}"
  fi
  if [[ ! -x "$dolt_bin" ]]; then
    fail "resolved Dolt binary is not executable locally: ${dolt_bin}"
  fi
  printf '%s' "$dolt_bin"
}

require_env_value FISHYSTUFF_GITOPS_ENABLE_BETA_REMOTE_DOLT_REF_MATERIALIZE 1
require_env_value FISHYSTUFF_GITOPS_ENABLE_BETA_REMOTE_DOLT_STOP_SERVICES 1
require_env_value FISHYSTUFF_GITOPS_BETA_REMOTE_DOLT_REF_TARGET "$target"
require_beta_deploy_profile
assert_deployment_configuration_safe beta
assert_beta_infra_cluster_dns_scope_safe
require_safe_target "$target"
require_command_or_executable jq jq
require_command_or_executable sed sed
require_command_or_executable "$ssh_bin" ssh_bin

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
dolt_commit="$(summary_value '.active_release.dolt_commit')"
dolt_branch_context="$(summary_value '.active_release.dolt.branch_context')"
dolt_release_ref="$(summary_value '.active_release.dolt.release_ref')"
dolt_bundle="$(summary_value '.active_release.closures.dolt_service')"
dolt_repo_path="${FISHYSTUFF_GITOPS_BETA_REMOTE_DOLT_REPO_PATH:-/var/lib/fishystuff/beta-dolt/fishystuff}"
dolt_bin="$(extract_dolt_bin "$dolt_bundle")"

require_safe_ref_name release_id "$release_id"
require_safe_ref_name dolt_commit "$dolt_commit"
require_safe_ref_name dolt_branch_context "$dolt_branch_context"
require_safe_ref_name dolt_release_ref "$dolt_release_ref"
require_store_path dolt_bundle "$dolt_bundle"

if [[ "$dolt_branch_context" != "beta" ]]; then
  fail "beta Dolt ref materialization requires branch_context=beta, got: ${dolt_branch_context}"
fi
if [[ "$dolt_release_ref" != fishystuff/gitops-beta/* ]]; then
  fail "beta Dolt release ref must live under fishystuff/gitops-beta/, got: ${dolt_release_ref}"
fi

tmp_key="$(create_temp_ssh_key_from_env /tmp/fishystuff-beta-dolt-ref-key.XXXXXX)"
known_hosts="$(mktemp /tmp/fishystuff-beta-dolt-ref-known-hosts.XXXXXX)"
cleanup() {
  rm -f "$tmp_key" "$known_hosts"
}
trap cleanup EXIT

printf 'gitops_beta_remote_materialize_dolt_ref_checked=true\n'
printf 'deployment=beta\n'
printf 'resident_target=%s\n' "$target"
printf 'release_id=%s\n' "$release_id"
printf 'dolt_commit=%s\n' "$dolt_commit"
printf 'dolt_branch_context=%s\n' "$dolt_branch_context"
printf 'dolt_release_ref=%s\n' "$dolt_release_ref"
printf 'dolt_bin=%s\n' "$dolt_bin"
printf 'dolt_repo_path=%s\n' "$dolt_repo_path"

ssh_common=(
  -i "$tmp_key"
  -o IdentitiesOnly=yes
  -o StrictHostKeyChecking=accept-new
  -o UserKnownHostsFile="$known_hosts"
)

"$ssh_bin" "${ssh_common[@]}" "$target" bash -s -- \
  "$expected_hostname" \
  "$dolt_bin" \
  "$dolt_repo_path" \
  "$dolt_branch_context" \
  "$dolt_release_ref" \
  "$dolt_commit" <<'REMOTE'
set -euo pipefail
expected_hostname="$1"
dolt_bin="$2"
repo="$3"
branch_context="$4"
release_ref="$5"
commit="$6"

fail() {
  echo "$1" >&2
  exit 2
}

if [[ "$(hostname)" != "$expected_hostname" ]]; then
  fail "remote hostname mismatch: expected ${expected_hostname}, got $(hostname)"
fi
if [[ "$dolt_bin" != /nix/store/*/bin/dolt ]]; then
  fail "remote Dolt binary must be a /nix/store path, got: ${dolt_bin}"
fi
if [[ ! -x "$dolt_bin" ]]; then
  fail "remote Dolt binary is not executable: ${dolt_bin}"
fi
if [[ ! -d "$repo/.dolt" ]]; then
  fail "remote beta Dolt repo is missing: ${repo}"
fi
if [[ "$branch_context" != "beta" ]]; then
  fail "remote beta Dolt materialization requires branch_context=beta, got: ${branch_context}"
fi
if [[ "$release_ref" != fishystuff/gitops-beta/* ]]; then
  fail "remote beta Dolt release ref must live under fishystuff/gitops-beta/, got: ${release_ref}"
fi

systemctl stop fishystuff-beta-api.service || true
systemctl stop fishystuff-beta-dolt.service || true
runuser -u fishystuff-beta-dolt -- env \
  HOME=/var/lib/fishystuff/beta-dolt/home \
  DOLT_BIN="$dolt_bin" \
  REPO="$repo" \
  BRANCH_CONTEXT="$branch_context" \
  RELEASE_REF="$release_ref" \
  COMMIT="$commit" \
  bash <<'INNER'
set -euo pipefail
cd "$REPO"
"$DOLT_BIN" fetch origin "$BRANCH_CONTEXT"
"$DOLT_BIN" checkout "$BRANCH_CONTEXT"
"$DOLT_BIN" reset --hard "origin/$BRANCH_CONTEXT"
"$DOLT_BIN" log -n 1 "$BRANCH_CONTEXT" --oneline | grep -F "$COMMIT" >/dev/null
"$DOLT_BIN" branch -f "$RELEASE_REF" "$COMMIT"
"$DOLT_BIN" log -n 1 "$RELEASE_REF" --oneline | grep -F "$COMMIT" >/dev/null
printf 'remote_dolt_branch_commit=%s\n' "$COMMIT"
printf 'remote_dolt_release_ref=%s\n' "$RELEASE_REF"
printf 'remote_dolt_release_ref_commit=%s\n' "$COMMIT"
INNER
printf 'remote_hostname=%s\n' "$(hostname)"
printf 'remote_host_mutation_performed=true\n'
printf 'remote_deploy_performed=false\n'
printf 'infrastructure_mutation_performed=false\n'
printf 'local_host_mutation_performed=false\n'
REMOTE

printf 'gitops_beta_remote_materialize_dolt_ref_ok=true\n'
printf 'remote_host_mutation_performed=true\n'
printf 'remote_deploy_performed=false\n'
printf 'infrastructure_mutation_performed=false\n'
printf 'local_host_mutation_performed=false\n'
