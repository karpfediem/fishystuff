#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "${SCRIPT_DIR}/lib/common.sh"

output="$(normalize_named_arg output "${1-data/gitops/production-activation.draft.desired.json}")"
summary_file="$(normalize_named_arg summary_file "${2-data/gitops/production-current.handoff-summary.json}")"
admission_file="$(normalize_named_arg admission_file "${3-}")"
mgmt_bin="$(normalize_named_arg mgmt_bin "${4-auto}")"
deploy_bin="$(normalize_named_arg deploy_bin "${5-auto}")"

cd "$RECIPE_REPO_ROOT"

require_command() {
  local command_name="$1"
  if ! command -v "$command_name" >/dev/null; then
    echo "missing required command: ${command_name}" >&2
    exit 2
  fi
}

require_positive_int() {
  local name="$1"
  local value="$2"
  if [[ ! "$value" =~ ^[1-9][0-9]*$ ]]; then
    echo "$name must be a positive integer, got: ${value:-<empty>}" >&2
    exit 2
  fi
}

require_command jq
require_command sha256sum

if [[ "$output" == "-" ]]; then
  echo "gitops-production-activation-draft requires a file output, not '-'" >&2
  exit 2
fi
if [[ -z "$admission_file" ]]; then
  admission_file="${FISHYSTUFF_GITOPS_ADMISSION_EVIDENCE_FILE:-}"
fi
if [[ -z "$admission_file" ]]; then
  echo "gitops-production-activation-draft requires admission_file or FISHYSTUFF_GITOPS_ADMISSION_EVIDENCE_FILE" >&2
  exit 2
fi

if [[ "$output" != /* ]]; then
  output="${RECIPE_REPO_ROOT}/${output}"
fi
if [[ "$summary_file" != /* ]]; then
  summary_file="${RECIPE_REPO_ROOT}/${summary_file}"
fi
if [[ "$admission_file" != /* ]]; then
  admission_file="${RECIPE_REPO_ROOT}/${admission_file}"
fi
if [[ ! -f "$admission_file" ]]; then
  echo "admission evidence does not exist: ${admission_file}" >&2
  exit 2
fi

bash scripts/recipes/gitops-check-handoff-summary.sh "$summary_file"

state_file="$(jq -er '.desired_state_path | select(type == "string" and length > 0)' "$summary_file")"
desired_state_sha256="$(jq -er '.desired_state_sha256 | select(type == "string" and test("^[0-9a-f]{64}$"))' "$summary_file")"
read -r handoff_summary_sha256 _ < <(sha256sum "$summary_file")
environment="$(jq -er '.environment.name | select(type == "string" and length > 0)' "$summary_file")"
active_release_id="$(jq -er '.environment.active_release | select(type == "string" and length > 0)' "$summary_file")"
dolt_commit="$(jq -er '.active_release.dolt_commit | select(type == "string" and length > 0)' "$summary_file")"
release_identity="$(
  jq -er \
    --arg release_id "$active_release_id" \
    '(.releases[$release_id] // error("active release is missing")) as $release
    | "release=\($release_id);generation=\($release.generation);git_rev=\($release.git_rev);dolt_commit=\($release.dolt_commit);dolt_repository=\($release.dolt.repository);dolt_branch_context=\($release.dolt.branch_context);dolt_mode=\($release.dolt.mode);api=\($release.closures.api.store_path);site=\($release.closures.site.store_path);cdn_runtime=\($release.closures.cdn_runtime.store_path);dolt_service=\($release.closures.dolt_service.store_path)"' \
    "$state_file"
)"

if ! jq -e \
  --arg environment "$environment" \
  --arg handoff_summary_sha256 "$handoff_summary_sha256" \
  --arg desired_state_sha256 "$desired_state_sha256" \
  --arg release_id "$active_release_id" \
  --arg release_identity "$release_identity" \
  --arg dolt_commit "$dolt_commit" \
  '
    .schema == "fishystuff.gitops.activation-admission.v1"
    and .environment == $environment
    and .handoff_summary_sha256 == $handoff_summary_sha256
    and .desired_state_sha256 == $desired_state_sha256
    and .release_id == $release_id
    and .release_identity == $release_identity
    and .dolt_commit == $dolt_commit
    and (.api_upstream | type == "string" and length > 0)
    and (.api_upstream | test("^[A-Za-z][A-Za-z0-9+.-]*://"))
    and (.api_upstream | test("^[A-Za-z][A-Za-z0-9+.-]*://[^/?#]*@") | not)
    and (.api_meta.url | type == "string" and length > 0)
    and .api_meta.observed_status == 200
    and .api_meta.release_id == $release_id
    and .api_meta.release_identity == $release_identity
    and .api_meta.dolt_commit == $dolt_commit
    and (.db_backed_probe.name | type == "string" and length > 0)
    and .db_backed_probe.passed == true
    and (.site_cdn_probe.name | type == "string" and length > 0)
    and .site_cdn_probe.passed == true
  ' "$admission_file" >/dev/null; then
  echo "admission evidence does not match verified ${environment} handoff" >&2
  exit 2
fi

api_upstream="$(jq -er '.api_upstream' "$admission_file")"
admission_url="$(jq -er '.api_meta.url' "$admission_file")"
timeout_ms="$(jq -er '.api_meta.timeout_ms // 2000' "$admission_file")"
require_positive_int "admission api_meta.timeout_ms" "$timeout_ms"
if (( timeout_ms > 30000 )); then
  echo "admission api_meta.timeout_ms must be <= 30000, got: ${timeout_ms}" >&2
  exit 2
fi
if [[ "$api_upstream" == */ ]]; then
  echo "admission api_upstream must not end with /" >&2
  exit 2
