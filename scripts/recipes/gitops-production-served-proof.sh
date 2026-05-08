#!/usr/bin/env bash
set -euo pipefail

SCRIPT_PATH="$(readlink -f "${BASH_SOURCE[0]}")"
SCRIPT_DIR="$(cd "$(dirname "$SCRIPT_PATH")" && pwd)"
source "${SCRIPT_DIR}/lib/common.sh"

output_dir="$(normalize_named_arg output_dir "${1-data/gitops}")"
draft_file="$(normalize_named_arg draft_file "${2-data/gitops/production-activation.draft.desired.json}")"
summary_file="$(normalize_named_arg summary_file "${3-data/gitops/production-current.handoff-summary.json}")"
admission_file="$(normalize_named_arg admission_file "${4-}")"
operator_proof_file="$(normalize_named_arg proof_file "${5-}")"
deploy_bin="$(normalize_named_arg deploy_bin "${6-auto}")"
state_dir="$(normalize_named_arg state_dir "${7-/var/lib/fishystuff/gitops}")"
run_dir="$(normalize_named_arg run_dir "${8-/run/fishystuff/gitops}")"
proof_max_age_seconds="$(normalize_named_arg proof_max_age_seconds "${9-86400}")"

cd "$RECIPE_REPO_ROOT"

require_command() {
  local command_name="$1"
  if ! command -v "$command_name" >/dev/null 2>&1; then
    echo "missing required command: ${command_name}" >&2
    exit 127
  fi
}

