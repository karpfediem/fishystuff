#!/usr/bin/env bash
set -euo pipefail

SCRIPT_PATH="$(readlink -f "${BASH_SOURCE[0]}")"
SCRIPT_DIR="$(cd "$(dirname "$SCRIPT_PATH")" && pwd)"
source "${SCRIPT_DIR}/lib/common.sh"

cd "$RECIPE_REPO_ROOT"

pass_count=0

pass() {
  printf '[gitops-beta-remote-tls-resident-status-packet-test] pass: %s\n' "$1"
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
    printf '[gitops-beta-remote-tls-resident-status-packet-test] expected failure: %s\n' "$name" >&2
    exit 1
  fi
  if ! grep -F "$expected" "$stderr" >/dev/null; then
    printf '[gitops-beta-remote-tls-resident-status-packet-test] expected %s stderr to contain %q\n' "$name" "$expected" >&2
    cat "$stderr" >&2
    exit 1
  fi
  pass "$name"
}

root="$(mktemp -d)"
fake_ssh="${root}/ssh"

cat >"$fake_ssh" <<'SSH'
#!/usr/bin/env bash
set -euo pipefail
printf '%s\n' "$*" >>"${FISHYSTUFF_FAKE_REMOTE_LOG:?}"
cat >"${FISHYSTUFF_FAKE_REMOTE_STDIN:?}"
printf 'remote_hostname_match=true\n'
printf 'remote_hostname=site-nbg1-beta\n'
printf 'remote_tls_resident_unit_load_state=loaded\n'
printf 'remote_tls_resident_unit_active_state=active\n'
printf 'remote_tls_resident_unit_sub_state=running\n'
printf 'remote_tls_resident_unit_file_state=enabled\n'
printf 'remote_tls_resident_unit_result=success\n'
printf 'remote_tls_resident_unit_exec_main_status=0\n'
printf 'remote_tls_resident_unit_n_restarts=0\n'
printf 'remote_tls_resident_desired_state_exists=true\n'
printf 'remote_tls_resident_cloudflare_token_exists=true\n'
printf 'remote_tls_resident_cloudflare_token_mode=600\n'
printf 'remote_tls_resident_fullchain_parse_ok=true\n'
printf 'remote_tls_resident_fullchain_san_beta_fishystuff_fish=true\n'
printf 'remote_tls_resident_fullchain_san_api_beta_fishystuff_fish=true\n'
printf 'remote_tls_resident_fullchain_san_cdn_beta_fishystuff_fish=true\n'
printf 'remote_tls_resident_fullchain_san_telemetry_beta_fishystuff_fish=true\n'
printf 'remote_tls_resident_cert_key_match=true\n'
printf 'remote_host_mutation_performed=false\n'
printf 'remote_deploy_performed=false\n'
printf 'infrastructure_mutation_performed=false\n'
printf 'local_host_mutation_performed=false\n'
SSH
chmod +x "$fake_ssh"

env \
  FISHYSTUFF_OPERATOR_SECRETSPEC_PROFILE=beta-deploy \
  HETZNER_SSH_PRIVATE_KEY=fixture-private-key \
  FISHYSTUFF_FAKE_REMOTE_LOG="${root}/remote.log" \
  FISHYSTUFF_FAKE_REMOTE_STDIN="${root}/remote.sh" \
  bash scripts/recipes/gitops-beta-remote-tls-resident-status-packet.sh \
    root@203.0.113.20 \
    site-nbg1-beta \
    "$fake_ssh" >"${root}/status.out"
grep -F "gitops_beta_remote_tls_resident_status_packet_ok=true" "${root}/status.out" >/dev/null
grep -F "resident_target=root@203.0.113.20" "${root}/status.out" >/dev/null
grep -F "remote_tls_resident_unit_active_state=active" "${root}/status.out" >/dev/null
grep -F "remote_tls_resident_unit_result=success" "${root}/status.out" >/dev/null
grep -F "remote_tls_resident_unit_exec_main_status=0" "${root}/status.out" >/dev/null
grep -F "remote_tls_resident_unit_n_restarts=0" "${root}/status.out" >/dev/null
grep -F "remote_tls_resident_fullchain_parse_ok=true" "${root}/status.out" >/dev/null
grep -F "remote_tls_resident_cert_key_match=true" "${root}/status.out" >/dev/null
grep -F "remote_host_mutation_performed=false" "${root}/status.out" >/dev/null
grep -F "remote_deploy_performed=false" "${root}/status.out" >/dev/null
grep -F "BatchMode=yes" "${root}/remote.log" >/dev/null
grep -F "ConnectTimeout=120" "${root}/remote.log" >/dev/null
grep -F "ConnectionAttempts=1" "${root}/remote.log" >/dev/null
grep -F "ServerAliveInterval=10" "${root}/remote.log" >/dev/null
grep -F "ServerAliveCountMax=3" "${root}/remote.log" >/dev/null
grep -F "systemctl show \"\$unit_name\" -p ActiveState --value" "${root}/remote.sh" >/dev/null
grep -F "systemctl show \"\$unit_name\" -p ExecMainStatus --value" "${root}/remote.sh" >/dev/null
grep -F "systemctl show \"\$unit_name\" -p NRestarts --value" "${root}/remote.sh" >/dev/null
grep -F "openssl x509 -checkend 604800" "${root}/remote.sh" >/dev/null
pass "remote TLS resident status packet is read-only"

expect_fail_contains \
  "requires beta profile" \
  "gitops-beta-remote-tls-resident-status-packet requires FISHYSTUFF_OPERATOR_SECRETSPEC_PROFILE=beta-deploy" \
  env HETZNER_SSH_PRIVATE_KEY=fixture-private-key \
    bash scripts/recipes/gitops-beta-remote-tls-resident-status-packet.sh root@203.0.113.20 site-nbg1-beta "$fake_ssh"

expect_fail_contains \
  "refuses production profile" \
  "gitops-beta-remote-tls-resident-status-packet must not run with production SecretSpec profile active" \
  env FISHYSTUFF_OPERATOR_SECRETSPEC_PROFILE=production-deploy HETZNER_SSH_PRIVATE_KEY=fixture-private-key \
    bash scripts/recipes/gitops-beta-remote-tls-resident-status-packet.sh root@203.0.113.20 site-nbg1-beta "$fake_ssh"

printf '[gitops-beta-remote-tls-resident-status-packet-test] %s checks passed\n' "$pass_count"
