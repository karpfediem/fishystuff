#!/usr/bin/env bash
set -euo pipefail

deploy_bin="${1:-auto}"
environment="${2:-production}"
state_dir="${3:-/var/lib/fishystuff/gitops}"
rollback_set_path="${4:-}"

if [[ -z "$environment" ]]; then
  echo "environment must not be empty" >&2
  exit 2
fi

if [[ -z "$state_dir" ]]; then
  echo "state_dir must not be empty" >&2
  exit 2
fi

if [[ -z "$rollback_set_path" ]]; then
  rollback_set_path="${state_dir%/}/rollback-set/${environment}.json"
fi

if [[ ! -f "$rollback_set_path" ]]; then
  echo "rollback-set document does not exist: $rollback_set_path" >&2
  exit 2
fi

if ! command -v jq >/dev/null 2>&1; then
  echo "jq is required to read rollback-set member paths" >&2
  exit 127
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

member_paths_json="$(
  jq -ce '
    if (.retained_release_document_paths | type) != "array" then
      error("rollback-set retained_release_document_paths must be an array")
    elif (.retained_release_document_paths | length) == 0 then
      error("rollback-set retained_release_document_paths must not be empty")
    else
      [
        .retained_release_document_paths[]
        | if type == "string" and length > 0 then
            .
          else
            error("rollback-set retained_release_document_paths must contain non-empty strings")
          end
      ]
    end
  ' "$rollback_set_path"
)"

mapfile -t member_paths < <(jq -r '.[]' <<< "$member_paths_json")

args=(gitops retained-releases-json)
for member_path in "${member_paths[@]}"; do
  args+=(--rollback-member "$member_path")
done

printf 'running: %q' "$deploy_bin" >&2
printf ' %q' "${args[@]}" >&2
printf '\n' >&2
exec "$deploy_bin" "${args[@]}"
