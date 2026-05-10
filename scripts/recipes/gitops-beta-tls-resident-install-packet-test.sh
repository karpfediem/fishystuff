#!/usr/bin/env bash
set -euo pipefail

SCRIPT_PATH="$(readlink -f "${BASH_SOURCE[0]}")"
SCRIPT_DIR="$(cd "$(dirname "$SCRIPT_PATH")" && pwd)"
source "${SCRIPT_DIR}/lib/common.sh"

cd "$RECIPE_REPO_ROOT"

pass_count=0

pass() {
  printf '[gitops-beta-tls-resident-install-packet-test] pass: %s\n' "$1"
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
    printf '[gitops-beta-tls-resident-install-packet-test] expected failure: %s\n' "$name" >&2
    exit 1
  fi
  if ! grep -F "$expected" "$stderr" >/dev/null; then
    printf '[gitops-beta-tls-resident-install-packet-test] expected %s stderr to contain %q\n' "$name" "$expected" >&2
    cat "$stderr" >&2
    exit 1
  fi
  pass "$name"
}

root="$(mktemp -d)"
fake_bin="${root}/bin"
mgmt_dir="${root}/mgmt/bin"
gitops_dir="${root}/gitops"
desired="${root}/beta-tls.staging.desired.json"
unit="${root}/fishystuff-beta-tls-reconciler.service"
token="${root}/cloudflare-api-token"
mkdir -p "$fake_bin" "$mgmt_dir" "$gitops_dir"

cat >"${fake_bin}/hostname" <<'EOF'
#!/usr/bin/env bash
if [[ "${1-}" == "-f" ]]; then
  printf '%s\n' "${FISHYSTUFF_FAKE_HOSTNAME:-site-nbg1-beta}"
else
  printf '%s\n' "${FISHYSTUFF_FAKE_HOSTNAME:-site-nbg1-beta}"
fi
EOF
chmod +x "${fake_bin}/hostname"

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
  /var/lib/fishystuff/gitops-beta/desired/beta-tls.staging.desired.json \
  "${mgmt_dir}/mgmt" \
  "$gitops_dir" \
  /var/lib/fishystuff/gitops-beta/secrets/cloudflare-api-token \
  -1 >/dev/null 2>"${root}/unit.stderr"

read -r desired_sha256 _ < <(sha256sum "$desired")
read -r unit_sha256 _ < <(sha256sum "$unit")
read -r token_sha256 _ < <(sha256sum "$token")

env PATH="${fake_bin}:${PATH}" \
  bash scripts/recipes/gitops-beta-tls-resident-install-packet.sh \
    "${root}/missing.desired.json" \
    "${root}/missing.service" \
    "" >"${root}/missing.stdout"
grep -F "beta_tls_resident_install_packet_status=pending_inputs" "${root}/missing.stdout" >/dev/null
grep -F "beta_tls_resident_install_packet_desired_state_status=missing" "${root}/missing.stdout" >/dev/null
grep -F "beta_tls_resident_install_packet_unit_file_status=missing" "${root}/missing.stdout" >/dev/null
grep -F "beta_tls_resident_install_packet_cloudflare_token_status=missing_source" "${root}/missing.stdout" >/dev/null
grep -F "local_host_mutation_performed=false" "${root}/missing.stdout" >/dev/null
pass "pending missing inputs packet"

env PATH="${fake_bin}:${PATH}" \
  bash scripts/recipes/gitops-beta-tls-resident-install-packet.sh \
    "$desired" \
    "$unit" \
    "$token" >"${root}/ready.stdout"
grep -F "beta_tls_resident_install_packet_status=ready" "${root}/ready.stdout" >/dev/null
grep -F "beta_tls_resident_install_packet_desired_sha256=${desired_sha256}" "${root}/ready.stdout" >/dev/null
grep -F "beta_tls_resident_install_packet_unit_sha256=${unit_sha256}" "${root}/ready.stdout" >/dev/null
grep -F "beta_tls_resident_install_packet_cloudflare_token_sha256=${token_sha256}" "${root}/ready.stdout" >/dev/null
grep -F "FISHYSTUFF_GITOPS_ENABLE_BETA_TLS_RESIDENT_INSTALL=1 FISHYSTUFF_GITOPS_ENABLE_BETA_TLS_RESIDENT_RESTART=1" "${root}/ready.stdout" >/dev/null
grep -F "FISHYSTUFF_GITOPS_BETA_TLS_DESIRED_SHA256=${desired_sha256}" "${root}/ready.stdout" >/dev/null
grep -F "FISHYSTUFF_GITOPS_BETA_TLS_RESIDENT_UNIT_SHA256=${unit_sha256}" "${root}/ready.stdout" >/dev/null
grep -F "FISHYSTUFF_GITOPS_BETA_TLS_CLOUDFLARE_TOKEN_SHA256=${token_sha256}" "${root}/ready.stdout" >/dev/null
grep -F "just gitops-beta-install-tls-resident desired_state=${desired} unit_file=${unit} cloudflare_token_source=${token}" "${root}/ready.stdout" >/dev/null
if grep -F "fake-cloudflare-token" "${root}/ready.stdout" >/dev/null; then
  printf '[gitops-beta-tls-resident-install-packet-test] packet leaked token value\n' >&2
  exit 1
fi
pass "ready install packet"

env PATH="${fake_bin}:${PATH}" FISHYSTUFF_FAKE_HOSTNAME=operator-dev \
  bash scripts/recipes/gitops-beta-tls-resident-install-packet.sh \
    "$desired" \
    "$unit" \
    "$token" >"${root}/wrong-host.stdout"
grep -F "beta_tls_resident_install_packet_status=blocked_host" "${root}/wrong-host.stdout" >/dev/null
grep -F "beta_tls_resident_install_packet_hostname_match=false" "${root}/wrong-host.stdout" >/dev/null
pass "blocked wrong host packet"

env PATH="${fake_bin}:${PATH}" FISHYSTUFF_OPERATOR_SECRETSPEC_PROFILE=production-deploy \
  bash scripts/recipes/gitops-beta-tls-resident-install-packet.sh \
    "$desired" \
    "$unit" \
    "$token" >"${root}/production-profile.stdout"
grep -F "beta_tls_resident_install_packet_status=blocked_profile" "${root}/production-profile.stdout" >/dev/null
grep -F "beta_tls_resident_install_packet_next_required_action=load_beta_deploy_or_no_operator_profile" "${root}/production-profile.stdout" >/dev/null
pass "blocked production profile packet"

bad_unit="${root}/bad.service"
cp "$unit" "$bad_unit"
printf '\nEnvironmentFile=/var/lib/fishystuff/gitops-beta/secrets/acme.env\n' >>"$bad_unit"
expect_fail_contains \
  "reject dotenv-style unit" \
  "beta TLS resident unit must use LoadCredential, not EnvironmentFile" \
  env PATH="${fake_bin}:${PATH}" \
    bash scripts/recipes/gitops-beta-tls-resident-install-packet.sh "$desired" "$bad_unit" "$token"

printf '[gitops-beta-tls-resident-install-packet-test] %s checks passed\n' "$pass_count"