absolute_path() {
  local path="$1"
  if [[ "$path" == /* ]]; then
    printf '%s' "$path"
    return
  fi
  printf '%s/%s' "$RECIPE_REPO_ROOT" "$path"
}

file_sha256_or_empty() {
  local path="$1"
  local sha=""
  if [[ -f "$path" ]]; then
    read -r sha _ < <(sha256sum "$path")
  fi
  printf '%s' "$sha"
}

run_capture() {
  local name="$1"
  shift
  local stdout="${tmp_dir}/${name}.stdout"
  local stderr="${tmp_dir}/${name}.stderr"

  printf 'gitops_production_served_proof_step_start=%s\n' "$name" >&2
  if "$@" >"$stdout" 2>"$stderr"; then
    if [[ -s "$stderr" ]]; then
      sed "s/^/[${name}] /" "$stderr" >&2
    fi
    printf 'gitops_production_served_proof_step_pass=%s\n' "$name" >&2
    return
  fi

  printf 'gitops_production_served_proof_step_fail=%s\n' "$name" >&2
  if [[ -s "$stdout" ]]; then
    sed "s/^/[${name}:stdout] /" "$stdout" >&2
  fi
  if [[ -s "$stderr" ]]; then
    sed "s/^/[${name}:stderr] /" "$stderr" >&2
  fi
  exit 1
}

require_command date
require_command jq
require_command mkdir
require_command mktemp
require_command sed
require_command sha256sum

case "$proof_max_age_seconds" in
  '' | *[!0-9]*)
    echo "proof_max_age_seconds must be a non-negative integer, got: ${proof_max_age_seconds}" >&2
    exit 2
    ;;
esac

if [[ "$output_dir" == "-" || -z "$output_dir" ]]; then
  echo "gitops-production-served-proof requires an output directory, not '-'" >&2
  exit 2
fi
if [[ -z "$admission_file" ]]; then
  admission_file="${FISHYSTUFF_GITOPS_ADMISSION_EVIDENCE_FILE:-}"
fi
if [[ -z "$admission_file" ]]; then
  echo "gitops-production-served-proof requires admission_file or FISHYSTUFF_GITOPS_ADMISSION_EVIDENCE_FILE" >&2
  exit 2
fi
if [[ -z "$operator_proof_file" ]]; then
  operator_proof_file="${FISHYSTUFF_GITOPS_OPERATOR_PROOF_FILE:-}"
fi
if [[ -z "$operator_proof_file" ]]; then
  echo "gitops-production-served-proof requires proof_file or FISHYSTUFF_GITOPS_OPERATOR_PROOF_FILE" >&2
  exit 2
fi

output_dir="$(absolute_path "$output_dir")"
draft_file="$(absolute_path "$draft_file")"
summary_file="$(absolute_path "$summary_file")"
admission_file="$(absolute_path "$admission_file")"
operator_proof_file="$(absolute_path "$operator_proof_file")"
state_dir="$(absolute_path "$state_dir")"
run_dir="$(absolute_path "$run_dir")"

if [[ ! -f "$operator_proof_file" ]]; then
  echo "production operator proof does not exist: ${operator_proof_file}" >&2
  exit 2
fi

proof_draft_file="$(absolute_path "$(jq -er '.inputs.draft_file' "$operator_proof_file")")"
proof_summary_file="$(absolute_path "$(jq -er '.inputs.summary_file' "$operator_proof_file")")"
proof_admission_file="$(absolute_path "$(jq -er '.inputs.admission_file' "$operator_proof_file")")"
if [[ "$draft_file" != "$proof_draft_file" ]]; then
  echo "operator proof draft_file does not match activation draft" >&2
  echo "served proof: ${draft_file}" >&2
  echo "operator proof: ${proof_draft_file}" >&2
  exit 2
fi
if [[ "$summary_file" != "$proof_summary_file" ]]; then
  echo "operator proof summary_file does not match handoff summary" >&2
  echo "served proof: ${summary_file}" >&2
  echo "operator proof: ${proof_summary_file}" >&2
  exit 2
fi
if [[ "$admission_file" != "$proof_admission_file" ]]; then
  echo "operator proof admission_file does not match admission evidence" >&2
  echo "served proof: ${admission_file}" >&2
  echo "operator proof: ${proof_admission_file}" >&2
  exit 2
fi

mkdir -p "$output_dir"
tmp_dir="$(mktemp -d)"
cleanup() {
  rm -rf "$tmp_dir"
}
trap cleanup EXIT

operator_proof_check_cmd=(
  bash scripts/recipes/gitops-check-production-operator-proof.sh
  "$operator_proof_file"
  "$proof_max_age_seconds"
  ""
)
served_verification_cmd=(
  bash scripts/recipes/gitops-verify-activation-served.sh
  "$draft_file"
  "$summary_file"
  "$admission_file"
  "$deploy_bin"
  "$state_dir"
  "$run_dir"
)

run_capture operator_proof_check "${operator_proof_check_cmd[@]}"
run_capture served_verification "${served_verification_cmd[@]}"

created_at="$(date -u '+%Y-%m-%dT%H:%M:%SZ')"
timestamp="$(date -u '+%Y%m%dT%H%M%SZ')"
proof_file="${output_dir%/}/production-served-proof.${timestamp}.json"
if [[ -e "$proof_file" ]]; then
  proof_file="${output_dir%/}/production-served-proof.${timestamp}.$$.json"
fi

draft_sha256="$(file_sha256_or_empty "$draft_file")"
summary_sha256="$(file_sha256_or_empty "$summary_file")"
admission_sha256="$(file_sha256_or_empty "$admission_file")"
operator_proof_sha256="$(file_sha256_or_empty "$operator_proof_file")"

release_id="$(awk -F= '$1 == "gitops_activation_served_ok" { print $2 }' "${tmp_dir}/served_verification.stdout")"
generation="$(awk -F= '$1 == "gitops_activation_served_generation" { print $2 }' "${tmp_dir}/served_verification.stdout")"
require_value "$release_id" "served verification did not report a release ID"
require_value "$generation" "served verification did not report a generation"

jq -n \
  --arg schema "fishystuff.gitops.production-served-proof.v1" \
  --arg created_at "$created_at" \
  --arg environment "production" \
  --arg output_path "$proof_file" \
  --arg draft_file "$draft_file" \
  --arg draft_sha256 "$draft_sha256" \
  --arg summary_file "$summary_file" \
  --arg summary_sha256 "$summary_sha256" \
  --arg admission_file "$admission_file" \
  --arg admission_sha256 "$admission_sha256" \
  --arg operator_proof_file "$operator_proof_file" \
  --arg operator_proof_sha256 "$operator_proof_sha256" \
  --arg deploy_bin "$deploy_bin" \
  --arg state_dir "$state_dir" \
  --arg run_dir "$run_dir" \
  --arg proof_max_age_seconds "$proof_max_age_seconds" \
  --arg release_id "$release_id" \
  --arg generation "$generation" \
  --rawfile operator_proof_check_stdout "${tmp_dir}/operator_proof_check.stdout" \
  --rawfile operator_proof_check_stderr "${tmp_dir}/operator_proof_check.stderr" \
  --rawfile served_verification_stdout "${tmp_dir}/served_verification.stdout" \
  --rawfile served_verification_stderr "${tmp_dir}/served_verification.stderr" \
  '
    def kv($text):
      reduce (
        $text
        | split("\n")[]
        | capture("^(?<key>[A-Za-z0-9_]+)=(?<value>.*)$")?
      ) as $line ({}; .[$line.key] = $line.value);
    {
      schema: $schema,
      created_at: $created_at,
      environment: $environment,
      output_path: $output_path,
      inputs: {
        draft_file: $draft_file,
        draft_sha256: $draft_sha256,
        summary_file: $summary_file,
        summary_sha256: $summary_sha256,
        admission_file: $admission_file,
        admission_sha256: $admission_sha256,
        operator_proof_file: $operator_proof_file,
        operator_proof_sha256: $operator_proof_sha256,
        deploy_bin: $deploy_bin,
        state_dir: $state_dir,
        run_dir: $run_dir,
        proof_max_age_seconds: $proof_max_age_seconds
      },
      served: {
        release_id: $release_id,
        generation: ($generation | tonumber)
      },
      commands: {
        operator_proof_check: {
          argv: $ARGS.positional[0:5],
          success: true,
          stdout: $operator_proof_check_stdout,
          stderr: $operator_proof_check_stderr,
          kv: kv($operator_proof_check_stdout)
        },
        served_verification: {
          argv: $ARGS.positional[5:13],
          success: true,
          stdout: $served_verification_stdout,
          stderr: $served_verification_stderr,
          kv: kv($served_verification_stdout)
        }
      },
      remote_deploy_performed: false,
      infrastructure_mutation_performed: false
    }
  ' \
  --args \
  "${operator_proof_check_cmd[@]}" \
  "${served_verification_cmd[@]}" >"$proof_file"

printf 'gitops_production_served_proof_ok=%s\n' "$proof_file"
printf 'gitops_production_served_proof_environment=production\n'
printf 'gitops_production_served_proof_release_id=%s\n' "$release_id"
printf 'gitops_production_served_proof_generation=%s\n' "$generation"
printf 'gitops_production_served_proof_operator_proof=%s\n' "$operator_proof_file"
printf 'gitops_production_served_proof_operator_proof_sha256=%s\n' "$operator_proof_sha256"
printf 'remote_deploy_performed=false\n'
printf 'infrastructure_mutation_performed=false\n'
