#!/usr/bin/env bash
set -euo pipefail

SCRIPT_PATH="$(readlink -f "${BASH_SOURCE[0]}")"
SCRIPT_DIR="$(cd "$(dirname "$SCRIPT_PATH")" && pwd)"
source "${SCRIPT_DIR}/lib/common.sh"

cd "$RECIPE_REPO_ROOT"

pass_count=0

pass() {
  printf '[gitops-beta-tls-reconcile-packet-test] pass: %s\n' "$1"
  pass_count="$((pass_count + 1))"
}

expect_fail_contains() {
  local name="$1"
  local expected="$2"
  shift 2
  local test_root=""
  local stderr=""

  test_root="$(mktemp -d)"
  stderr="${test_root}/stderr"
  if "$@" >"${test_root}/stdout" 2>"$stderr"; then
    printf '[gitops-beta-tls-reconcile-packet-test] expected failure: %s\n' "$name" >&2
    exit 1
  fi
  if ! grep -F "$expected" "$stderr" >/dev/null; then
    printf '[gitops-beta-tls-reconcile-packet-test] expected %s stderr to contain %q\n' "$name" "$expected" >&2
    cat "$stderr" >&2
    exit 1
  fi
  pass "$name"
}

root="$(mktemp -d)"
fake_bin="${root}/bin"
mkdir -p "$fake_bin"

cat >"${fake_bin}/secretspec" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail

case "$*" in
  "check --profile beta-deploy --no-prompt")
    exit 0
    ;;
  "get --profile beta-deploy CLOUDFLARE_API_TOKEN")
    printf 'fake-cloudflare-token\n'
    exit 0
    ;;
  *)
    exit 2
    ;;
esac
EOF
chmod +x "${fake_bin}/secretspec"
PATH="${fake_bin}:${PATH}"

state="${root}/beta-tls.desired.json"
missing_state="${root}/missing-beta-tls.desired.json"

bash scripts/recipes/gitops-beta-tls-reconcile-packet.sh \
  "$missing_state" \
  staging \
  ops@fishystuff.invalid >"${root}/missing.stdout"
grep -F "beta_tls_packet_status=missing_desired_state" "${root}/missing.stdout" >/dev/null
grep -F "beta_tls_packet_next_required_action=write_tls_desired_state" "${root}/missing.stdout" >/dev/null
grep -F "beta_tls_packet_next_command_01=just gitops-beta-tls-desired output=${missing_state} ca=staging contact_email=ops@fishystuff.invalid" "${root}/missing.stdout" >/dev/null
grep -F "remote_deploy_performed=false" "${root}/missing.stdout" >/dev/null
grep -F "infrastructure_mutation_performed=false" "${root}/missing.stdout" >/dev/null
grep -F "local_host_mutation_performed=false" "${root}/missing.stdout" >/dev/null
pass "missing desired packet"

bash scripts/recipes/gitops-beta-tls-desired.sh \
  "$state" \
  staging \
  ops@fishystuff.invalid >"${root}/desired.stdout" 2>"${root}/desired.stderr"
jq -e '
  .cluster == "beta"
  and .mode == "local-apply"
  and .tls["beta-edge"].directory_url == "https://acme-staging-v02.api.letsencrypt.org/directory"
  and .tls["beta-edge"].fullchain_path == "/var/lib/fishystuff/gitops-beta/tls/live/fullchain.pem"
  and .tls["beta-edge"].reload_service == "fishystuff-beta-edge"
  and .tls["beta-edge"].reload_service_action == "reload-or-try-restart"
  and ((.tls["beta-edge"].domains | sort) == [
    "api.beta.fishystuff.fish",
    "beta.fishystuff.fish",
    "cdn.beta.fishystuff.fish",
    "telemetry.beta.fishystuff.fish"
  ])
' "$state" >/dev/null
grep -F "gitops_beta_tls_desired_ok=true" "${root}/desired.stderr" >/dev/null
pass "staging desired generation"

env_state="${root}/beta-tls.env-contact.desired.json"
env FISHYSTUFF_GITOPS_BETA_ACME_CONTACT_EMAIL=env-ops@fishystuff.invalid \
  bash scripts/recipes/gitops-beta-tls-desired.sh \
    "$env_state" \
    staging \
    "" >"${root}/desired-env.stdout" 2>"${root}/desired-env.stderr"
jq -e '.tls["beta-edge"].contact_email == "env-ops@fishystuff.invalid"' "$env_state" >/dev/null
pass "desired contact email env fallback"

bash scripts/recipes/gitops-beta-tls-reconcile-packet.sh \
  "$state" \
  staging >"${root}/ready.stdout"
grep -F "beta_tls_packet_status=ready" "${root}/ready.stdout" >/dev/null
grep -F "beta_tls_packet_secretspec_profile=beta-deploy" "${root}/ready.stdout" >/dev/null
grep -F "beta_tls_packet_cloudflare_api_token_status=present" "${root}/ready.stdout" >/dev/null
grep -F "beta_tls_packet_reload_service=fishystuff-beta-edge" "${root}/ready.stdout" >/dev/null
grep -F "beta_tls_packet_reload_service_action=reload-or-try-restart" "${root}/ready.stdout" >/dev/null
grep -F "beta_tls_packet_next_required_action=run_staging_acme_reconcile" "${root}/ready.stdout" >/dev/null
grep -F "FISHYSTUFF_GITOPS_ENABLE_BETA_TLS_APPLY=1 FISHYSTUFF_GITOPS_ENABLE_LOCAL_APPLY=1 secretspec run --profile beta-deploy -- just gitops-beta-reconcile-tls state_file=${state} ca=staging" "${root}/ready.stdout" >/dev/null
pass "ready staging packet"

bad_state="${root}/bad-domain.desired.json"
jq '.tls["beta-edge"].domains += ["fishystuff.fish"]' "$state" >"$bad_state"
expect_fail_contains \
  "reject production domain in beta TLS desired state" \
  "beta TLS desired state does not match the guarded beta staging ACME shape" \
  bash scripts/recipes/gitops-beta-tls-reconcile-packet.sh "$bad_state" staging

cat >"${fake_bin}/secretspec" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
printf 'DBus error: Operation not permitted\n' >&2
exit 1
EOF
chmod +x "${fake_bin}/secretspec"

bash scripts/recipes/gitops-beta-tls-reconcile-packet.sh \
  "$state" \
  staging >"${root}/unavailable.stdout"
grep -F "beta_tls_packet_status=blocked_credentials" "${root}/unavailable.stdout" >/dev/null
grep -F "beta_tls_packet_cloudflare_api_token_status=unavailable" "${root}/unavailable.stdout" >/dev/null
grep -F "beta_tls_packet_next_required_action=load_or_unlock_beta_deploy_secrets" "${root}/unavailable.stdout" >/dev/null
pass "unavailable credentials packet"

expect_fail_contains \
  "require explicit production desired opt-in" \
  "gitops-beta-tls-desired requires FISHYSTUFF_GITOPS_ENABLE_BETA_TLS_PRODUCTION_DESIRED=1" \
  bash scripts/recipes/gitops-beta-tls-desired.sh "${root}/production.desired.json" production ops@fishystuff.invalid

printf '[gitops-beta-tls-reconcile-packet-test] %s checks passed\n' "$pass_count"
