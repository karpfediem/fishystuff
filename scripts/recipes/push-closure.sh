#!/usr/bin/env bash
set -euo pipefail

SCRIPT_PATH="$(readlink -f "${BASH_SOURCE[0]}")"
SCRIPT_DIR="$(cd "$(dirname "$SCRIPT_PATH")" && pwd)"
source "${SCRIPT_DIR}/lib/common.sh"

cd "$RECIPE_REPO_ROOT"

if (( $# < 2 )); then
  echo "usage: push-closure.sh <ssh-target> <closure-or-installable> [closure-or-installable ...]" >&2
  exit 2
fi

ssh_target="${1:?missing ssh target}"
shift

exec_with_secretspec_profile_if_needed "$(operator_secretspec_profile)" bash "$SCRIPT_PATH" "$ssh_target" "$@"

tmp_key="$(create_temp_ssh_key_from_env /tmp/fishystuff-push-closure.XXXXXX)"
trap 'rm -f "$tmp_key"' EXIT

remote_nix_daemon_path="$(detect_remote_nix_daemon_path "$ssh_target" "$tmp_key")"
require_value "$remote_nix_daemon_path" "could not detect remote nix-daemon path on $ssh_target"
nix_copy_target="$(build_nix_copy_target "$ssh_target" "$tmp_key" "$remote_nix_daemon_path")"

paths_to_copy=()
for input in "$@"; do
  if [[ -e "$input" ]]; then
    paths_to_copy+=("$(readlink -f "$input")")
    continue
  fi
  while IFS= read -r built_path; do
    [[ -n "$built_path" ]] || continue
    paths_to_copy+=("$built_path")
  done < <(nix build "$input" --no-link --print-out-paths)
done

if (( ${#paths_to_copy[@]} == 0 )); then
  echo "no closure paths resolved for push-closure" >&2
  exit 2
fi

nix copy --no-check-sigs --substitute-on-destination --to "$nix_copy_target" "${paths_to_copy[@]}"
