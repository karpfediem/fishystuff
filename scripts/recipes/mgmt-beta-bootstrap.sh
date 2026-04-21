#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "${SCRIPT_DIR}/lib/common.sh"

cd "$RECIPE_REPO_ROOT"

state="$(normalize_named_arg state "${1-absent}")"
converged_timeout="$(normalize_named_arg converged_timeout "${2-30}")"
mgmt_bin="$(normalize_named_arg mgmt_bin "${3-../result/bin/mgmt}")"
client_urls="$(normalize_named_arg client_urls "${4-http://127.0.0.1:3379}")"
server_urls="$(normalize_named_arg server_urls "${5-http://127.0.0.1:3380}")"
prometheus="$(normalize_named_arg prometheus "${6-false}")"
prometheus_listen="$(normalize_named_arg prometheus_listen "${7-127.0.0.1:9233}")"
pprof_path="$(normalize_named_arg pprof_path "${8-}")"

FISHYSTUFF_HETZNER_STATE="$state" \
  secretspec run --profile beta-deploy -- \
  bash -lc '
    set -euo pipefail
    cd mgmt
    cmd=(
      "$1" run
      --client-urls="$2"
      --server-urls="$3"
      --advertise-client-urls="$2"
      --advertise-server-urls="$3"
    )
    case "$5" in
      true|1|yes)
        cmd+=(--prometheus --prometheus-listen "$6")
        ;;
    esac
    cmd+=(lang --tmp-prefix --no-watch --converged-timeout "$4" main.mcl)
    if [[ -n "$7" ]]; then
      export MGMT_PPROF_PATH="$7"
    fi
    "${cmd[@]}"
  ' \
  -- "$mgmt_bin" "$client_urls" "$server_urls" "$converged_timeout" "$prometheus" "$prometheus_listen" "$pprof_path"
