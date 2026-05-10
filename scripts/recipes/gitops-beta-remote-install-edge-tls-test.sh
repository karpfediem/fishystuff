#!/usr/bin/env bash
set -euo pipefail

SCRIPT_PATH="$(readlink -f "${BASH_SOURCE[0]}")"
SCRIPT_DIR="$(cd "$(dirname "$SCRIPT_PATH")" && pwd)"
source "${SCRIPT_DIR}/lib/common.sh"

cd "$RECIPE_REPO_ROOT"

pass_count=0

pass() {
  printf '[gitops-beta-remote-install-edge-tls-test] pass: %s\n' "$1"
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
    printf '[gitops-beta-remote-install-edge-tls-test] expected failure: %s\n' "$name" >&2
    exit 1
  fi
  if ! grep -F "$expected" "$stderr" >/dev/null; then
    printf '[gitops-beta-remote-install-edge-tls-test] expected %s stderr to contain %q\n' "$name" "$expected" >&2
    cat "$stderr" >&2
    exit 1
  fi
  pass "$name"
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
cert_root="${root}/cert"
fake_ssh="${root}/ssh"
fake_scp="${root}/scp"
mkdir -p "$cert_root"
make_beta_cert "$cert_root"
fullchain="${cert_root}/fullchain.pem"
privkey="${cert_root}/privkey.pem"
read -r fullchain_sha256 _ < <(sha256sum "$fullchain")
read -r privkey_sha256 _ < <(sha256sum "$privkey")

cat >"$fake_scp" <<'SCP'
#!/usr/bin/env bash
set -euo pipefail
printf '%s\n' "$*" >>"${FISHYSTUFF_FAKE_SCP_LOG:?}"
SCP
chmod +x "$fake_scp"

cat >"$fake_ssh" <<'SSH'
#!/usr/bin/env bash
set -euo pipefail
printf '%s\n' "$*" >>"${FISHYSTUFF_FAKE_REMOTE_LOG:?}"
cat >"${FISHYSTUFF_FAKE_REMOTE_STDIN:?}"
printf 'edge_site_trusted_ready=true\n'
printf 'edge_api_meta_trusted_ready=true\n'
printf 'edge_cdn_runtime_trusted_ready=true\n'
printf 'remote_hostname=site-nbg1-beta\n'
printf 'remote_edge_tls_install_ok=true\n'
printf 'remote_edge_service_restart_ok=fishystuff-beta-edge.service\n'
printf 'remote_host_mutation_performed=true\n'
printf 'remote_deploy_performed=false\n'
printf 'infrastructure_mutation_performed=false\n'
printf 'local_host_mutation_performed=false\n'
SSH
chmod +x "$fake_ssh"

env \
  FISHYSTUFF_OPERATOR_SECRETSPEC_PROFILE=beta-deploy \
  FISHYSTUFF_GITOPS_ENABLE_BETA_REMOTE_EDGE_TLS_INSTALL=1 \
  FISHYSTUFF_GITOPS_ENABLE_BETA_REMOTE_EDGE_RESTART=1 \
  FISHYSTUFF_GITOPS_BETA_REMOTE_EDGE_TLS_TARGET=root@203.0.113.20 \
  FISHYSTUFF_GITOPS_BETA_EDGE_TLS_FULLCHAIN_SHA256="$fullchain_sha256" \
  FISHYSTUFF_GITOPS_BETA_EDGE_TLS_PRIVKEY_SHA256="$privkey_sha256" \
  HETZNER_SSH_PRIVATE_KEY='fixture-private-key' \
  FISHYSTUFF_FAKE_SCP_LOG="${root}/scp.log" \
  FISHYSTUFF_FAKE_REMOTE_LOG="${root}/remote.log" \
  FISHYSTUFF_FAKE_REMOTE_STDIN="${root}/remote.sh" \
  bash scripts/recipes/gitops-beta-remote-install-edge-tls.sh root@203.0.113.20 site-nbg1-beta "$fullchain" "$privkey" "$fake_ssh" "$fake_scp" >"${root}/tls.out"
grep -F "gitops_beta_remote_install_edge_tls_checked=true" "${root}/tls.out" >/dev/null
grep -F "gitops_beta_remote_install_edge_tls_ok=true" "${root}/tls.out" >/dev/null
grep -F "tls_mode=operator_supplied" "${root}/tls.out" >/dev/null
grep -F "remote_edge_tls_install_ok=true" "${root}/tls.out" >/dev/null
grep -F "root@203.0.113.20:/tmp/fishystuff-beta-edge-operator-fullchain.pem" "${root}/scp.log" >/dev/null
grep -F "root@203.0.113.20:/tmp/fishystuff-beta-edge-operator-privkey.pem" "${root}/scp.log" >/dev/null
grep -F "curl -fsS --max-time 3 --resolve" "${root}/remote.sh" >/dev/null
grep -F "systemctl restart fishystuff-beta-edge.service" "${root}/remote.sh" >/dev/null
pass "remote edge TLS install validates and restarts with trusted checks"

base_env=(
  FISHYSTUFF_OPERATOR_SECRETSPEC_PROFILE=beta-deploy
  FISHYSTUFF_GITOPS_ENABLE_BETA_REMOTE_EDGE_TLS_INSTALL=1
  FISHYSTUFF_GITOPS_ENABLE_BETA_REMOTE_EDGE_RESTART=1
  FISHYSTUFF_GITOPS_BETA_REMOTE_EDGE_TLS_TARGET=root@203.0.113.20
  FISHYSTUFF_GITOPS_BETA_EDGE_TLS_FULLCHAIN_SHA256="$fullchain_sha256"
  FISHYSTUFF_GITOPS_BETA_EDGE_TLS_PRIVKEY_SHA256="$privkey_sha256"
  HETZNER_SSH_PRIVATE_KEY=fixture-private-key
  FISHYSTUFF_FAKE_SCP_LOG="${root}/scp-fail.log"
  FISHYSTUFF_FAKE_REMOTE_LOG="${root}/remote-fail.log"
  FISHYSTUFF_FAKE_REMOTE_STDIN="${root}/remote-fail.sh"
)

expect_fail_contains \
  "requires install opt-in" \
  "gitops-beta-remote-install-edge-tls requires FISHYSTUFF_GITOPS_ENABLE_BETA_REMOTE_EDGE_TLS_INSTALL=1" \
  env \
    FISHYSTUFF_OPERATOR_SECRETSPEC_PROFILE=beta-deploy \
    HETZNER_SSH_PRIVATE_KEY=fixture-private-key \
    bash scripts/recipes/gitops-beta-remote-install-edge-tls.sh root@203.0.113.20 site-nbg1-beta "$fullchain" "$privkey" "$fake_ssh" "$fake_scp"

expect_fail_contains \
  "requires target acknowledgement" \
  "gitops-beta-remote-install-edge-tls requires FISHYSTUFF_GITOPS_BETA_REMOTE_EDGE_TLS_TARGET=root@203.0.113.20" \
  env \
    "${base_env[@]}" \
    FISHYSTUFF_GITOPS_BETA_REMOTE_EDGE_TLS_TARGET=root@203.0.113.21 \
    bash scripts/recipes/gitops-beta-remote-install-edge-tls.sh root@203.0.113.20 site-nbg1-beta "$fullchain" "$privkey" "$fake_ssh" "$fake_scp"

expect_fail_contains \
  "rejects production profile" \
  "must not run with production SecretSpec profile active" \
  env \
    "${base_env[@]}" \
    FISHYSTUFF_OPERATOR_SECRETSPEC_PROFILE=production-deploy \
    bash scripts/recipes/gitops-beta-remote-install-edge-tls.sh root@203.0.113.20 site-nbg1-beta "$fullchain" "$privkey" "$fake_ssh" "$fake_scp"

expect_fail_contains \
  "rejects dns target" \
  "target host must be an IPv4 address" \
  env \
    "${base_env[@]}" \
    FISHYSTUFF_GITOPS_BETA_REMOTE_EDGE_TLS_TARGET=root@beta.fishystuff.fish \
    bash scripts/recipes/gitops-beta-remote-install-edge-tls.sh root@beta.fishystuff.fish site-nbg1-beta "$fullchain" "$privkey" "$fake_ssh" "$fake_scp"

expect_fail_contains \
  "rejects stale fullchain hash" \
  "FISHYSTUFF_GITOPS_BETA_EDGE_TLS_FULLCHAIN_SHA256 does not match checked beta edge fullchain" \
  env \
    "${base_env[@]}" \
    FISHYSTUFF_GITOPS_BETA_EDGE_TLS_FULLCHAIN_SHA256=wrong \
    bash scripts/recipes/gitops-beta-remote-install-edge-tls.sh root@203.0.113.20 site-nbg1-beta "$fullchain" "$privkey" "$fake_ssh" "$fake_scp"

printf '[gitops-beta-remote-install-edge-tls-test] %s checks passed\n' "$pass_count"
