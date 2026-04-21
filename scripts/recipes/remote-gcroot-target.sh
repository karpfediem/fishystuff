#!/usr/bin/env bash
set -euo pipefail

SCRIPT_PATH="$(readlink -f "${BASH_SOURCE[0]}")"
SCRIPT_DIR="$(cd "$(dirname "$SCRIPT_PATH")" && pwd)"
source "${SCRIPT_DIR}/lib/common.sh"

cd "$RECIPE_REPO_ROOT"

ssh_target="${1-}"
gcroot_path="${2-}"

require_value "$ssh_target" "usage: remote-gcroot-target.sh <ssh-target> <gcroot-path>"
require_value "$gcroot_path" "usage: remote-gcroot-target.sh <ssh-target> <gcroot-path>"

if [[ -z "${HETZNER_SSH_PRIVATE_KEY:-}" ]]; then
  exec secretspec run --profile beta-deploy -- bash "$SCRIPT_PATH" "$@"
fi

tmp_key="$(create_temp_ssh_key_from_env /tmp/fishystuff-remote-gcroot.XXXXXX)"
trap 'rm -f "$tmp_key"' EXIT

quoted_gcroot_path="$(printf '%q' "$gcroot_path")"

ssh \
  -i "$tmp_key" \
  -o IdentitiesOnly=yes \
  -o StrictHostKeyChecking=accept-new \
  "$ssh_target" \
  "set -euo pipefail; if [[ -e ${quoted_gcroot_path} ]]; then readlink -f ${quoted_gcroot_path}; fi"
