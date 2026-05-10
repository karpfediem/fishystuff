#!/usr/bin/env bash
set -euo pipefail

SCRIPT_PATH="$(readlink -f "${BASH_SOURCE[0]}")"
SCRIPT_DIR="$(cd "$(dirname "$SCRIPT_PATH")" && pwd)"
source "${SCRIPT_DIR}/lib/common.sh"

cd "$RECIPE_REPO_ROOT"

pass_count=0

pass() {
  printf '[gitops-beta-tls-resident-status-packet-test] pass: %s\n' "$1"
  pass_count="$((pass_count + 1))"
}

make_beta_cert() {
  local root="$1"

  openssl req \
    -x509 \
    -newkey rsa:2048 \
    -nodes \
    -keyout "${root}/privkey.pem" \
    -out "${root}/fullchain.pem" \
    -days 30 \
    -subj "/CN=api.beta.fishystuff.fish" \
    -addext "subjectAltName=DNS:api.beta.fishystuff.fish,DNS:beta.fishystuff.fish,DNS:cdn.beta.fishystuff.fish,DNS:telemetry.beta.fishystuff.fish" \
    >"${root}/openssl.log" 2>&1
}

root="$(mktemp -d)"
fake_bin="${root}/bin"
cert_root="${root}/cert"
desired="${root}/beta-tls.staging.desired.json"
unit="${root}/fishystuff-beta-tls-reconciler.service"
token="${root}/cloudflare-api-token"
mkdir -p "$fake_bin" "$cert_root"

cat >"${fake_bin}/hostname" <<'EOF'
#!/usr/bin/env bash
if [[ "${1-}" == "-f" ]]; then
  printf '%s\n' "${FISHYSTUFF_FAKE_HOSTNAME:-site-nbg1-beta}"
else
  printf '%s\n' "${FISHYSTUFF_FAKE_HOSTNAME:-site-nbg1-beta}"
fi
EOF
chmod +x "${fake_bin}/hostname"

cat >"${fake_bin}/systemctl" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
if [[ "${1-}" != "show" ]]; then
  exit 1
fi
property="${4-}"
case "$property" in
  LoadState) printf '%s\n' "${FISHYSTUFF_FAKE_UNIT_LOAD_STATE:-loaded}" ;;
  ActiveState) printf '%s\n' "${FISHYSTUFF_FAKE_UNIT_ACTIVE_STATE:-active}" ;;
  SubState) printf '%s\n' "${FISHYSTUFF_FAKE_UNIT_SUB_STATE:-running}" ;;
  UnitFileState) printf '%s\n' "${FISHYSTUFF_FAKE_UNIT_FILE_STATE:-enabled}" ;;
  MainPID) printf '%s\n' "${FISHYSTUFF_FAKE_UNIT_MAIN_PID:-1234}" ;;
  *) printf 'unknown\n' ;;
esac
EOF
chmod +x "${fake_bin}/systemctl"

printf 'fake-cloudflare-token\n' >"$token"
chmod 600 "$token"
make_beta_cert "$cert_root"

env FISHYSTUFF_GITOPS_BETA_ACME_CONTACT_EMAIL=ops@fishystuff.invalid \
  bash scripts/recipes/gitops-beta-tls-desired.sh "$desired" staging "" >/dev/null 2>"${root}/desired.stderr"
cat >"$unit" <<EOF
[Unit]
Description=FishyStuff beta GitOps TLS ACME reconciler

[Service]
Type=simple
Environment=FISHYSTUFF_GITOPS_STATE_FILE=${desired}
LoadCredential=cloudflare-api-token:${token}
ExecStart=/bin/sh -ceu 'export CLOUDFLARE_API_TOKEN="\$(cat "\$CREDENTIALS_DIRECTORY/cloudflare-api-token")"; exec /nix/store/example-mgmt/bin/mgmt run main.mcl'

[Install]
WantedBy=multi-user.target
EOF

read -r token_sha256 _ < <(sha256sum "$token")

env PATH="${fake_bin}:${PATH}" \
  bash scripts/recipes/gitops-beta-tls-resident-status-packet.sh \
    "$desired" \
    "$unit" \
    "$token" \
    "${cert_root}/fullchain.pem" \
    "${cert_root}/privkey.pem" \
    systemctl \
    openssl >"${root}/status.stdout"

