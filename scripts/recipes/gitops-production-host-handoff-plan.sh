#!/usr/bin/env bash
set -euo pipefail

SCRIPT_PATH="$(readlink -f "${BASH_SOURCE[0]}")"
SCRIPT_DIR="$(cd "$(dirname "$SCRIPT_PATH")" && pwd)"
source "${SCRIPT_DIR}/lib/common.sh"

draft_file="$(normalize_named_arg draft_file "${1-data/gitops/production-activation.draft.desired.json}")"
summary_file="$(normalize_named_arg summary_file "${2-data/gitops/production-current.handoff-summary.json}")"
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
bash scripts/recipes/gitops-check-production-edge-handoff-bundle.sh "$edge_bundle" >"$edge_output"

edge_bundle_path="$(awk -F= '$1 == "gitops_edge_handoff_bundle_ok" { print $2 }' "$edge_output")"
edge_caddyfile="$(awk -F= '$1 == "gitops_edge_handoff_caddyfile" { print $2 }' "$edge_output")"
edge_executable="$(awk -F= '$1 == "gitops_edge_handoff_executable" { print $2 }' "$edge_output")"
edge_api_upstream="$(awk -F= '$1 == "gitops_edge_handoff_api_upstream" { print $2 }' "$edge_output")"

require_value "$edge_bundle_path" "edge handoff bundle check did not report a bundle path"
require_value "$edge_caddyfile" "edge handoff bundle check did not report a Caddyfile"
require_value "$edge_executable" "edge handoff bundle check did not report an executable"
require_value "$edge_api_upstream" "edge handoff bundle check did not report an API upstream"

bundle_json="${edge_bundle_path}/bundle.json"
if [[ ! -f "$bundle_json" ]]; then
  echo "production GitOps edge handoff bundle does not contain bundle.json: ${bundle_json}" >&2
  exit 2
fi

require_bundle_json() {
  local label="$1"
  local filter="$2"
  if ! jq -e "$filter" "$bundle_json" >/dev/null; then
    echo "production GitOps edge handoff bundle metadata is missing ${label}" >&2
    exit 2
  fi
}

require_bundle_json "fishystuff-edge service ID" '.id == "fishystuff-edge"'
require_bundle_json "GitOps site required path" '.activation.requiredPaths | index("/var/lib/fishystuff/gitops/served/production/site") != null'
require_bundle_json "GitOps CDN required path" '.activation.requiredPaths | index("/var/lib/fishystuff/gitops/served/production/cdn") != null'
require_bundle_json "systemd fishystuff-edge unit" '.backends.systemd.units[]? | select(.name == "fishystuff-edge.service" and .install_path == "/etc/systemd/system/fishystuff-edge.service" and .state == "running")'
require_bundle_json "TLS fullchain runtime overlay" '.runtimeOverlays[]? | select(.targetPath == "/run/fishystuff/edge/tls/fullchain.pem" and .required == true)'
require_bundle_json "TLS private key runtime overlay" '.runtimeOverlays[]? | select(.targetPath == "/run/fishystuff/edge/tls/privkey.pem" and .required == true)'

state_file="$(jq -er '.desired_state_path | select(type == "string" and length > 0)' "$summary_file")"
read -r draft_sha256 _ < <(sha256sum "$draft_file")
read -r summary_sha256 _ < <(sha256sum "$summary_file")
desired_state_sha256="$(jq -er '.desired_state_sha256' "$summary_file")"
generation="$(jq -er '.generation | select(type == "number")' "$draft_file")"
host="$(jq -er '.environments.production.host | select(type == "string" and length > 0)' "$draft_file")"
release_id="$(jq -er '.environments.production.active_release | select(type == "string" and length > 0)' "$draft_file")"
release_identity="$(jq -er '.release_identity | select(type == "string" and length > 0)' "$admission_file")"
api_upstream="$(jq -er '.environments.production.api_upstream | select(type == "string" and length > 0)' "$draft_file")"
api_upstream_authority="$(jq -nr --arg url "$api_upstream" '$url | sub("^[A-Za-z][A-Za-z0-9+.-]*://"; "") | split("/")[0]')"
retained_release_ids="$(jq -cer '.environments.production.retained_releases' "$draft_file")"
api_closure="$(jq -er --arg release_id "$release_id" '.releases[$release_id].closures.api.store_path' "$draft_file")"
site_closure="$(jq -er --arg release_id "$release_id" '.releases[$release_id].closures.site.store_path' "$draft_file")"
cdn_runtime_closure="$(jq -er --arg release_id "$release_id" '.releases[$release_id].closures.cdn_runtime.store_path' "$draft_file")"
dolt_service_closure="$(jq -er --arg release_id "$release_id" '.releases[$release_id].closures.dolt_service.store_path' "$draft_file")"
systemd_unit_install_path="$(jq -er '.backends.systemd.units[] | select(.name == "fishystuff-edge.service") | .install_path' "$bundle_json")"
systemd_unit_source="${edge_bundle_path}/artifacts/systemd/unit"

