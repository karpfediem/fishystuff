#!/usr/bin/env bash
set -euo pipefail

SCRIPT_PATH="$(readlink -f "${BASH_SOURCE[0]}")"
SCRIPT_DIR="$(cd "$(dirname "$SCRIPT_PATH")" && pwd)"
source "${SCRIPT_DIR}/lib/common.sh"

output_dir="$(normalize_named_arg output_dir "${1-data/gitops}")"
draft_file="$(normalize_named_arg draft_file "${2-data/gitops/production-activation.draft.desired.json}")"
summary_file="$(normalize_named_arg summary_file "${3-data/gitops/production-current.handoff-summary.json}")"
admission_file="$(normalize_named_arg admission_file "${4-}")"
edge_bundle="$(normalize_named_arg edge_bundle "${5-auto}")"
deploy_bin="$(normalize_named_arg deploy_bin "${6-auto}")"
run_helper_tests="$(normalize_named_arg run_helper_tests "${7-true}")"
served_state_dir="$(normalize_named_arg served_state_dir "${8-}")"
rollback_set_path="$(normalize_named_arg rollback_set_path "${9-}")"
state_dir="$(normalize_named_arg state_dir "${10-/var/lib/fishystuff/gitops}")"
run_dir="$(normalize_named_arg run_dir "${11-/run/fishystuff/gitops}")"
systemd_unit_path="$(normalize_named_arg systemd_unit_path "${12-/etc/systemd/system/fishystuff-edge.service}")"
tls_fullchain_path="$(normalize_named_arg tls_fullchain_path "${13-/run/fishystuff/edge/tls/fullchain.pem}")"
tls_privkey_path="$(normalize_named_arg tls_privkey_path "${14-/run/fishystuff/edge/tls/privkey.pem}")"
environment="$(normalize_named_arg environment "${15-production}")"

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

  printf 'gitops_production_operator_proof_step_start=%s\n' "$name" >&2
  if "$@" >"$stdout" 2>"$stderr"; then
    if [[ -s "$stderr" ]]; then
      sed "s/^/[${name}] /" "$stderr" >&2
    fi
    printf 'gitops_production_operator_proof_step_pass=%s\n' "$name" >&2
    return
  fi

  printf 'gitops_production_operator_proof_step_fail=%s\n' "$name" >&2
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

if [[ "$output_dir" == "-" || -z "$output_dir" ]]; then
  echo "gitops-production-operator-proof requires an output directory, not '-'" >&2
  exit 2
fi
if [[ -z "$admission_file" ]]; then
  admission_file="${FISHYSTUFF_GITOPS_ADMISSION_EVIDENCE_FILE:-}"
fi
if [[ -z "$admission_file" ]]; then
  echo "gitops-production-operator-proof requires admission_file or FISHYSTUFF_GITOPS_ADMISSION_EVIDENCE_FILE" >&2
  exit 2
fi
if [[ -z "$environment" ]]; then
  echo "environment must not be empty" >&2
  exit 2
fi

output_dir="$(absolute_path "$output_dir")"
draft_file="$(absolute_path "$draft_file")"
summary_file="$(absolute_path "$summary_file")"
admission_file="$(absolute_path "$admission_file")"
state_dir="$(absolute_path "$state_dir")"
run_dir="$(absolute_path "$run_dir")"
systemd_unit_path="$(absolute_path "$systemd_unit_path")"
tls_fullchain_path="$(absolute_path "$tls_fullchain_path")"
tls_privkey_path="$(absolute_path "$tls_privkey_path")"
if [[ -n "$served_state_dir" ]]; then
  served_state_dir="$(absolute_path "$served_state_dir")"
fi
if [[ -n "$rollback_set_path" ]]; then
  rollback_set_path="$(absolute_path "$rollback_set_path")"
fi

mkdir -p "$output_dir"
tmp_dir="$(mktemp -d)"
cleanup() {
  rm -rf "$tmp_dir"
}
trap cleanup EXIT

inventory_cmd=(
  bash scripts/recipes/gitops-production-host-inventory.sh
  "$state_dir"
  "$run_dir"
  "$edge_bundle"
  "$systemd_unit_path"
  "$tls_fullchain_path"
  "$tls_privkey_path"
  "$environment"
)
preflight_cmd=(
  bash scripts/recipes/gitops-production-preflight.sh
  "$draft_file"
  "$summary_file"
  "$admission_file"
  "$edge_bundle"
  "$deploy_bin"
  "$run_helper_tests"
  "$served_state_dir"
  "$rollback_set_path"
)
handoff_plan_cmd=(
  bash scripts/recipes/gitops-production-host-handoff-plan.sh
  "$draft_file"
  "$summary_file"
  "$admission_file"
  "$edge_bundle"
  "$deploy_bin"
)

