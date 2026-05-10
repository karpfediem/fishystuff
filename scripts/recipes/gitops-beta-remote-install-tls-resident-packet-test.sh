#!/usr/bin/env bash
set -euo pipefail

SCRIPT_PATH="$(readlink -f "${BASH_SOURCE[0]}")"
SCRIPT_DIR="$(cd "$(dirname "$SCRIPT_PATH")" && pwd)"
source "${SCRIPT_DIR}/lib/common.sh"

cd "$RECIPE_REPO_ROOT"

pass_count=0

pass() {
  printf '[gitops-beta-remote-install-tls-resident-packet-test] pass: %s\n' "$1"
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
    printf '[gitops-beta-remote-install-tls-resident-packet-test] expected failure: %s\n' "$name" >&2
    exit 1
  fi
  if ! grep -F "$expected" "$stderr" >/dev/null; then
    printf '[gitops-beta-remote-install-tls-resident-packet-test] expected %s stderr to contain %q\n' "$name" "$expected" >&2
    cat "$stderr" >&2
    exit 1
  fi
  pass "$name"
}

root="$(mktemp -d)"
mgmt_dir="${root}/mgmt/bin"
gitops_dir="${root}/gitops"
desired="${root}/beta-tls.desired.json"
unit="${root}/fishystuff-beta-tls-reconciler.service"
token="${root}/cloudflare-api-token"
mkdir -p "$mgmt_dir" "$gitops_dir"

cat >"${mgmt_dir}/mgmt" <<'EOF'
#!/usr/bin/env bash
printf 'fake mgmt\n'
EOF
chmod +x "${mgmt_dir}/mgmt"
printf '# fake main\n' >"${gitops_dir}/main.mcl"
printf 'fake-cloudflare-token\n' >"$token"

env FISHYSTUFF_GITOPS_BETA_ACME_CONTACT_EMAIL=ops@fishystuff.invalid \
  bash scripts/recipes/gitops-beta-tls-desired.sh "$desired" staging "" >/dev/null 2>"${root}/desired.stderr"
bash scripts/recipes/gitops-beta-tls-resident-unit.sh \
  "$unit" \
  /var/lib/fishystuff/gitops-beta/desired/beta-tls.desired.json \
  "${mgmt_dir}/mgmt" \
  "$gitops_dir" \
  /var/lib/fishystuff/gitops-beta/secrets/cloudflare-api-token \
  -1 >/dev/null 2>"${root}/unit.stderr"

read -r desired_sha256 _ < <(sha256sum "$desired")
read -r unit_sha256 _ < <(sha256sum "$unit")
read -r token_sha256 _ < <(sha256sum "$token")

env FISHYSTUFF_OPERATOR_SECRETSPEC_PROFILE=beta-deploy CLOUDFLARE_API_TOKEN=fake-cloudflare-token \
  bash scripts/recipes/gitops-beta-remote-install-tls-resident-packet.sh \
    root@203.0.113.20 \
    site-nbg1-beta \
    "$desired" \
    "$unit" \
    env:CLOUDFLARE_API_TOKEN >"${root}/ready.stdout"
grep -F "beta_remote_tls_resident_install_packet_status=ready" "${root}/ready.stdout" >/dev/null
grep -F "beta_remote_tls_resident_install_packet_target_status=ready" "${root}/ready.stdout" >/dev/null
grep -F "beta_remote_tls_resident_install_packet_desired_sha256=${desired_sha256}" "${root}/ready.stdout" >/dev/null
grep -F "beta_remote_tls_resident_install_packet_unit_sha256=${unit_sha256}" "${root}/ready.stdout" >/dev/null
grep -F "beta_remote_tls_resident_install_packet_cloudflare_token_sha256=${token_sha256}" "${root}/ready.stdout" >/dev/null
grep -F "FISHYSTUFF_GITOPS_ENABLE_BETA_REMOTE_TLS_RESIDENT_INSTALL=1 FISHYSTUFF_GITOPS_ENABLE_BETA_REMOTE_TLS_RESIDENT_CLOSURE_COPY=1 FISHYSTUFF_GITOPS_ENABLE_BETA_REMOTE_TLS_RESIDENT_RESTART=1" "${root}/ready.stdout" >/dev/null
grep -F "FISHYSTUFF_GITOPS_BETA_REMOTE_TLS_RESIDENT_TARGET=root@203.0.113.20" "${root}/ready.stdout" >/dev/null
grep -F "secretspec run --profile beta-deploy -- just gitops-beta-remote-install-tls-resident target=root@203.0.113.20 expected_hostname=site-nbg1-beta" "${root}/ready.stdout" >/dev/null
if grep -F "fake-cloudflare-token" "${root}/ready.stdout" >/dev/null; then
  printf '[gitops-beta-remote-install-tls-resident-packet-test] packet leaked token value\n' >&2
  exit 1
