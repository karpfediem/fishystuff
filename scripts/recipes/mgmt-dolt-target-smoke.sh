#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "${SCRIPT_DIR}/lib/common.sh"

cd "$RECIPE_REPO_ROOT"

target="$(normalize_named_arg target "${1-}")"
gcroot="$(normalize_named_arg gcroot "${2-/nix/var/nix/gcroots/mgmt/fishystuff/dolt-current}")"
sql_host="$(normalize_named_arg sql_host "${3-127.0.0.1}")"
sql_port="$(normalize_named_arg sql_port "${4-3306}")"
query_timeout="$(normalize_named_arg query_timeout "${5-20}")"

require_value "$target" "missing target=... for mgmt-dolt-target-smoke"

bundle="$(nix build .#dolt-service-bundle --no-link --print-out-paths)"

secretspec run --profile beta-deploy -- \
  bash -lc '
    set -euo pipefail
    source "$1/scripts/recipes/lib/common.sh"
    tmp_key="$(create_temp_ssh_key_from_env /tmp/fishystuff-dolt-smoke-ssh.XXXXXX)"
    trap '\''rm -f "$tmp_key"'\'' EXIT
    remote_nix_daemon_path="$(detect_remote_nix_daemon_path "$3" "$tmp_key")"
    if [[ -z "$remote_nix_daemon_path" ]]; then
      echo "could not detect remote nix-daemon path on $3" >&2
      exit 1
    fi
    SSH_OPTS="-i $tmp_key -o IdentitiesOnly=yes -o StrictHostKeyChecking=accept-new" \
    NIX_SSH_KEY_PATH="$tmp_key" \
    NIX_REMOTE_PROGRAM_PATH="$remote_nix_daemon_path" \
    bash mgmt/scripts/smoke-fishystuff-dolt-target.sh \
      "$2" \
      "$3" \
      "$4" \
      "$5" \
      "$6" \
      "$7"
  ' \
  -- "$RECIPE_REPO_ROOT" "$bundle" "$target" "$gcroot" "$sql_host" "$sql_port" "$query_timeout"