if [[ "$api_upstream_authority" != "$edge_api_upstream" ]]; then
  echo "activation draft API upstream does not match edge handoff bundle upstream" >&2
  echo "activation draft: ${api_upstream}" >&2
  echo "edge bundle:      ${edge_api_upstream}" >&2
  exit 2
fi
if [[ ! -f "$systemd_unit_source" ]]; then
  echo "production GitOps edge handoff bundle systemd unit artifact is missing: ${systemd_unit_source}" >&2
  exit 2
fi

printf 'gitops_production_host_handoff_plan_ok=%s\n' "$draft_file"
printf 'activation_draft_sha256=%s\n' "$draft_sha256"
printf 'handoff_summary=%s\n' "$summary_file"
printf 'handoff_summary_sha256=%s\n' "$summary_sha256"
printf 'handoff_desired_state=%s\n' "$state_file"
printf 'handoff_desired_state_sha256=%s\n' "$desired_state_sha256"
printf 'environment=production\n'
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
printf 'systemd_unit_source=%s\n' "$systemd_unit_source"
printf 'systemd_unit_install_path=%s\n' "$systemd_unit_install_path"
printf 'served_site_link=/var/lib/fishystuff/gitops/served/production/site\n'
printf 'served_cdn_link=/var/lib/fishystuff/gitops/served/production/cdn\n'
printf 'tls_fullchain=/run/fishystuff/edge/tls/fullchain.pem\n'
printf 'tls_privkey=/run/fishystuff/edge/tls/privkey.pem\n'
printf 'refusal_condition_01=do not run on a host that is not the intended production host\n'
printf 'refusal_condition_02=do not proceed unless activation review, admission evidence, and edge bundle checks pass on the exact files above\n'
printf 'refusal_condition_03=do not proceed unless /api/v1/meta on %s reports the release identity above\n' "$api_upstream"
printf 'refusal_condition_04=do not proceed unless GitOps served symlinks point at the site/CDN closure tuple above after local apply\n'
printf 'refusal_condition_05=do not proceed unless %s and %s exist and are current production certificates\n' "/run/fishystuff/edge/tls/fullchain.pem" "/run/fishystuff/edge/tls/privkey.pem"
printf 'planned_host_step_01=FISHYSTUFF_GITOPS_ENABLE_PRODUCTION_APPLY=1 FISHYSTUFF_GITOPS_ENABLE_LOCAL_APPLY=1 FISHYSTUFF_GITOPS_APPLY_DRAFT_SHA256=%s just gitops-apply-activation-draft draft_file=%s summary_file=%s admission_file=%s\n' "$draft_sha256" "$draft_file" "$summary_file" "$admission_file"
printf 'planned_host_step_02=just gitops-verify-activation-served draft_file=%s summary_file=%s admission_file=%s\n' "$draft_file" "$summary_file" "$admission_file"
printf 'planned_host_step_03=install -D -m 0644 %s %s\n' "$systemd_unit_source" "$systemd_unit_install_path"
printf 'planned_host_step_04=systemctl daemon-reload\n'
printf 'planned_host_step_05=systemctl restart fishystuff-edge.service\n'
printf 'planned_host_step_06=systemctl is-active --quiet fishystuff-edge.service\n'
printf 'planned_host_step_07=inspect public site/API/CDN/telemetry through production host routing before considering this handoff complete\n'
printf 'remote_deploy_performed=false\n'
printf 'infrastructure_mutation_performed=false\n'
