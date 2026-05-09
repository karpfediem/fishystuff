#!/usr/bin/env bash
set -euo pipefail

SCRIPT_PATH="$(readlink -f "${BASH_SOURCE[0]}")"
SCRIPT_DIR="$(cd "$(dirname "$SCRIPT_PATH")" && pwd)"
source "${SCRIPT_DIR}/lib/common.sh"

environment="$(normalize_named_arg environment "${6-${FISHYSTUFF_GITOPS_ENVIRONMENT:-production}}")"

case "$environment" in
  production)
    default_draft_file="data/gitops/production-activation.draft.desired.json"
    default_summary_file="data/gitops/production-current.handoff-summary.json"
    service_id="fishystuff-edge"
    unit_name="fishystuff-edge.service"
    site_root="/var/lib/fishystuff/gitops/served/production/site"
    cdn_root="/var/lib/fishystuff/gitops/served/production/cdn"
    tls_fullchain_path="/run/fishystuff/edge/tls/fullchain.pem"
    tls_privkey_path="/run/fishystuff/edge/tls/privkey.pem"
    edge_check_recipe="gitops-production-edge-handoff-bundle"
    host_label="production"
    apply_gate_available="true"
    ;;
  beta)
    default_draft_file="data/gitops/beta-activation.draft.desired.json"
    default_summary_file="data/gitops/beta-current.handoff-summary.json"
    service_id="fishystuff-beta-edge"
    unit_name="fishystuff-beta-edge.service"
    site_root="/var/lib/fishystuff/gitops-beta/served/beta/site"
    cdn_root="/var/lib/fishystuff/gitops-beta/served/beta/cdn"
    tls_fullchain_path="/run/fishystuff/beta-edge/tls/fullchain.pem"
    tls_privkey_path="/run/fishystuff/beta-edge/tls/privkey.pem"
    edge_check_recipe="gitops-beta-edge-handoff-bundle"
    host_label="beta"
    apply_gate_available="false"
    ;;
  *)
    echo "unsupported GitOps host handoff environment: ${environment}" >&2
    exit 2
    ;;
esac

draft_file="$(normalize_named_arg draft_file "${1-$default_draft_file}")"
summary_file="$(normalize_named_arg summary_file "${2-$default_summary_file}")"
admission_file="$(normalize_named_arg admission_file "${3-}")"
edge_bundle="$(normalize_named_arg edge_bundle "${4-auto}")"
deploy_bin="$(normalize_named_arg deploy_bin "${5-auto}")"

cd "$RECIPE_REPO_ROOT"

require_command() {
  local command_name="$1"
  if ! command -v "$command_name" >/dev/null 2>&1; then
    echo "missing required command: ${command_name}" >&2
    exit 127
  fi
}

require_command jq
require_command sha256sum