fi
pass "ready remote install packet"

env FISHYSTUFF_OPERATOR_SECRETSPEC_PROFILE=beta-deploy \
  bash scripts/recipes/gitops-beta-remote-install-tls-resident-packet.sh \
    root@203.0.113.20 \
    site-nbg1-beta \
    "$desired" \
    "$unit" \
    env:CLOUDFLARE_API_TOKEN >"${root}/missing-env.stdout"
grep -F "beta_remote_tls_resident_install_packet_status=pending_inputs" "${root}/missing-env.stdout" >/dev/null
grep -F "beta_remote_tls_resident_install_packet_cloudflare_token_status=missing_env" "${root}/missing-env.stdout" >/dev/null
grep -F "beta_remote_tls_resident_install_packet_next_required_action=load_beta_deploy_cloudflare_token" "${root}/missing-env.stdout" >/dev/null
pass "pending missing env token"

env FISHYSTUFF_OPERATOR_SECRETSPEC_PROFILE=beta-deploy CLOUDFLARE_API_TOKEN=fake-cloudflare-token \
  bash scripts/recipes/gitops-beta-remote-install-tls-resident-packet.sh \
    root@178.104.230.121 \
    site-nbg1-beta \
    "$desired" \
    "$unit" \
    env:CLOUDFLARE_API_TOKEN >"${root}/previous-host.stdout"
grep -F "beta_remote_tls_resident_install_packet_status=blocked_target" "${root}/previous-host.stdout" >/dev/null
grep -F "beta_remote_tls_resident_install_packet_target_status=blocked_previous_beta_host" "${root}/previous-host.stdout" >/dev/null
pass "blocked previous beta host"

env FISHYSTUFF_OPERATOR_SECRETSPEC_PROFILE=production-deploy CLOUDFLARE_API_TOKEN=fake-cloudflare-token \
  bash scripts/recipes/gitops-beta-remote-install-tls-resident-packet.sh \
    root@203.0.113.20 \
    site-nbg1-beta \
    "$desired" \
    "$unit" \
    env:CLOUDFLARE_API_TOKEN >"${root}/production-profile.stdout"
grep -F "beta_remote_tls_resident_install_packet_status=blocked_profile" "${root}/production-profile.stdout" >/dev/null
grep -F "beta_remote_tls_resident_install_packet_next_required_action=load_beta_deploy_or_no_operator_profile" "${root}/production-profile.stdout" >/dev/null
pass "blocked production profile"

bad_unit="${root}/bad.service"
cp "$unit" "$bad_unit"
printf '\nEnvironmentFile=/var/lib/fishystuff/gitops-beta/secrets/acme.env\n' >>"$bad_unit"
expect_fail_contains \
  "reject dotenv-style unit" \
  "beta TLS resident unit must use LoadCredential, not EnvironmentFile" \
  env FISHYSTUFF_OPERATOR_SECRETSPEC_PROFILE=beta-deploy CLOUDFLARE_API_TOKEN=fake-cloudflare-token \
    bash scripts/recipes/gitops-beta-remote-install-tls-resident-packet.sh root@203.0.113.20 site-nbg1-beta "$desired" "$bad_unit" env:CLOUDFLARE_API_TOKEN

printf '[gitops-beta-remote-install-tls-resident-packet-test] %s checks passed\n' "$pass_count"
