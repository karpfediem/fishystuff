#!/usr/bin/env bash
set -euo pipefail

SCRIPT_PATH="$(readlink -f "${BASH_SOURCE[0]}")"
SCRIPT_DIR="$(cd "$(dirname "$SCRIPT_PATH")" && pwd)"
source "${SCRIPT_DIR}/lib/common.sh"

cd "$RECIPE_REPO_ROOT"

pass_count=0

pass() {
  printf '[gitops-beta-deploy-credentials-test] pass: %s\n' "$1"
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
    printf '[gitops-beta-deploy-credentials-test] expected failure: %s\n' "$name" >&2
    exit 1
  fi
  if ! grep -F "$expected" "$stderr" >/dev/null; then
    printf '[gitops-beta-deploy-credentials-test] expected %s stderr to contain %q\n' "$name" "$expected" >&2
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
    if [[ "${FISHYSTUFF_FAKE_SECRETSPEC_GET_DBUS_ERROR:-}" == "1" ]]; then
      echo "DBus error: Failed to connect to socket /run/user/1000/bus: Operation not permitted" >&2
      exit 1
    fi
    name="${*: -1}"
    file="$(secret_path "$name")"
    if [[ ! -s "$file" ]]; then
      exit 1
    fi
    cat "$file"
    ;;
  set)
    shift
    parse_profile "$@"
    args=()
    while (( $# > 0 )); do
      case "$1" in
        --profile | -P)
          shift 2
          ;;
        --profile=*)
          shift
          ;;
        *)
          args+=("$1")
          shift
          ;;
      esac
    done
    if [[ "${#args[@]}" -ne 2 ]]; then
      echo "fake secretspec set expected NAME VALUE" >&2
      exit 2
    fi
    mkdir -p "$store"
    printf '%s\n' "${args[1]}" >"$(secret_path "${args[0]}")"
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

ssh-keygen -q -t ed25519 -a 64 -N "" -C "existing-beta" -f "${root}/existing-beta-key"

present_store="${root}/present-store"
mkdir -p "$present_store"
printf 'token\n' >"${present_store}/HETZNER_API_TOKEN"
printf 'token\n' >"${present_store}/CLOUDFLARE_API_TOKEN"
printf 'fishystuff-beta-deploy\n' >"${present_store}/HETZNER_SSH_KEY_NAME"
cp "${root}/existing-beta-key" "${present_store}/HETZNER_SSH_PRIVATE_KEY"
cp "${root}/existing-beta-key.pub" "${present_store}/HETZNER_SSH_PUBLIC_KEY"

FISHYSTUFF_FAKE_SECRETSPEC_STORE="$present_store" \
  bash scripts/recipes/gitops-beta-deploy-credentials-packet.sh >"${root}/present.packet"
grep -F "gitops_beta_deploy_credentials_packet_ok=true" "${root}/present.packet" >/dev/null
grep -F "beta_deploy_credentials_status=present" "${root}/present.packet" >/dev/null
grep -F "beta_deploy_ssh_key_pair_match=true" "${root}/present.packet" >/dev/null
grep -F "beta_deploy_credentials_next_required_action=run_key_boundary_check" "${root}/present.packet" >/dev/null
grep -F "remote_deploy_performed=false" "${root}/present.packet" >/dev/null
grep -F "infrastructure_mutation_performed=false" "${root}/present.packet" >/dev/null
pass "ready beta deploy credential packet"

FISHYSTUFF_FAKE_SECRETSPEC_STORE="$present_store" \
  FISHYSTUFF_FAKE_SECRETSPEC_GET_DBUS_ERROR=1 \
  bash scripts/recipes/gitops-beta-deploy-credentials-packet.sh >"${root}/unavailable.packet"
grep -F "beta_deploy_credentials_status=unavailable" "${root}/unavailable.packet" >/dev/null
grep -F "beta_deploy_hetzner_api_token_status=unavailable" "${root}/unavailable.packet" >/dev/null
grep -F "beta_deploy_ssh_private_key_status=unavailable" "${root}/unavailable.packet" >/dev/null
grep -F "beta_deploy_credentials_next_required_action=load_or_unlock_beta_deploy_secrets" "${root}/unavailable.packet" >/dev/null
pass "unavailable beta deploy credential packet"

