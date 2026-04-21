#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "${SCRIPT_DIR}/lib/common.sh"

cd "$RECIPE_REPO_ROOT/mgmt"

mgmt_bin="$(normalize_named_arg mgmt_bin "${1-../result/bin/mgmt}")"
"$mgmt_bin" run lang --only-unify main.mcl
