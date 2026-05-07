#!/usr/bin/env bash
set -euo pipefail

deploy_bin="${1:-auto}"
state_file="${2:-data/gitops/production-current.desired.json}"
environment="${3:-production}"

if [[ -z "$state_file" ]]; then
  echo "state_file must not be empty" >&2
  exit 2
fi

if [[ -z "$environment" ]]; then
  echo "environment must not be empty" >&2
  exit 2
fi

if [[ ! -f "$state_file" ]]; then
  echo "desired-state file does not exist: $state_file" >&2
  exit 2
fi

if [[ "$deploy_bin" == "auto" ]]; then
  if command -v fishystuff_deploy >/dev/null 2>&1; then
    deploy_bin="$(command -v fishystuff_deploy)"
  elif [[ -x ./result/bin/fishystuff_deploy ]]; then
    deploy_bin="./result/bin/fishystuff_deploy"
  elif [[ -x ./target/debug/fishystuff_deploy ]]; then
    deploy_bin="./target/debug/fishystuff_deploy"
  else
    echo "fishystuff_deploy not found; pass deploy_bin=... or enter a dev shell with fishystuff_deploy on PATH" >&2
    exit 127
  fi
fi

if [[ ! -x "$deploy_bin" ]]; then
  echo "fishystuff_deploy binary is not executable: $deploy_bin" >&2
  exit 127
fi

args=(
  gitops
  check-desired-serving
  --state "$state_file"
  --environment "$environment"
)

printf 'running: %q' "$deploy_bin" >&2
printf ' %q' "${args[@]}" >&2
printf '\n' >&2
exec "$deploy_bin" "${args[@]}"