run_capture inventory "${inventory_cmd[@]}"
run_capture preflight "${preflight_cmd[@]}"
run_capture host_handoff_plan "${handoff_plan_cmd[@]}"

created_at="$(date -u '+%Y-%m-%dT%H:%M:%SZ')"
timestamp="$(date -u '+%Y%m%dT%H%M%SZ')"
proof_file="${output_dir%/}/production-operator-proof.${timestamp}.json"
if [[ -e "$proof_file" ]]; then
  proof_file="${output_dir%/}/production-operator-proof.${timestamp}.$$.json"
fi

draft_sha256="$(file_sha256_or_empty "$draft_file")"
summary_sha256="$(file_sha256_or_empty "$summary_file")"
admission_sha256="$(file_sha256_or_empty "$admission_file")"

jq -n \
  --arg schema "fishystuff.gitops.production-operator-proof.v1" \
  --arg created_at "$created_at" \
  --arg environment "$environment" \
  --arg output_path "$proof_file" \
  --arg draft_file "$draft_file" \
  --arg draft_sha256 "$draft_sha256" \
  --arg summary_file "$summary_file" \
  --arg summary_sha256 "$summary_sha256" \
  --arg admission_file "$admission_file" \
  --arg admission_sha256 "$admission_sha256" \
  --arg edge_bundle "$edge_bundle" \
  --arg deploy_bin "$deploy_bin" \
  --arg run_helper_tests "$run_helper_tests" \
  --arg served_state_dir "$served_state_dir" \
  --arg rollback_set_path "$rollback_set_path" \
  --arg state_dir "$state_dir" \
  --arg run_dir "$run_dir" \
  --arg systemd_unit_path "$systemd_unit_path" \
  --arg tls_fullchain_path "$tls_fullchain_path" \
  --arg tls_privkey_path "$tls_privkey_path" \
  --rawfile inventory_stdout "${tmp_dir}/inventory.stdout" \
  --rawfile inventory_stderr "${tmp_dir}/inventory.stderr" \
  --rawfile preflight_stdout "${tmp_dir}/preflight.stdout" \
  --rawfile preflight_stderr "${tmp_dir}/preflight.stderr" \
  --rawfile handoff_plan_stdout "${tmp_dir}/host_handoff_plan.stdout" \
  --rawfile handoff_plan_stderr "${tmp_dir}/host_handoff_plan.stderr" \
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
        edge_bundle: $edge_bundle,
        deploy_bin: $deploy_bin,
        run_helper_tests: $run_helper_tests,
        served_state_dir: $served_state_dir,
        rollback_set_path: $rollback_set_path,
        state_dir: $state_dir,
        run_dir: $run_dir,
        systemd_unit_path: $systemd_unit_path,
        tls_fullchain_path: $tls_fullchain_path,
        tls_privkey_path: $tls_privkey_path
      },
      commands: {
        inventory: {
          argv: $ARGS.positional[0:9],
          success: true,
          stdout: $inventory_stdout,
          stderr: $inventory_stderr,
          kv: kv($inventory_stdout)
        },
        preflight: {
          argv: $ARGS.positional[9:19],
          success: true,
          stdout: $preflight_stdout,
          stderr: $preflight_stderr,
          kv: kv($preflight_stdout)
        },
        host_handoff_plan: {
          argv: $ARGS.positional[19:26],
          success: true,
          stdout: $handoff_plan_stdout,
          stderr: $handoff_plan_stderr,
          kv: kv($handoff_plan_stdout)
        }
      },
      remote_deploy_performed: false,
      infrastructure_mutation_performed: false
    }
  ' \
  --args \
  "${inventory_cmd[@]}" \
  "${preflight_cmd[@]}" \
  "${handoff_plan_cmd[@]}" >"$proof_file"

printf 'gitops_production_operator_proof_ok=%s\n' "$proof_file"
printf 'gitops_production_operator_proof_environment=%s\n' "$environment"
printf 'gitops_production_operator_proof_draft_sha256=%s\n' "$draft_sha256"
printf 'gitops_production_operator_proof_summary_sha256=%s\n' "$summary_sha256"
printf 'gitops_production_operator_proof_admission_sha256=%s\n' "$admission_sha256"
printf 'remote_deploy_performed=false\n'
printf 'infrastructure_mutation_performed=false\n'