fi
require_loopback_http_url "admission api_upstream" "$api_upstream"
case "$admission_url" in
  "${api_upstream}/"*) ;;
  "${api_upstream}?"*) ;;
  *)
    echo "admission api_meta.url must target api_upstream" >&2
    exit 2
    ;;
esac
if [[ "$admission_url" != */api/v1/meta ]]; then
  echo "admission api_meta.url must target /api/v1/meta" >&2
  exit 2
fi

current_generation="$(jq -er '.generation | select(type == "number")' "$state_file")"
activation_generation="${FISHYSTUFF_GITOPS_ACTIVATION_GENERATION:-$((current_generation + 1))}"
require_positive_int FISHYSTUFF_GITOPS_ACTIVATION_GENERATION "$activation_generation"
if (( activation_generation <= current_generation )); then
  echo "activation generation must be greater than handoff generation ${current_generation}, got: ${activation_generation}" >&2
  exit 2
fi

mkdir -p "$(dirname "$output")"
tmp="$(mktemp "$(dirname "$output")/.${output##*/}.XXXXXX")"
jq -S \
  --argjson activation_generation "$activation_generation" \
  --arg environment "$environment" \
  --arg api_upstream "$api_upstream" \
  --arg admission_url "$admission_url" \
  --argjson timeout_ms "$timeout_ms" \
  --arg transition_reason "verified ${environment} handoff admission" \
  '.mode = "local-apply"
  | .generation = $activation_generation
  | .environments[$environment].serve = true
  | .environments[$environment].api_upstream = $api_upstream
  | .environments[$environment].admission_probe = {
      kind: "api_meta",
      probe_name: "api-meta",
      url: $admission_url,
      expected_status: 200,
      timeout_ms: $timeout_ms
    }
  | .environments[$environment].transition = {
      kind: "activate",
      from_release: "",
      reason: $transition_reason
    }' \
  "$state_file" >"$tmp"
mv "$tmp" "$output"

bash scripts/recipes/gitops-check-activation-draft.sh "$output" "$summary_file" "$admission_file" "$deploy_bin"
bash scripts/recipes/gitops-unify.sh "$mgmt_bin" "$output"

printf 'gitops_activation_draft=%s\n' "$output" >&2
printf 'gitops_activation_environment=%s\n' "$environment" >&2
printf 'gitops_activation_release_id=%s\n' "$active_release_id" >&2
printf 'gitops_activation_dolt_commit=%s\n' "$dolt_commit" >&2
if [[ "$environment" == "production" ]]; then
  printf 'production_activation_draft=%s\n' "$output" >&2
  printf 'production_activation_release_id=%s\n' "$active_release_id" >&2
  printf 'production_activation_dolt_commit=%s\n' "$dolt_commit" >&2
fi
