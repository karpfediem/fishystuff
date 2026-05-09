#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "${SCRIPT_DIR}/lib/common.sh"

output="$(normalize_named_arg output "${1-data/gitops/beta-current.desired.json}")"
dolt_ref="$(normalize_named_arg dolt_ref "${2-beta}")"

export FISHYSTUFF_GITOPS_CLUSTER="${FISHYSTUFF_GITOPS_CLUSTER:-beta}"
export FISHYSTUFF_GITOPS_ENVIRONMENT="${FISHYSTUFF_GITOPS_ENVIRONMENT:-beta}"
export FISHYSTUFF_GITOPS_HOST_KEY="${FISHYSTUFF_GITOPS_HOST_KEY:-beta-single-host}"
export FISHYSTUFF_GITOPS_HOSTNAME="${FISHYSTUFF_GITOPS_HOSTNAME:-beta-single-host}"
export FISHYSTUFF_GITOPS_DOLT_BRANCH_CONTEXT="${FISHYSTUFF_GITOPS_DOLT_BRANCH_CONTEXT:-beta}"
export FISHYSTUFF_GITOPS_DOLT_CACHE_DIR="${FISHYSTUFF_GITOPS_DOLT_CACHE_DIR:-/var/lib/fishystuff/gitops-beta/dolt-cache/fishystuff}"
export FISHYSTUFF_GITOPS_GCROOT_BASE="${FISHYSTUFF_GITOPS_GCROOT_BASE:-/nix/var/nix/gcroots/fishystuff/gitops-beta}"
export FISHYSTUFF_GITOPS_DOLT_RELEASE_REF_PREFIX="${FISHYSTUFF_GITOPS_DOLT_RELEASE_REF_PREFIX:-fishystuff/gitops-beta}"
export FISHYSTUFF_GITOPS_API_ATTR="${FISHYSTUFF_GITOPS_API_ATTR:-api-service-bundle-beta-gitops-handoff}"
export FISHYSTUFF_GITOPS_SITE_ATTR="${FISHYSTUFF_GITOPS_SITE_ATTR:-site-content-beta}"
export FISHYSTUFF_GITOPS_CDN_RUNTIME_ATTR="${FISHYSTUFF_GITOPS_CDN_RUNTIME_ATTR:-cdn-serving-root}"
export FISHYSTUFF_GITOPS_DOLT_SERVICE_ATTR="${FISHYSTUFF_GITOPS_DOLT_SERVICE_ATTR:-dolt-service-bundle-beta-gitops-handoff}"

if [[ "$FISHYSTUFF_GITOPS_CLUSTER" != "beta" || "$FISHYSTUFF_GITOPS_ENVIRONMENT" != "beta" ]]; then
  echo "gitops-beta-current-desired only writes beta desired state" >&2
  exit 2
fi

if [[ "$FISHYSTUFF_GITOPS_CDN_RUNTIME_ATTR" == "cdn-serving-root" && -z "${FISHYSTUFF_GITOPS_CDN_RUNTIME_CLOSURE:-}" && -z "${FISHYSTUFF_OPERATOR_ROOT:-}" ]]; then
  echo "FISHYSTUFF_OPERATOR_ROOT must be set to build cdn-serving-root, or FISHYSTUFF_GITOPS_CDN_RUNTIME_CLOSURE must provide an exact CDN runtime closure" >&2
  exit 2
fi

bash "${SCRIPT_DIR}/gitops-production-current-desired.sh" "$output" "$dolt_ref"
