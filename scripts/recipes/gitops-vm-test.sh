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
  dolt-fetch-pin)
    check_attr="gitops-dolt-fetch-pin-vm"
    ;;
  dolt-admission-pin)
    check_attr="gitops-dolt-admission-pin-vm"
    ;;
  served-retained-dolt-fetch-pin)
    check_attr="gitops-served-retained-dolt-fetch-pin-vm"
    ;;
  multi-environment-candidates)
    check_attr="gitops-multi-environment-candidates-vm"
    ;;
  multi-environment-served)
    check_attr="gitops-multi-environment-served-vm"
    ;;
  closure-roots)
    check_attr="gitops-closure-roots-vm"
    ;;
  unused-release-closure-noop)
    check_attr="gitops-unused-release-closure-noop-vm"
    ;;
  served-closure-roots)
    check_attr="gitops-served-closure-roots-vm"
    ;;
  json-status-escaping)
    check_attr="gitops-json-status-escaping-vm"
    ;;
  served-candidate)
    check_attr="gitops-served-candidate-vm"
    ;;
  generated-served-candidate)
    check_attr="gitops-generated-served-candidate-vm"
    ;;
  served-symlink-transition)
    check_attr="gitops-served-symlink-transition-vm"
    ;;
  served-caddy-handoff)
    check_attr="gitops-served-caddy-handoff-vm"
    ;;
  served-caddy-rollback-transition)
    check_attr="gitops-served-caddy-rollback-transition-vm"
    ;;
  served-rollback-transition)
    check_attr="gitops-served-rollback-transition-vm"
    ;;
  failed-candidate)
    check_attr="gitops-failed-candidate-vm"
    ;;
  failed-served-candidate-refusal)
    check_attr="gitops-failed-served-candidate-refusal"
    ;;
  local-apply-without-optin-refusal)
    check_attr="gitops-local-apply-without-optin-refusal"
    ;;
  missing-active-artifact-refusal)
    check_attr="gitops-missing-active-artifact-refusal"
    ;;
  missing-retained-artifact-refusal)
    check_attr="gitops-missing-retained-artifact-refusal"
    ;;
  missing-retained-release-refusal)
    check_attr="gitops-missing-retained-release-refusal"
    ;;
  no-retained-release-refusal)
    check_attr="gitops-no-retained-release-refusal"
    ;;
  active-retained-release-refusal)
    check_attr="gitops-active-retained-release-refusal"
    ;;
  rollback-transition-retention-refusal)
    check_attr="gitops-rollback-transition-retention-refusal"
    ;;
  raw-cdn-serve-refusal)
    check_attr="gitops-raw-cdn-serve-refusal"
    ;;
  missing-cdn-runtime-file-refusal)
    check_attr="gitops-missing-cdn-runtime-file-refusal"
    ;;
  missing-cdn-serving-manifest-entry-refusal)
    check_attr="gitops-missing-cdn-serving-manifest-entry-refusal"
    ;;
  missing-cdn-retained-root-refusal)
    check_attr="gitops-missing-cdn-retained-root-refusal"
    ;;
  wrong-cdn-retained-root-refusal)
    check_attr="gitops-wrong-cdn-retained-root-refusal"
    ;;
  *)
    echo "unknown gitops VM test: $test_name" >&2
    echo "known tests: empty-unify, single-host-candidate, dolt-fetch-pin, dolt-admission-pin, served-retained-dolt-fetch-pin, multi-environment-candidates, multi-environment-served, closure-roots, unused-release-closure-noop, served-closure-roots, json-status-escaping, served-candidate, generated-served-candidate, served-symlink-transition, served-caddy-handoff, served-caddy-rollback-transition, served-rollback-transition, failed-candidate, failed-served-candidate-refusal, local-apply-without-optin-refusal, missing-active-artifact-refusal, missing-retained-artifact-refusal, missing-retained-release-refusal, no-retained-release-refusal, active-retained-release-refusal, rollback-transition-retention-refusal, raw-cdn-serve-refusal, missing-cdn-runtime-file-refusal, missing-cdn-serving-manifest-entry-refusal, missing-cdn-retained-root-refusal, wrong-cdn-retained-root-refusal" >&2
    exit 2
    ;;
esac

cmd=(nix build ".#checks.${system}.${check_attr}")
printf 'running:'
printf ' %q' "${cmd[@]}"
printf '\n'

cd "$RECIPE_REPO_ROOT"
"${cmd[@]}"