if [[ "$draft_file" != /* ]]; then
  draft_file="${RECIPE_REPO_ROOT}/${draft_file}"
fi
if [[ "$summary_file" != /* ]]; then
  summary_file="${RECIPE_REPO_ROOT}/${summary_file}"
fi
if [[ -z "$admission_file" ]]; then
  admission_file="${FISHYSTUFF_GITOPS_ADMISSION_EVIDENCE_FILE:-}"
fi
if [[ -z "$admission_file" ]]; then
  echo "gitops-production-host-handoff-plan requires admission_file or FISHYSTUFF_GITOPS_ADMISSION_EVIDENCE_FILE" >&2
  exit 2
fi
if [[ "$admission_file" != /* ]]; then
  admission_file="${RECIPE_REPO_ROOT}/${admission_file}"
fi

review_output="$(mktemp)"
edge_output="$(mktemp)"
cleanup() {
  rm -f "$review_output" "$edge_output"
}
trap cleanup EXIT

bash scripts/recipes/gitops-review-activation-draft.sh "$draft_file" "$summary_file" "$admission_file" "$deploy_bin" >"$review_output"
bash scripts/recipes/gitops-check-edge-handoff-bundle.sh "$edge_bundle" "$environment" >"$edge_output"

summary_environment="$(jq -er '.environment.name | select(type == "string" and length > 0)' "$summary_file")"
if [[ "$summary_environment" != "$environment" ]]; then
  echo "host handoff plan environment does not match handoff summary" >&2
  echo "plan:    ${environment}" >&2
  echo "summary: ${summary_environment}" >&2
  exit 2
fi

edge_bundle_path="$(awk -F= '$1 == "gitops_edge_handoff_bundle_ok" { print $2 }' "$edge_output")"
edge_caddyfile="$(awk -F= '$1 == "gitops_edge_handoff_caddyfile" { print $2 }' "$edge_output")"
edge_executable="$(awk -F= '$1 == "gitops_edge_handoff_executable" { print $2 }' "$edge_output")"
edge_api_upstream="$(awk -F= '$1 == "gitops_edge_handoff_api_upstream" { print $2 }' "$edge_output")"
edge_caddy_validate="$(awk -F= '$1 == "gitops_edge_handoff_caddy_validate" { print $2 }' "$edge_output")"
edge_caddyfile_store="$(awk -F= '$1 == "gitops_edge_handoff_caddyfile_store" { print $2 }' "$edge_output")"
edge_executable_store="$(awk -F= '$1 == "gitops_edge_handoff_executable_store" { print $2 }' "$edge_output")"
edge_systemd_unit_store="$(awk -F= '$1 == "gitops_edge_handoff_systemd_unit_store" { print $2 }' "$edge_output")"

require_value "$edge_bundle_path" "edge handoff bundle check did not report a bundle path"
require_value "$edge_caddyfile" "edge handoff bundle check did not report a Caddyfile"
require_value "$edge_executable" "edge handoff bundle check did not report an executable"
require_value "$edge_api_upstream" "edge handoff bundle check did not report an API upstream"
require_value "$edge_caddy_validate" "edge handoff bundle check did not report Caddy validation"
require_value "$edge_caddyfile_store" "edge handoff bundle check did not report a Caddyfile store path"
require_value "$edge_executable_store" "edge handoff bundle check did not report an executable store path"
require_value "$edge_systemd_unit_store" "edge handoff bundle check did not report a systemd unit store path"

if [[ "$edge_caddy_validate" != "true" ]]; then
  echo "edge handoff bundle check did not validate the Caddyfile" >&2
  exit 2
fi

bundle_json="${edge_bundle_path}/bundle.json"
if [[ ! -f "$bundle_json" ]]; then
  echo "${environment} GitOps edge handoff bundle does not contain bundle.json: ${bundle_json}" >&2
  exit 2
fi

require_bundle_json() {
  local label="$1"
  shift
  if ! jq -e "$@" "$bundle_json" >/dev/null; then
    echo "${environment} GitOps edge handoff bundle metadata is missing ${label}" >&2
    exit 2
  fi
}

require_bundle_json "${service_id} service ID" --arg service_id "$service_id" '.id == $service_id'
require_bundle_json "GitOps site required path" --arg site_root "$site_root" '.activation.requiredPaths | index($site_root) != null'
require_bundle_json "GitOps CDN required path" --arg cdn_root "$cdn_root" '.activation.requiredPaths | index($cdn_root) != null'
require_bundle_json "systemd ${unit_name} unit" --arg unit_name "$unit_name" '.backends.systemd.units[]? | select(.name == $unit_name and .install_path == ("/etc/systemd/system/" + $unit_name) and .state == "running")'
require_bundle_json "TLS fullchain runtime overlay" --arg tls_fullchain_path "$tls_fullchain_path" '.runtimeOverlays[]? | select(.targetPath == $tls_fullchain_path and .required == true)'
require_bundle_json "TLS private key runtime overlay" --arg tls_privkey_path "$tls_privkey_path" '.runtimeOverlays[]? | select(.targetPath == $tls_privkey_path and .required == true)'

state_file="$(jq -er '.desired_state_path | select(type == "string" and length > 0)' "$summary_file")"
read -r draft_sha256 _ < <(sha256sum "$draft_file")
read -r summary_sha256 _ < <(sha256sum "$summary_file")
desired_state_sha256="$(jq -er '.desired_state_sha256' "$summary_file")"
generation="$(jq -er '.generation | select(type == "number")' "$draft_file")"
host="$(jq -er --arg environment "$environment" '.environments[$environment].host | select(type == "string" and length > 0)' "$draft_file")"
release_id="$(jq -er --arg environment "$environment" '.environments[$environment].active_release | select(type == "string" and length > 0)' "$draft_file")"
release_identity="$(jq -er '.release_identity | select(type == "string" and length > 0)' "$admission_file")"
api_upstream="$(jq -er --arg environment "$environment" '.environments[$environment].api_upstream | select(type == "string" and length > 0)' "$draft_file")"
api_upstream_authority="$(jq -nr --arg url "$api_upstream" '$url | sub("^[A-Za-z][A-Za-z0-9+.-]*://"; "") | split("/")[0]')"
retained_release_ids="$(jq -cer --arg environment "$environment" '.environments[$environment].retained_releases' "$draft_file")"
api_closure="$(jq -er --arg release_id "$release_id" '.releases[$release_id].closures.api.store_path' "$draft_file")"
site_closure="$(jq -er --arg release_id "$release_id" '.releases[$release_id].closures.site.store_path' "$draft_file")"
cdn_runtime_closure="$(jq -er --arg release_id "$release_id" '.releases[$release_id].closures.cdn_runtime.store_path' "$draft_file")"
dolt_service_closure="$(jq -er --arg release_id "$release_id" '.releases[$release_id].closures.dolt_service.store_path' "$draft_file")"
systemd_unit_install_path="$(jq -er --arg unit_name "$unit_name" '.backends.systemd.units[] | select(.name == $unit_name) | .install_path' "$bundle_json")"
systemd_unit_source="${edge_bundle_path}/artifacts/systemd/unit"

if [[ "$api_upstream_authority" != "$edge_api_upstream" ]]; then
  echo "activation draft API upstream does not match edge handoff bundle upstream" >&2
  echo "activation draft: ${api_upstream}" >&2
  echo "edge bundle:      ${edge_api_upstream}" >&2
  exit 2
fi
if [[ ! -f "$systemd_unit_source" ]]; then
  echo "${environment} GitOps edge handoff bundle systemd unit artifact is missing: ${systemd_unit_source}" >&2
  exit 2
fi

printf 'gitops_host_handoff_plan_ok=%s\n' "$draft_file"
printf 'gitops_host_handoff_environment=%s\n' "$environment"
if [[ "$environment" == "production" ]]; then
  printf 'gitops_production_host_handoff_plan_ok=%s\n' "$draft_file"
elif [[ "$environment" == "beta" ]]; then
  printf 'gitops_beta_host_handoff_plan_ok=%s\n' "$draft_file"
fi
printf 'activation_draft_sha256=%s\n' "$draft_sha256"
printf 'handoff_summary=%s\n' "$summary_file"
printf 'handoff_summary_sha256=%s\n' "$summary_sha256"
printf 'handoff_desired_state=%s\n' "$state_file"
printf 'handoff_desired_state_sha256=%s\n' "$desired_state_sha256"
printf 'environment=%s\n' "$environment"
printf 'desired_generation=%s\n' "$generation"
printf 'host=%s\n' "$host"
printf 'release_id=%s\n' "$release_id"
printf 'release_identity=%s\n' "$release_identity"
printf 'retained_release_ids=%s\n' "$retained_release_ids"
printf 'api_closure=%s\n' "$api_closure"
printf 'site_closure=%s\n' "$site_closure"
printf 'cdn_runtime_closure=%s\n' "$cdn_runtime_closure"
printf 'dolt_service_closure=%s\n' "$dolt_service_closure"
printf 'api_upstream=%s\n' "$api_upstream"
printf 'edge_bundle=%s\n' "$edge_bundle_path"
printf 'edge_caddyfile=%s\n' "$edge_caddyfile"
printf 'edge_executable=%s\n' "$edge_executable"
printf 'edge_caddyfile_store=%s\n' "$edge_caddyfile_store"
printf 'edge_executable_store=%s\n' "$edge_executable_store"
printf 'edge_systemd_unit_store=%s\n' "$edge_systemd_unit_store"
printf 'edge_caddy_validate=%s\n' "$edge_caddy_validate"
printf 'systemd_unit_source=%s\n' "$systemd_unit_source"
printf 'systemd_unit_install_path=%s\n' "$systemd_unit_install_path"
printf 'served_site_link=%s\n' "$site_root"
printf 'served_cdn_link=%s\n' "$cdn_root"
printf 'tls_fullchain=%s\n' "$tls_fullchain_path"
printf 'tls_privkey=%s\n' "$tls_privkey_path"
printf 'apply_gate_available=%s\n' "$apply_gate_available"
if [[ "$environment" == "beta" ]]; then
  printf 'beta_apply_gate_available=%s\n' "$apply_gate_available"
fi
printf 'read_only_readiness_check_01=just gitops-check-handoff-summary summary_file=%s state_file=%s\n' "$summary_file" "$state_file"
printf 'read_only_readiness_check_02=just gitops-check-activation-draft draft_file=%s summary_file=%s admission_file=%s\n' "$draft_file" "$summary_file" "$admission_file"
printf 'read_only_readiness_check_03=just gitops-review-activation-draft draft_file=%s summary_file=%s admission_file=%s\n' "$draft_file" "$summary_file" "$admission_file"
printf 'read_only_readiness_check_04=just %s bundle=%s\n' "$edge_check_recipe" "$edge_bundle_path"
printf 'read_only_readiness_check_05=verify edge_caddy_validate=true and edge bundle store paths above match the exact artifacts to install\n'
printf 'read_only_readiness_check_06=verify /api/v1/meta on %s reports release_identity=%s before local apply\n' "$api_upstream" "$release_identity"
printf 'read_only_readiness_check_07=verify %s and %s exist and are current %s certificates before edge restart\n' "$tls_fullchain_path" "$tls_privkey_path" "$host_label"
printf 'refusal_condition_01=do not run on a host that is not the intended %s host\n' "$host_label"
printf 'refusal_condition_02=do not proceed unless activation review, admission evidence, and edge bundle checks pass on the exact files above\n'
printf 'refusal_condition_03=do not proceed unless /api/v1/meta on %s reports the release identity above\n' "$api_upstream"
printf 'refusal_condition_04=do not proceed unless GitOps served symlinks point at the site/CDN closure tuple above after local apply\n'
printf 'refusal_condition_05=do not proceed unless %s and %s exist and are current %s certificates\n' "$tls_fullchain_path" "$tls_privkey_path" "$host_label"
if [[ "$environment" == "production" ]]; then
  printf 'refusal_condition_06=do not install or restart the edge service unless just gitops-production-proof-index proof_dir=data/gitops require_complete=true passes after served-proof generation\n'
  printf 'guarded_host_action_01=FISHYSTUFF_GITOPS_ENABLE_PRODUCTION_APPLY=1 FISHYSTUFF_GITOPS_ENABLE_LOCAL_APPLY=1 FISHYSTUFF_GITOPS_APPLY_OPERATOR_PROOF_SHA256=<checked operator proof sha256> just gitops-apply-activation-draft draft_file=%s summary_file=%s admission_file=%s proof_file=<checked operator proof file>\n' "$draft_file" "$summary_file" "$admission_file"
else
  printf 'refusal_condition_06=do not install or restart the edge service until a checked beta operator proof and beta apply gate exist\n'
  printf 'guarded_host_action_01=blocked: beta activation apply gate is not implemented yet\n'
fi
printf 'guarded_host_action_02=install -D -m 0644 %s %s\n' "$systemd_unit_source" "$systemd_unit_install_path"
printf 'guarded_host_action_03=systemctl daemon-reload\n'
printf 'guarded_host_action_04=systemctl restart %s\n' "$unit_name"
if [[ "$environment" == "production" ]]; then
  printf 'post_handoff_read_only_check_01=just gitops-verify-activation-served draft_file=%s summary_file=%s admission_file=%s\n' "$draft_file" "$summary_file" "$admission_file"
  printf 'post_handoff_audit_step_01=just gitops-production-served-proof draft_file=%s summary_file=%s admission_file=%s proof_file=<checked operator proof file>\n' "$draft_file" "$summary_file" "$admission_file"
  printf 'post_handoff_read_only_check_02=just gitops-production-proof-index proof_dir=data/gitops require_complete=true\n'
  printf 'planned_host_step_01=FISHYSTUFF_GITOPS_ENABLE_PRODUCTION_APPLY=1 FISHYSTUFF_GITOPS_ENABLE_LOCAL_APPLY=1 FISHYSTUFF_GITOPS_APPLY_OPERATOR_PROOF_SHA256=<checked operator proof sha256> just gitops-apply-activation-draft draft_file=%s summary_file=%s admission_file=%s proof_file=<checked operator proof file>\n' "$draft_file" "$summary_file" "$admission_file"
  printf 'planned_host_step_02=just gitops-verify-activation-served draft_file=%s summary_file=%s admission_file=%s\n' "$draft_file" "$summary_file" "$admission_file"
  printf 'planned_host_step_03=just gitops-production-served-proof draft_file=%s summary_file=%s admission_file=%s proof_file=<checked operator proof file>\n' "$draft_file" "$summary_file" "$admission_file"
  printf 'planned_host_step_04=just gitops-production-proof-index proof_dir=data/gitops require_complete=true\n'
else
  printf 'post_handoff_read_only_check_01=after beta apply support exists, verify beta served state under /var/lib/fishystuff/gitops-beta and /run/fishystuff/gitops-beta\n'
  printf 'post_handoff_audit_step_01=after beta apply support exists, write a beta served proof linked to the checked beta operator proof\n'
  printf 'post_handoff_read_only_check_02=after beta proof indexing exists, require a complete beta proof chain\n'
  printf 'planned_host_step_01=implement checked beta operator proof and beta local apply gate before consuming this draft\n'
  printf 'planned_host_step_02=after checked beta apply, verify beta served state under /var/lib/fishystuff/gitops-beta and /run/fishystuff/gitops-beta\n'
  printf 'planned_host_step_03=after beta served proof support exists, write beta served proof and require a complete beta proof chain\n'
  printf 'planned_host_step_04=only then install or restart the beta edge service\n'
fi
printf 'post_handoff_read_only_check_03=systemctl is-active --quiet %s\n' "$unit_name"
printf 'post_handoff_read_only_check_04=inspect public site/API/CDN/telemetry through %s host routing before considering this handoff complete\n' "$host_label"
printf 'planned_host_step_05=install -D -m 0644 %s %s\n' "$systemd_unit_source" "$systemd_unit_install_path"
printf 'planned_host_step_06=systemctl daemon-reload\n'
printf 'planned_host_step_07=systemctl restart %s\n' "$unit_name"
printf 'planned_host_step_08=systemctl is-active --quiet %s\n' "$unit_name"
printf 'planned_host_step_09=inspect public site/API/CDN/telemetry through %s host routing before considering this handoff complete\n' "$host_label"
printf 'remote_deploy_performed=false\n'
printf 'infrastructure_mutation_performed=false\n'
