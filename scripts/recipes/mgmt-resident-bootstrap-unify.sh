#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "${SCRIPT_DIR}/lib/common.sh"

cd "$RECIPE_REPO_ROOT/mgmt/resident-bootstrap"

mgmt_bin="$(normalize_named_arg mgmt_bin "${1-../result/bin/mgmt}")"
module_path="/tmp/fishystuff-mgmt-modules/"
mkdir -p "$module_path"
"$mgmt_bin" run lang --module-path "$module_path" --download --only-unify main.mcl
