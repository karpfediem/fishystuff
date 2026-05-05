#!/usr/bin/env bash
set -euo pipefail

deploy_bin="${1:-auto}"
environment="${2:-local-test}"
state_dir="${3:-/var/lib/fishystuff/gitops}"
host="${4:-}"
release_id="${5:-}"
helper_command="${6:-check-served}"

case "$helper_command" in
  check-served | summary-served) ;;
  *)
    echo "unsupported GitOps helper command: $helper_command" >&2
    exit 2
    ;;
esac

if [[ -z "$environment" ]]; then
  echo "environment must not be empty" >&2
  exit 2
fi

if [[ -z "$state_dir" ]]; then
  echo "state_dir must not be empty" >&2
  exit 2
fi

if [[ "$deploy_bin" == "auto" ]]; then
  if command -v fishystuff_deploy >/dev/null 2>&1; then
    deploy_bin="$(command -v fishystuff_deploy)"
  elif [[ -x ./result/bin/fishystuff_deploy ]]; then
    deploy_bin="./result/bin/fishystuff_deploy"
  else
    echo "fishystuff_deploy not found; pass deploy_bin=... or enter a dev shell with fishystuff_deploy on PATH" >&2
    exit 127
  fi
fi

if [[ ! -x "$deploy_bin" ]]; then
  echo "fishystuff_deploy binary is not executable: $deploy_bin" >&2
  exit 127
fi

status_path="${state_dir%/}/status/${environment}.json"
active_path="${state_dir%/}/active/${environment}.json"
rollback_set_path="${state_dir%/}/rollback-set/${environment}.json"

args=(
  gitops
  "$helper_command"
  --status "$status_path"
  --active "$active_path"
  --rollback-set "$rollback_set_path"
  --environment "$environment"
)

if [[ -n "$host" ]]; then
  args+=(--host "$host")
fi

if [[ -n "$release_id" ]]; then
  args+=(--release-id "$release_id")
fi

exec "$deploy_bin" "${args[@]}"
