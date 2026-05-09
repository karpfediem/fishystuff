#!/usr/bin/env bash
set -euo pipefail

SCRIPT_PATH="$(readlink -f "${BASH_SOURCE[0]}")"
SCRIPT_DIR="$(cd "$(dirname "$SCRIPT_PATH")" && pwd)"
source "${SCRIPT_DIR}/lib/common.sh"

cd "$RECIPE_REPO_ROOT"

pass_count=0

pass() {
  printf '[gitops-beta-host-provision-plan-test] pass: %s\n' "$1"
  pass_count="$((pass_count + 1))"
}

expect_fail_contains() {
  local name="$1"
  local expected="$2"
  shift 2
  local root=""
  local stderr=""

  root="$(mktemp -d)"
  stderr="${root}/stderr"
  if "$@" >"${root}/stdout" 2>"$stderr"; then
    printf '[gitops-beta-host-provision-plan-test] expected failure: %s\n' "$name" >&2
    exit 1
  fi
  if ! grep -F "$expected" "$stderr" >/dev/null; then
    printf '[gitops-beta-host-provision-plan-test] expected %s stderr to contain %q\n' "$name" "$expected" >&2
    cat "$stderr" >&2
    exit 1
  fi
  pass "$name"
}

write_fake_secretspec() {
  local path="$1"

  cat >"$path" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail

store="${FISHYSTUFF_FAKE_SECRETSPEC_STORE:?}"
profile=""

parse_profile() {
  while (( $# > 0 )); do
    case "$1" in
      --profile | -P)
        shift
        profile="${1-}"
        ;;
      --profile=*)
        profile="${1#*=}"
        ;;
    esac
    shift || true
  done
  if [[ "$profile" != "beta-deploy" ]]; then
    echo "fake secretspec expected beta-deploy profile, got: ${profile}" >&2
    exit 2
  fi
}

secret_path() {
  printf '%s/%s' "$store" "$1"
}

case "${1-}" in
  check)
    shift
    parse_profile "$@"
    test -s "$(secret_path HETZNER_API_TOKEN)"
    test -s "$(secret_path HETZNER_SSH_PRIVATE_KEY)"
    test -s "$(secret_path HETZNER_SSH_PUBLIC_KEY)"
    ;;
  get)
    shift
    parse_profile "$@"
    name="${*: -1}"
    file="$(secret_path "$name")"
    if [[ ! -s "$file" ]]; then
      exit 1
    fi
    cat "$file"
    ;;
  *)
    echo "unsupported fake secretspec command: $*" >&2
    exit 2
    ;;
esac
EOF
  chmod +x "$path"
}

root="$(mktemp -d)"
fake_bin="${root}/bin"
mkdir -p "$fake_bin"
write_fake_secretspec "${fake_bin}/secretspec"
PATH="${fake_bin}:$PATH"

ssh-keygen -q -t ed25519 -a 64 -N "" -C "beta-provision" -f "${root}/beta-provision-key"

present_store="${root}/present-store"
mkdir -p "$present_store"
printf 'token\n' >"${present_store}/HETZNER_API_TOKEN"
printf 'token\n' >"${present_store}/CLOUDFLARE_API_TOKEN"
printf 'fishystuff-beta-deploy\n' >"${present_store}/HETZNER_SSH_KEY_NAME"
cp "${root}/beta-provision-key" "${present_store}/HETZNER_SSH_PRIVATE_KEY"
cp "${root}/beta-provision-key.pub" "${present_store}/HETZNER_SSH_PUBLIC_KEY"

FISHYSTUFF_FAKE_SECRETSPEC_STORE="$present_store" \
  bash scripts/recipes/gitops-beta-host-provision-plan.sh >"${root}/ready.stdout"