grep -F "gitops_beta_tls_resident_status_packet_ok=true" "${root}/status.stdout" >/dev/null
grep -F "beta_tls_resident_status=active_with_tls_material" "${root}/status.stdout" >/dev/null
grep -F "beta_tls_resident_hostname_match=true" "${root}/status.stdout" >/dev/null
grep -F "beta_tls_resident_unit_load_state=loaded" "${root}/status.stdout" >/dev/null
grep -F "beta_tls_resident_unit_active_state=active" "${root}/status.stdout" >/dev/null
grep -F "beta_tls_resident_cloudflare_token_exists=true" "${root}/status.stdout" >/dev/null
grep -F "beta_tls_resident_cloudflare_token_mode=600" "${root}/status.stdout" >/dev/null
grep -F "beta_tls_resident_cloudflare_token_sha256=${token_sha256}" "${root}/status.stdout" >/dev/null
grep -F "beta_tls_resident_unit_has_environment_file=false" "${root}/status.stdout" >/dev/null
grep -F "beta_tls_resident_unit_has_expected_token_credential=true" "${root}/status.stdout" >/dev/null
grep -F "beta_tls_resident_unit_has_expected_state_file=true" "${root}/status.stdout" >/dev/null
grep -F "beta_tls_resident_unit_contains_non_beta_domain=false" "${root}/status.stdout" >/dev/null
grep -F "beta_tls_resident_fullchain_parse_ok=true" "${root}/status.stdout" >/dev/null
grep -F "beta_tls_resident_fullchain_valid_more_than_7d=true" "${root}/status.stdout" >/dev/null
grep -F "beta_tls_resident_fullchain_san_beta_fishystuff_fish=true" "${root}/status.stdout" >/dev/null
grep -F "beta_tls_resident_fullchain_san_api_beta_fishystuff_fish=true" "${root}/status.stdout" >/dev/null
grep -F "beta_tls_resident_fullchain_san_cdn_beta_fishystuff_fish=true" "${root}/status.stdout" >/dev/null
grep -F "beta_tls_resident_fullchain_san_telemetry_beta_fishystuff_fish=true" "${root}/status.stdout" >/dev/null
grep -F "beta_tls_resident_privkey_parse_ok=true" "${root}/status.stdout" >/dev/null
grep -F "beta_tls_resident_cert_key_match=true" "${root}/status.stdout" >/dev/null
grep -F "local_host_mutation_performed=false" "${root}/status.stdout" >/dev/null
grep -F "remote_deploy_performed=false" "${root}/status.stdout" >/dev/null
if grep -F "fake-cloudflare-token" "${root}/status.stdout" >/dev/null; then
  printf '[gitops-beta-tls-resident-status-packet-test] status packet leaked token value\n' >&2
  exit 1
fi
pass "reports active resident TLS status without leaking token"

env PATH="${fake_bin}:${PATH}" \
  bash scripts/recipes/gitops-beta-tls-resident-status-packet.sh \
    "$desired" \
    "$unit" \
    "${root}/missing-cloudflare-api-token" \
    "${cert_root}/fullchain.pem" \
    "${cert_root}/privkey.pem" \
    systemctl \
    openssl >"${root}/missing-token.stdout"
grep -F "beta_tls_resident_status=pending_install" "${root}/missing-token.stdout" >/dev/null
grep -F "beta_tls_resident_cloudflare_token_exists=false" "${root}/missing-token.stdout" >/dev/null
pass "reports missing resident token as pending install"

env PATH="${fake_bin}:${PATH}" FISHYSTUFF_FAKE_HOSTNAME=operator-dev \
  bash scripts/recipes/gitops-beta-tls-resident-status-packet.sh \
    "$desired" \
    "$unit" \
    "$token" \
    "${cert_root}/fullchain.pem" \
    "${cert_root}/privkey.pem" \
    systemctl \
    openssl >"${root}/wrong-host.stdout"
grep -F "beta_tls_resident_status=wrong_host" "${root}/wrong-host.stdout" >/dev/null
grep -F "beta_tls_resident_hostname_match=false" "${root}/wrong-host.stdout" >/dev/null
pass "reports wrong host context"

env PATH="${fake_bin}:${PATH}" FISHYSTUFF_FAKE_UNIT_ACTIVE_STATE=failed \
  bash scripts/recipes/gitops-beta-tls-resident-status-packet.sh \
    "$desired" \
    "$unit" \
    "$token" \
    "${cert_root}/fullchain.pem" \
    "${cert_root}/privkey.pem" \
    systemctl \
    openssl >"${root}/inactive.stdout"
grep -F "beta_tls_resident_status=unit_not_active" "${root}/inactive.stdout" >/dev/null
grep -F "beta_tls_resident_unit_active_state=failed" "${root}/inactive.stdout" >/dev/null
pass "reports inactive resident unit"

printf '[gitops-beta-tls-resident-status-packet-test] %s checks passed\n' "$pass_count"
