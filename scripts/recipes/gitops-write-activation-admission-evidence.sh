#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "${SCRIPT_DIR}/lib/common.sh"

output="$(normalize_named_arg output "${1-data/gitops/production-admission.evidence.json}")"
summary_file="$(normalize_named_arg summary_file "${2-data/gitops/production-current.handoff-summary.json}")"
api_upstream="$(normalize_named_arg api_upstream "${3-}")"
api_meta_source="$(normalize_named_arg api_meta_source "${4-}")"
db_probe_file="$(normalize_named_arg db_probe_file "${5-}")"
site_cdn_probe_file="$(normalize_named_arg site_cdn_probe_file "${6-}")"

cd "$RECIPE_REPO_ROOT"

require_command() {
  local command_name="$1"
  if ! command -v "$command_name" >/dev/null; then
    echo "missing required command: ${command_name}" >&2
    exit 2
  fi
}

require_command jq
require_command sha256sum

if [[ "$output" == "-" ]]; then
  echo "gitops-write-activation-admission-evidence requires a file output, not '-'" >&2
  exit 2
fi
if [[ -z "$api_upstream" ]]; then
  echo "gitops-write-activation-admission-evidence requires api_upstream" >&2
  exit 2
fi
if [[ "$api_upstream" == */ ]]; then
  echo "api_upstream must not end with /" >&2
  exit 2