missing_store="${root}/missing-store"
mkdir -p "$missing_store"
printf 'token\n' >"${missing_store}/HETZNER_API_TOKEN"
FISHYSTUFF_FAKE_SECRETSPEC_STORE="$missing_store" \
  bash scripts/recipes/gitops-beta-deploy-credentials-packet.sh >"${root}/missing.packet"
grep -F "beta_deploy_credentials_status=missing" "${root}/missing.packet" >/dev/null
grep -F "beta_deploy_ssh_private_key_status=missing" "${root}/missing.packet" >/dev/null
grep -F "beta_deploy_ssh_public_key_status=missing" "${root}/missing.packet" >/dev/null
grep -F "beta_deploy_credentials_next_required_action=generate_or_store_beta_deploy_key" "${root}/missing.packet" >/dev/null
grep -F "FISHYSTUFF_GITOPS_ENABLE_BETA_DEPLOY_KEY_GENERATE=1 just gitops-beta-deploy-key-ensure" "${root}/missing.packet" >/dev/null
pass "missing beta deploy credential packet"

FISHYSTUFF_FAKE_SECRETSPEC_STORE="$missing_store" \
  FISHYSTUFF_GITOPS_ENABLE_BETA_DEPLOY_KEY_GENERATE=1 \
  bash scripts/recipes/gitops-beta-deploy-key-ensure.sh >"${root}/ensure.packet"
grep -F "gitops_beta_deploy_key_ensure_ok=stored" "${root}/ensure.packet" >/dev/null
grep -F "beta_deploy_key_ensure_action=generated_and_stored" "${root}/ensure.packet" >/dev/null
grep -F "local_host_mutation_performed=true" "${root}/ensure.packet" >/dev/null
test -s "${missing_store}/HETZNER_SSH_PRIVATE_KEY"
test -s "${missing_store}/HETZNER_SSH_PUBLIC_KEY"
FISHYSTUFF_FAKE_SECRETSPEC_STORE="$missing_store" \
  bash scripts/recipes/gitops-beta-deploy-credentials-packet.sh >"${root}/after-ensure.packet"
grep -F "beta_deploy_ssh_key_pair_match=true" "${root}/after-ensure.packet" >/dev/null
pass "generate and store missing beta deploy key"

FISHYSTUFF_FAKE_SECRETSPEC_STORE="$present_store" \
  FISHYSTUFF_GITOPS_ENABLE_BETA_DEPLOY_KEY_GENERATE=1 \
  bash scripts/recipes/gitops-beta-deploy-key-ensure.sh >"${root}/already.packet"
grep -F "gitops_beta_deploy_key_ensure_ok=already_present" "${root}/already.packet" >/dev/null
grep -F "local_host_mutation_performed=false" "${root}/already.packet" >/dev/null
pass "skip existing beta deploy key"

expect_fail_contains \
  "refuse key generation when secrets are unavailable" \
  "beta deploy credentials are unavailable" \
  env FISHYSTUFF_FAKE_SECRETSPEC_STORE="$present_store" FISHYSTUFF_FAKE_SECRETSPEC_GET_DBUS_ERROR=1 FISHYSTUFF_GITOPS_ENABLE_BETA_DEPLOY_KEY_GENERATE=1 \
    bash scripts/recipes/gitops-beta-deploy-key-ensure.sh

expect_fail_contains \
  "reject production profile for credential packet" \
  "must not run with production SecretSpec profile active" \
  env FISHYSTUFF_OPERATOR_SECRETSPEC_PROFILE=production-deploy FISHYSTUFF_FAKE_SECRETSPEC_STORE="$present_store" \
    bash scripts/recipes/gitops-beta-deploy-credentials-packet.sh

expect_fail_contains \
  "reject key generation without opt-in" \
  "gitops-beta-deploy-key-ensure requires FISHYSTUFF_GITOPS_ENABLE_BETA_DEPLOY_KEY_GENERATE=1" \
  env FISHYSTUFF_FAKE_SECRETSPEC_STORE="$missing_store" \
    bash scripts/recipes/gitops-beta-deploy-key-ensure.sh

printf '[gitops-beta-deploy-credentials-test] %s checks passed\n' "$pass_count"
