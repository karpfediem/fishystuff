#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "${SCRIPT_DIR}/lib/common.sh"

test_name="$(normalize_named_arg test_name "${1-single-host-candidate}")"
system="${FISHYSTUFF_NIX_SYSTEM:-x86_64-linux}"

case "$test_name" in
  empty-unify)
    check_attr="gitops-empty-unify"
    ;;
  single-host-candidate)
    check_attr="gitops-single-host-candidate-vm"
    ;;
  closure-roots)
    check_attr="gitops-closure-roots-vm"
    ;;
  served-candidate)
    check_attr="gitops-served-candidate-vm"
    ;;
  generated-served-candidate)
    check_attr="gitops-generated-served-candidate-vm"
    ;;
  raw-cdn-serve-refusal)
    check_attr="gitops-raw-cdn-serve-refusal"
    ;;
  *)
    echo "unknown gitops VM test: $test_name" >&2
    echo "known tests: empty-unify, single-host-candidate, closure-roots, served-candidate, generated-served-candidate, raw-cdn-serve-refusal" >&2
    exit 2
    ;;
esac

cmd=(nix build ".#checks.${system}.${check_attr}")
printf 'running:'
printf ' %q' "${cmd[@]}"
printf '\n'

cd "$RECIPE_REPO_ROOT"
"${cmd[@]}"