grep -F "gitops_beta_host_provision_plan_ok=true" "${root}/ready.stdout" >/dev/null
grep -F "provision_plan_status=ready_for_manual_confirmation" "${root}/ready.stdout" >/dev/null
grep -F "provision_ready=true" "${root}/ready.stdout" >/dev/null
grep -F "manual_confirmation_required=true" "${root}/ready.stdout" >/dev/null
grep -F "host_name=site-nbg1-beta" "${root}/ready.stdout" >/dev/null
grep -F "host_expected_hostname=site-nbg1-beta" "${root}/ready.stdout" >/dev/null
grep -F "host_name_matches_expected_hostname=true" "${root}/ready.stdout" >/dev/null
grep -F "host_location=nbg1" "${root}/ready.stdout" >/dev/null
grep -F "host_datacenter=nbg1-dc3" "${root}/ready.stdout" >/dev/null
grep -F "host_server_type=cx33" "${root}/ready.stdout" >/dev/null
grep -F "host_image=debian-13" "${root}/ready.stdout" >/dev/null
grep -F "host_ssh_key_name=fishystuff-beta-deploy" "${root}/ready.stdout" >/dev/null
grep -F "resident_target_dns_cutover_warning=do_not_use_public_beta_dns_for_new_host_until_operator_confirms_it_points_at_the_new_host" "${root}/ready.stdout" >/dev/null
grep -F "beta_deploy_credentials_status=present" "${root}/ready.stdout" >/dev/null
grep -F "beta_deploy_credentials_ssh_key_pair_match=true" "${root}/ready.stdout" >/dev/null
grep -F "read_only_check_01=just gitops-beta-deploy-credentials-packet" "${root}/ready.stdout" >/dev/null
grep -F "read_only_check_02=just deploy-key-boundary-check" "${root}/ready.stdout" >/dev/null
grep -F "hcloud_command_emitted=false" "${root}/ready.stdout" >/dev/null
grep -F "ssh_command_emitted=false" "${root}/ready.stdout" >/dev/null
grep -F "dns_mutation_command_emitted=false" "${root}/ready.stdout" >/dev/null
grep -F "remote_deploy_performed=false" "${root}/ready.stdout" >/dev/null
grep -F "infrastructure_mutation_performed=false" "${root}/ready.stdout" >/dev/null
grep -F "local_host_mutation_performed=false" "${root}/ready.stdout" >/dev/null
pass "ready beta host provision plan"

missing_store="${root}/missing-store"
mkdir -p "$missing_store"
printf 'token\n' >"${missing_store}/HETZNER_API_TOKEN"
FISHYSTUFF_FAKE_SECRETSPEC_STORE="$missing_store" \
  bash scripts/recipes/gitops-beta-host-provision-plan.sh >"${root}/missing.stdout"
grep -F "provision_plan_status=pending_beta_deploy_credentials" "${root}/missing.stdout" >/dev/null
grep -F "provision_ready=false" "${root}/missing.stdout" >/dev/null
grep -F "beta_deploy_credentials_status=missing" "${root}/missing.stdout" >/dev/null
grep -F "beta_deploy_credentials_next_required_action=generate_or_store_beta_deploy_key" "${root}/missing.stdout" >/dev/null
grep -F "hcloud_command_emitted=false" "${root}/missing.stdout" >/dev/null
pass "pending credential beta host provision plan"

FISHYSTUFF_FAKE_SECRETSPEC_STORE="$present_store" \
  bash scripts/recipes/gitops-beta-host-provision-plan.sh host_name=beta-next.example >"${root}/override.stdout"
grep -F "host_name=beta-next.example" "${root}/override.stdout" >/dev/null
grep -F "host_expected_hostname=site-nbg1-beta" "${root}/override.stdout" >/dev/null
grep -F "host_name_matches_expected_hostname=false" "${root}/override.stdout" >/dev/null
pass "custom host name beta host provision plan"

expect_fail_contains \
  "reject production profile" \
  "must not run with production SecretSpec profile active" \
  env FISHYSTUFF_OPERATOR_SECRETSPEC_PROFILE=production-deploy FISHYSTUFF_FAKE_SECRETSPEC_STORE="$present_store" \
    bash scripts/recipes/gitops-beta-host-provision-plan.sh

expect_fail_contains \
  "reject production-looking host" \
  "beta host_name must not look like production" \
  env FISHYSTUFF_FAKE_SECRETSPEC_STORE="$present_store" \
    bash scripts/recipes/gitops-beta-host-provision-plan.sh host_name=site-nbg1-prod

expect_fail_contains \
  "reject non-nbg1 location" \
  "restricted to location=nbg1" \
  env FISHYSTUFF_FAKE_SECRETSPEC_STORE="$present_store" \
    bash scripts/recipes/gitops-beta-host-provision-plan.sh site-nbg1-beta cx33 debian-13 fsn1

printf '[gitops-beta-host-provision-plan-test] %s checks passed\n' "$pass_count"
