#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "${SCRIPT_DIR}/lib/common.sh"

cd "$RECIPE_REPO_ROOT"

target="$(normalize_named_arg target "${1-}")"
dir="$(normalize_named_arg dir "${2-mgmt/resident-deploy-probe}")"
timeout="$(normalize_named_arg timeout "${3-120}")"
remote_mgmt_bin="$(normalize_named_arg remote_mgmt_bin "${4-/usr/local/bin/mgmt}")"

require_value "$target" "missing target=... for mgmt-resident-deploy-remote"

secretspec run --profile beta-deploy -- \
  bash -lc '
    set -euo pipefail
    source "$1/scripts/recipes/lib/common.sh"
    tmp_key="$(create_temp_ssh_key_from_env /tmp/fishystuff-mgmt-ssh.XXXXXX)"
    trap '\''rm -f "$tmp_key"'\'' EXIT
    SSH_OPTS="-i $tmp_key -o IdentitiesOnly=yes" \
      bash mgmt/scripts/deploy-fishystuff-resident-remote.sh \
        "$2" \
        "$3" \
        "$4" \
        "$5"
  ' \
  -- "$RECIPE_REPO_ROOT" "$dir" "$target" "$timeout" "$remote_mgmt_bin"