fi
if [[ "$api_upstream" =~ ^[A-Za-z][A-Za-z0-9+.-]*://[^/?#]*@ ]]; then
  echo "api_upstream must not contain embedded credentials" >&2
  exit 2
fi
require_loopback_http_url api_upstream "$api_upstream"
if [[ -z "$api_meta_source" ]]; then
  echo "gitops-write-activation-admission-evidence requires api_meta_source" >&2
  exit 2
fi
if [[ -z "$db_probe_file" || -z "$site_cdn_probe_file" ]]; then
  echo "gitops-write-activation-admission-evidence requires db_probe_file and site_cdn_probe_file" >&2
  exit 2
fi

if [[ "$output" != /* ]]; then
  output="${RECIPE_REPO_ROOT}/${output}"
fi
if [[ "$summary_file" != /* ]]; then
  summary_file="${RECIPE_REPO_ROOT}/${summary_file}"
fi
if [[ "$db_probe_file" != /* ]]; then
  db_probe_file="${RECIPE_REPO_ROOT}/${db_probe_file}"
fi
if [[ "$site_cdn_probe_file" != /* ]]; then
  site_cdn_probe_file="${RECIPE_REPO_ROOT}/${site_cdn_probe_file}"
fi
if [[ ! -f "$db_probe_file" ]]; then
  echo "DB-backed probe evidence does not exist: ${db_probe_file}" >&2
  exit 2
fi
if [[ ! -f "$site_cdn_probe_file" ]]; then
  echo "site/CDN probe evidence does not exist: ${site_cdn_probe_file}" >&2
  exit 2
fi

bash scripts/recipes/gitops-check-handoff-summary.sh "$summary_file"

state_file="$(jq -er '.desired_state_path | select(type == "string" and length > 0)' "$summary_file")"
desired_state_sha256="$(jq -er '.desired_state_sha256 | select(type == "string" and test("^[0-9a-f]{64}$"))' "$summary_file")"
read -r handoff_summary_sha256 _ < <(sha256sum "$summary_file")
active_release_id="$(jq -er '.environment.active_release | select(type == "string" and length > 0)' "$summary_file")"
dolt_commit="$(jq -er '.active_release.dolt_commit | select(type == "string" and length > 0)' "$summary_file")"
release_identity="$(
  jq -er \
    --arg release_id "$active_release_id" \
    '(.releases[$release_id] // error("active release is missing")) as $release
    | "release=\($release_id);generation=\($release.generation);git_rev=\($release.git_rev);dolt_commit=\($release.dolt_commit);dolt_repository=\($release.dolt.repository);dolt_branch_context=\($release.dolt.branch_context);dolt_mode=\($release.dolt.mode);api=\($release.closures.api.store_path);site=\($release.closures.site.store_path);cdn_runtime=\($release.closures.cdn_runtime.store_path);dolt_service=\($release.closures.dolt_service.store_path)"' \
    "$state_file"
)"

api_meta_tmp="$(mktemp)"
if [[ "$api_meta_source" =~ ^https?:// ]]; then
  curl -fsS "$api_meta_source" >"$api_meta_tmp"
  api_meta_url="$api_meta_source"
else
  if [[ "$api_meta_source" != /* ]]; then
    api_meta_source="${RECIPE_REPO_ROOT}/${api_meta_source}"
  fi
  if [[ ! -f "$api_meta_source" ]]; then
    echo "API meta observation does not exist: ${api_meta_source}" >&2
    exit 2
  fi
  cp "$api_meta_source" "$api_meta_tmp"
  api_meta_url="${FISHYSTUFF_GITOPS_API_META_URL:-${api_upstream}/api/v1/meta}"
fi

case "$api_meta_url" in
  "${api_upstream}/"*) ;;
  "${api_upstream}?"*) ;;
  *)
    echo "API meta URL must target api_upstream" >&2
    exit 2
    ;;
esac
if [[ "$api_meta_url" != */api/v1/meta ]]; then
  echo "API meta URL must target /api/v1/meta" >&2
  exit 2
fi

if ! jq -e \
  --arg release_id "$active_release_id" \
  --arg release_identity "$release_identity" \
  --arg dolt_commit "$dolt_commit" \
  '.release_id == $release_id
  and .release_identity == $release_identity
  and .dolt_commit == $dolt_commit' \
  "$api_meta_tmp" >/dev/null; then
  echo "API meta observation does not match verified production handoff" >&2
  exit 2
fi

if ! jq -e '(.name | type == "string" and length > 0) and .passed == true' "$db_probe_file" >/dev/null; then
  echo "DB-backed probe evidence must contain name and passed=true" >&2
  exit 2
fi
if ! jq -e '(.name | type == "string" and length > 0) and .passed == true' "$site_cdn_probe_file" >/dev/null; then
  echo "site/CDN probe evidence must contain name and passed=true" >&2
  exit 2
fi

mkdir -p "$(dirname "$output")"
tmp="$(mktemp "$(dirname "$output")/.${output##*/}.XXXXXX")"
jq -n -S \
  --arg handoff_summary_sha256 "$handoff_summary_sha256" \
  --arg desired_state_sha256 "$desired_state_sha256" \
  --arg release_id "$active_release_id" \
  --arg release_identity "$release_identity" \
  --arg dolt_commit "$dolt_commit" \
  --arg api_upstream "$api_upstream" \
  --arg api_meta_url "$api_meta_url" \
  --slurpfile db_probe "$db_probe_file" \
  --slurpfile site_cdn_probe "$site_cdn_probe_file" \
  '{
    schema: "fishystuff.gitops.activation-admission.v1",
    environment: "production",
    handoff_summary_sha256: $handoff_summary_sha256,
    desired_state_sha256: $desired_state_sha256,
    release_id: $release_id,
    release_identity: $release_identity,
    dolt_commit: $dolt_commit,
    api_upstream: $api_upstream,
    api_meta: {
      url: $api_meta_url,
      observed_status: 200,
      timeout_ms: 2000,
      release_id: $release_id,
      release_identity: $release_identity,
      dolt_commit: $dolt_commit
    },
    db_backed_probe: $db_probe[0],
    site_cdn_probe: $site_cdn_probe[0]
  }' >"$tmp"
mv "$tmp" "$output"
rm -f "$api_meta_tmp"

printf 'production_admission_evidence=%s\n' "$output" >&2
