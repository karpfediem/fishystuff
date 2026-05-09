#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "${SCRIPT_DIR}/lib/common.sh"

output_dir="$(normalize_named_arg output_dir "${1-data/gitops}")"
draft_file="$(normalize_named_arg draft_file "${2-data/gitops/beta-activation.draft.desired.json}")"
summary_file="$(normalize_named_arg summary_file "${3-data/gitops/beta-current.handoff-summary.json}")"
admission_file="$(normalize_named_arg admission_file "${4-}")"
edge_bundle="$(normalize_named_arg edge_bundle "${5-auto}")"
deploy_bin="$(normalize_named_arg deploy_bin "${6-auto}")"
run_helper_tests="$(normalize_named_arg run_helper_tests "${7-true}")"
served_state_dir="$(normalize_named_arg served_state_dir "${8-}")"
rollback_set_path="$(normalize_named_arg rollback_set_path "${9-}")"
state_dir="$(normalize_named_arg state_dir "${10-/var/lib/fishystuff/gitops-beta}")"
run_dir="$(normalize_named_arg run_dir "${11-/run/fishystuff/gitops-beta}")"
systemd_unit_path="$(normalize_named_arg systemd_unit_path "${12-/etc/systemd/system/fishystuff-beta-edge.service}")"
tls_fullchain_path="$(normalize_named_arg tls_fullchain_path "${13-/var/lib/fishystuff/gitops-beta/tls/live/fullchain.pem}")"
tls_privkey_path="$(normalize_named_arg tls_privkey_path "${14-/var/lib/fishystuff/gitops-beta/tls/live/privkey.pem}")"

cd "$RECIPE_REPO_ROOT"

summary_path="$summary_file"
if [[ "$summary_path" != /* ]]; then
  summary_path="${RECIPE_REPO_ROOT}/${summary_path}"
fi
if [[ -f "$summary_path" ]]; then
  if ! command -v jq >/dev/null; then
    echo "missing required command: jq" >&2
    exit 127
  fi
  environment="$(jq -er '.environment.name | select(type == "string" and length > 0)' "$summary_path")"
  if [[ "$environment" != "beta" ]]; then
    echo "gitops-beta-operator-proof requires a beta handoff summary, got: ${environment}" >&2
    exit 2
  fi
fi

bash "${SCRIPT_DIR}/gitops-production-operator-proof.sh" \
  "$output_dir" \
  "$draft_file" \
  "$summary_file" \
  "$admission_file" \
  "$edge_bundle" \
  "$deploy_bin" \
  "$run_helper_tests" \
  "$served_state_dir" \
  "$rollback_set_path" \
  "$state_dir" \
  "$run_dir" \
  "$systemd_unit_path" \
  "$tls_fullchain_path" \
  "$tls_privkey_path" \
  beta
