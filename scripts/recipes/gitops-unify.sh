#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "${SCRIPT_DIR}/lib/common.sh"

mgmt_bin="$(normalize_named_arg mgmt_bin "${1-auto}")"
state_file="$(normalize_named_arg state_file "${2-gitops/fixtures/empty.desired.json}")"
mgmt_flake="${FISHYSTUFF_GITOPS_MGMT_FLAKE:-${RECIPE_REPO_ROOT}#mgmt-gitops}"

if [[ "$state_file" != /* ]]; then
  state_file="${RECIPE_REPO_ROOT}/${state_file}"
fi

if [[ ! -f "$state_file" ]]; then
  echo "gitops desired-state file does not exist: $state_file" >&2
  exit 2
fi

if [[ "$mgmt_bin" == "auto" ]]; then
  mgmt_out="$(nix build "$mgmt_flake" --no-link --print-out-paths)"
  mgmt_bin="${mgmt_out}/bin/mgmt"
fi

if [[ "$mgmt_bin" == */* && ! -x "$mgmt_bin" ]]; then
  echo "mgmt binary is missing or not executable: $mgmt_bin" >&2
  exit 2
fi

cd "$RECIPE_REPO_ROOT/gitops"
export FISHYSTUFF_GITOPS_STATE_FILE="$state_file"

"$mgmt_bin" run --tmp-prefix --no-network --no-pgp lang --only-unify main.mcl
