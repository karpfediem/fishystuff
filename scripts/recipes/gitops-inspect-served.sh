#!/usr/bin/env bash
set -euo pipefail

deploy_bin="${1:-auto}"
environment="${2:-local-test}"
state_dir="${3:-/var/lib/fishystuff/gitops}"
run_dir="${4:-/run/fishystuff/gitops}"
host="${5:-}"
release_id="${6:-}"

if [[ -z "$environment" ]]; then
  echo "environment must not be empty" >&2
  exit 2
fi

if [[ -z "$state_dir" ]]; then
  echo "state_dir must not be empty" >&2
  exit 2
fi

if [[ -z "$run_dir" ]]; then
  echo "run_dir must not be empty" >&2
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
rollback_path="${state_dir%/}/rollback/${environment}.json"
admission_path="${run_dir%/}/admission/${environment}.json"
route_path="${run_dir%/}/routes/${environment}.json"
roots_dir="${run_dir%/}/roots"

args=(
  gitops
  inspect-served
  --status "$status_path"
  --active "$active_path"
  --rollback-set "$rollback_set_path"
  --rollback "$rollback_path"
  --admission "$admission_path"
  --route "$route_path"
  --roots-dir "$roots_dir"
  --environment "$environment"
)

if [[ -n "$host" ]]; then
  args+=(--host "$host")
fi

if [[ -n "$release_id" ]]; then
  args+=(--release-id "$release_id")
fi

printf 'running: %q' "$deploy_bin"
printf ' %q' "${args[@]}"
printf '\n'
exec "$deploy_bin" "${args[@]}"
