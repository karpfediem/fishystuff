#!/usr/bin/env bash
set -euo pipefail

SCRIPT_PATH="$(readlink -f "${BASH_SOURCE[0]}")"
SCRIPT_DIR="$(cd "$(dirname "$SCRIPT_PATH")" && pwd)"
source "${SCRIPT_DIR}/lib/common.sh"

output="$(normalize_named_arg output "${1-data/gitops/beta-admission.evidence.json}")"
summary_file="$(normalize_named_arg summary_file "${2-data/gitops/beta-current.handoff-summary.json}")"
api_upstream="$(normalize_named_arg api_upstream "${3-http://127.0.0.1:18192}")"
observation_dir="$(normalize_named_arg observation_dir "${4-data/gitops/beta-admission-observations}")"
curl_bin="$(normalize_named_arg curl_bin "${5-${FISHYSTUFF_GITOPS_CURL_BIN:-curl}}")"
expected_cdn_base_url="$(normalize_named_arg expected_cdn_base_url "${6-${FISHYSTUFF_PUBLIC_CDN_BASE_URL:-https://cdn.beta.fishystuff.fish/}}")"

cd "$RECIPE_REPO_ROOT"

require_command() {
  local command_name="$1"
  if ! command -v "$command_name" >/dev/null 2>&1; then
    echo "missing required command: ${command_name}" >&2
    exit 127
  fi
}

require_executable_or_command() {
  local command_name="$1"
  local label="$2"

  if [[ "$command_name" == */* ]]; then
    if [[ ! -x "$command_name" ]]; then
      echo "${label} is not executable: ${command_name}" >&2
      exit 127
    fi
    return
  fi
  require_command "$command_name"
}

absolute_path() {
  local path="$1"
  if [[ "$path" == /* ]]; then
    printf '%s' "$path"
    return
  fi
  printf '%s/%s' "$RECIPE_REPO_ROOT" "$path"
}

curl_json() {
  local url="$1"
  local target="$2"

  if ! "$curl_bin" -fsS "$url" >"$target"; then
    echo "failed to fetch beta admission probe URL: ${url}" >&2
    exit 2
  fi
  if ! jq -e 'type == "object"' "$target" >/dev/null; then
    echo "beta admission probe did not return a JSON object: ${url}" >&2
    exit 2
  fi
}

resolve_manifest_asset() {
  local cdn_root="$1"
  local manifest_path="$2"
  local asset_path="$3"

  if [[ -z "$asset_path" ]]; then
    echo "runtime manifest asset path must not be empty" >&2
    exit 2
  fi
  case "$asset_path" in
    http://* | https://*)
      echo "runtime manifest asset path must be local to the CDN runtime: ${asset_path}" >&2
      exit 2
      ;;
    /*)
      printf '%s/%s' "${cdn_root%/}" "${asset_path#/}"
      ;;
    *)
      printf '%s/%s' "$(dirname "$manifest_path")" "${asset_path#./}"
      ;;
  esac
}

require_command jq
require_command sha256sum
require_executable_or_command "$curl_bin" curl_bin

if [[ "$api_upstream" == */ ]]; then
  echo "api_upstream must not end with /" >&2
  exit 2
fi
if [[ "$api_upstream" =~ ^[A-Za-z][A-Za-z0-9+.-]*://[^/?#]*@ ]]; then
  echo "api_upstream must not contain embedded credentials" >&2
  exit 2
fi
require_loopback_http_url api_upstream "$api_upstream"

output="$(absolute_path "$output")"
summary_file="$(absolute_path "$summary_file")"
observation_dir="$(absolute_path "$observation_dir")"
mkdir -p "$observation_dir"

bash scripts/recipes/gitops-check-handoff-summary.sh "$summary_file" >/dev/null
environment="$(jq -er '.environment.name | select(type == "string" and length > 0)' "$summary_file")"
if [[ "$environment" != "beta" ]]; then
  echo "gitops-beta-observe-admission requires a beta handoff summary, got: ${environment}" >&2
  exit 2
fi

site_root="$(jq -er '.active_release.closures.site | select(type == "string" and length > 0)' "$summary_file")"
cdn_runtime_root="$(jq -er '.active_release.closures.cdn_runtime | select(type == "string" and length > 0)' "$summary_file")"
if [[ ! -d "$site_root" ]]; then
  echo "beta active site closure is not a directory: ${site_root}" >&2
  exit 2
fi
if [[ ! -d "$cdn_runtime_root" ]]; then
  echo "beta active CDN runtime closure is not a directory: ${cdn_runtime_root}" >&2
  exit 2
fi

api_meta_file="${observation_dir}/api-meta.json"
db_response_file="${observation_dir}/api-fish-list.response.json"
db_probe_file="${observation_dir}/db-backed-probe.json"
site_cdn_probe_file="${observation_dir}/site-cdn-probe.json"
api_meta_url="${api_upstream}/api/v1/meta"
db_probe_url="${api_upstream}/api/v1/fish?lang=en"

curl_json "$api_meta_url" "$api_meta_file"
curl_json "$db_probe_url" "$db_response_file"

if ! jq -e '.fish | type == "array" and length > 0' "$db_response_file" >/dev/null; then
  echo "DB-backed fish probe must return a non-empty fish array" >&2
  exit 2
fi
fish_count="$(jq -er '.count | select(type == "number" and . > 0)' "$db_response_file")"
fish_revision="$(jq -er '.revision // "" | tostring' "$db_response_file")"

jq -n -S \
  --arg name "beta-api-fish-list-en" \
  --arg url "$db_probe_url" \
  --arg revision "$fish_revision" \
  --argjson count "$fish_count" \
  '{
    name: $name,
    passed: true,
    url: $url,
    route: "/api/v1/fish?lang=en",
    expected_status: 200,
    observed_status: 200,
    response: {
      count: $count,
      revision: $revision
    }
  }' >"$db_probe_file"

runtime_config="${site_root%/}/runtime-config.js"
runtime_manifest="${cdn_runtime_root%/}/map/runtime-manifest.json"
if [[ ! -f "$runtime_config" ]]; then
  echo "beta active site closure does not contain runtime-config.js: ${runtime_config}" >&2
  exit 2
fi
if [[ ! -f "$runtime_manifest" ]]; then
  echo "beta active CDN runtime does not contain map runtime manifest: ${runtime_manifest}" >&2
  exit 2
fi
if ! grep -F "$expected_cdn_base_url" "$runtime_config" >/dev/null; then
  echo "beta site runtime config does not reference expected CDN base URL: ${expected_cdn_base_url}" >&2
  exit 2
fi

runtime_module="$(jq -er '.module | select(type == "string" and length > 0)' "$runtime_manifest")"
runtime_wasm="$(jq -er '.wasm | select(type == "string" and length > 0)' "$runtime_manifest")"
runtime_module_path="$(resolve_manifest_asset "$cdn_runtime_root" "$runtime_manifest" "$runtime_module")"
runtime_wasm_path="$(resolve_manifest_asset "$cdn_runtime_root" "$runtime_manifest" "$runtime_wasm")"
if [[ ! -f "$runtime_module_path" ]]; then
  echo "beta CDN runtime module referenced by manifest does not exist: ${runtime_module_path}" >&2
  exit 2
fi
if [[ ! -f "$runtime_wasm_path" ]]; then
  echo "beta CDN runtime wasm referenced by manifest does not exist: ${runtime_wasm_path}" >&2
  exit 2
fi

jq -n -S \
  --arg name "beta-site-cdn-runtime-manifest" \
  --arg site_root "$site_root" \
  --arg runtime_config "$runtime_config" \
  --arg expected_cdn_base_url "$expected_cdn_base_url" \
  --arg cdn_runtime_root "$cdn_runtime_root" \
  --arg runtime_manifest "$runtime_manifest" \
  --arg runtime_module "$runtime_module" \
  --arg runtime_module_path "$runtime_module_path" \
  --arg runtime_wasm "$runtime_wasm" \
  --arg runtime_wasm_path "$runtime_wasm_path" \
  '{
    name: $name,
    passed: true,
    site_root: $site_root,
    runtime_config: $runtime_config,
    expected_cdn_base_url: $expected_cdn_base_url,
    cdn_runtime_root: $cdn_runtime_root,
    runtime_manifest: $runtime_manifest,
    runtime_module: $runtime_module,
    runtime_module_path: $runtime_module_path,
    runtime_wasm: $runtime_wasm,
    runtime_wasm_path: $runtime_wasm_path
  }' >"$site_cdn_probe_file"

bash scripts/recipes/gitops-beta-write-activation-admission-evidence.sh \
  "$output" \
  "$summary_file" \
  "$api_upstream" \
  "$api_meta_file" \
  "$db_probe_file" \
  "$site_cdn_probe_file"

printf 'gitops_beta_admission_observation_ok=%s\n' "$output"
printf 'gitops_beta_admission_observation_dir=%s\n' "$observation_dir"
printf 'gitops_beta_admission_api_meta=%s\n' "$api_meta_file"
printf 'gitops_beta_admission_db_probe=%s\n' "$db_probe_file"
printf 'gitops_beta_admission_site_cdn_probe=%s\n' "$site_cdn_probe_file"
printf 'gitops_beta_admission_api_upstream=%s\n' "$api_upstream"
printf 'local_artifact_written=true\n'
printf 'local_host_mutation_performed=false\n'
printf 'remote_deploy_performed=false\n'
printf 'infrastructure_mutation_performed=false\n'
