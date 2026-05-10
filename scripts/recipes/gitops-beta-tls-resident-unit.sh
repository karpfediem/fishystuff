#!/usr/bin/env bash
set -euo pipefail

SCRIPT_PATH="$(readlink -f "${BASH_SOURCE[0]}")"
SCRIPT_DIR="$(cd "$(dirname "$SCRIPT_PATH")" && pwd)"
source "${SCRIPT_DIR}/lib/common.sh"

output="$(normalize_named_arg output "${1--}")"
state_file="$(normalize_named_arg state_file "${2-/var/lib/fishystuff/gitops-beta/desired/beta-tls.desired.json}")"
mgmt_bin="$(normalize_named_arg mgmt_bin "${3-auto}")"
gitops_dir="$(normalize_named_arg gitops_dir "${4-auto}")"
cloudflare_token_credential="$(normalize_named_arg cloudflare_token_credential "${5-/var/lib/fishystuff/gitops-beta/secrets/cloudflare-api-token}")"
converged_timeout="$(normalize_named_arg converged_timeout "${6--1}")"

cd "$RECIPE_REPO_ROOT"

require_command() {
  local name="$1"
  if ! command -v "$name" >/dev/null 2>&1; then
    echo "$name is required" >&2
    exit 127
  fi
}

require_absolute_path() {
  local name="$1"
  local value="$2"
  if [[ "$value" != /* ]]; then
    echo "$name must be an absolute path, got: ${value:-<empty>}" >&2
    exit 2
  fi
}

require_beta_prefix() {
  local name="$1"
  local value="$2"
  local prefix="$3"
  if [[ "$value" != "$prefix"* ]]; then
    echo "$name must stay under ${prefix}, got: ${value}" >&2
    exit 2
  fi
}

require_integer() {
  local name="$1"
  local value="$2"
  if [[ ! "$value" =~ ^-?[0-9]+$ ]]; then
    echo "$name must be an integer, got: ${value:-<empty>}" >&2
    exit 2
  fi
}

resolve_executable() {
  local value="$1"
  local label="$2"
  if [[ "$value" != /* ]]; then
    echo "$label must be an absolute path, got: ${value}" >&2
    exit 2
  fi
  if [[ ! -x "$value" ]]; then
    echo "$label is missing or not executable: ${value}" >&2
    exit 2
  fi
  readlink -f "$value"
}

resolve_directory() {
  local value="$1"
  local label="$2"
  if [[ "$value" != /* ]]; then
    echo "$label must be an absolute path, got: ${value}" >&2
    exit 2
  fi
  if [[ ! -d "$value" ]]; then
    echo "$label does not exist: ${value}" >&2
    exit 2
  fi
  readlink -f "$value"
}

require_absolute_path state_file "$state_file"
require_absolute_path cloudflare_token_credential "$cloudflare_token_credential"
require_beta_prefix state_file "$state_file" "/var/lib/fishystuff/gitops-beta/desired/"
require_beta_prefix cloudflare_token_credential "$cloudflare_token_credential" "/var/lib/fishystuff/gitops-beta/secrets/"
require_integer converged_timeout "$converged_timeout"

if [[ "$mgmt_bin" == "auto" ]]; then
  require_command nix
  mgmt_flake="${FISHYSTUFF_GITOPS_MGMT_FLAKE:-${RECIPE_REPO_ROOT}#mgmt-gitops}"
  mgmt_out="$(nix build "$mgmt_flake" --no-link --print-out-paths)"
  mgmt_bin="${mgmt_out}/bin/mgmt"
fi
mgmt_bin="$(resolve_executable "$mgmt_bin" mgmt_bin)"

if [[ "$gitops_dir" == "auto" ]]; then
  require_command nix
  gitops_flake="${FISHYSTUFF_GITOPS_SOURCE_FLAKE:-${RECIPE_REPO_ROOT}#gitops-src}"
  gitops_dir="$(nix build "$gitops_flake" --no-link --print-out-paths)"
fi
gitops_dir="$(resolve_directory "$gitops_dir" gitops_dir)"
if [[ ! -f "${gitops_dir}/main.mcl" ]]; then
  echo "gitops_dir does not contain main.mcl: ${gitops_dir}" >&2
  exit 2
fi

unit="$(
  cat <<EOF
[Unit]
Description=FishyStuff beta GitOps TLS ACME reconciler
Wants=network-online.target fishystuff-beta-edge.service
After=network-online.target fishystuff-beta-edge.service

[Service]
Type=simple
User=root
Group=root
WorkingDirectory=${gitops_dir}
Environment=FISHYSTUFF_GITOPS_ENABLE_LOCAL_APPLY=1
Environment=FISHYSTUFF_GITOPS_STATE_FILE=${state_file}
Environment=HOME=/var/lib/fishystuff/gitops-beta/mgmt-home
LoadCredential=cloudflare-api-token:${cloudflare_token_credential}
ExecStart=/bin/sh -ceu 'export CLOUDFLARE_API_TOKEN="\$(cat "\$CREDENTIALS_DIRECTORY/cloudflare-api-token")"; exec ${mgmt_bin} run --tmp-prefix --no-pgp lang --converged-timeout ${converged_timeout} main.mcl'
Restart=on-failure
RestartSec=10s
NoNewPrivileges=true
PrivateTmp=true
ProtectSystem=strict
ReadWritePaths=/var/lib/fishystuff/gitops-beta

[Install]
WantedBy=multi-user.target
EOF
)"

if grep -F "fishystuff.fish" <<<"$unit" | grep -v -F "beta.fishystuff" >/dev/null; then
  echo "generated beta TLS resident unit contains a non-beta production hostname" >&2
  exit 2
fi

if [[ "$output" == "-" ]]; then
  printf '%s\n' "$unit"
else
  if [[ "$output" != /* ]]; then
    output="${RECIPE_REPO_ROOT}/${output}"
  fi
  mkdir -p "$(dirname "$output")"
  tmp="$(mktemp "$(dirname "$output")/.${output##*/}.XXXXXX")"
  printf '%s\n' "$unit" >"$tmp"
  mv "$tmp" "$output"
  printf 'wrote %s\n' "$output" >&2
fi

printf 'gitops_beta_tls_resident_unit_ok=true\n' >&2
printf 'beta_tls_resident_unit_name=fishystuff-beta-tls-reconciler.service\n' >&2
printf 'beta_tls_resident_unit_state_file=%s\n' "$state_file" >&2
printf 'beta_tls_resident_unit_gitops_dir=%s\n' "$gitops_dir" >&2
printf 'beta_tls_resident_unit_mgmt_bin=%s\n' "$mgmt_bin" >&2
printf 'beta_tls_resident_unit_cloudflare_token_credential=%s\n' "$cloudflare_token_credential" >&2
printf 'remote_deploy_performed=false\n' >&2
printf 'infrastructure_mutation_performed=false\n' >&2
printf 'local_host_mutation_performed=false\n' >&2
