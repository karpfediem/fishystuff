#!/usr/bin/env bash
set -euo pipefail

SCRIPT_PATH="$(readlink -f "${BASH_SOURCE[0]}")"
SCRIPT_DIR="$(cd "$(dirname "$SCRIPT_PATH")" && pwd)"
source "${SCRIPT_DIR}/lib/common.sh"

cd "$RECIPE_REPO_ROOT"

pass_count=0

pass() {
  printf '[gitops-beta-remote-install-tls-resident-test] pass: %s\n' "$1"
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
    printf '[gitops-beta-remote-install-tls-resident-test] expected failure: %s\n' "$name" >&2
    exit 1
  fi
  if ! grep -F "$expected" "$stderr" >/dev/null; then
    printf '[gitops-beta-remote-install-tls-resident-test] expected %s stderr to contain %q\n' "$name" "$expected" >&2
    cat "$stderr" >&2
    exit 1
  fi
  pass "$name"
}

root="$(mktemp -d)"
fake_ssh="${root}/ssh"
fake_scp="${root}/scp"
mgmt_dir="${root}/mgmt/bin"
gitops_dir="${root}/gitops"
desired="${root}/beta-tls.staging.desired.json"
unit="${root}/fishystuff-beta-tls-reconciler.service"
mkdir -p "$mgmt_dir" "$gitops_dir"

cat >"${mgmt_dir}/mgmt" <<'EOF'
#!/usr/bin/env bash
printf 'fake mgmt\n'
EOF
chmod +x "${mgmt_dir}/mgmt"
printf '# fake main\n' >"${gitops_dir}/main.mcl"

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
token_sha256="$(printf '%s\n' "fake-cloudflare-token" | sha256sum | awk '{ print $1 }')"

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
printf 'remote_hostname=site-nbg1-beta\n'
printf 'remote_tls_resident_install_ok=fishystuff-beta-tls-reconciler.service\n'
printf 'remote_tls_resident_desired_target=/var/lib/fishystuff/gitops-beta/desired/beta-tls.staging.desired.json\n'
printf 'remote_tls_resident_unit_target=/etc/systemd/system/fishystuff-beta-tls-reconciler.service\n'
printf 'remote_tls_resident_cloudflare_token_target=/var/lib/fishystuff/gitops-beta/secrets/cloudflare-api-token\n'
printf 'remote_host_mutation_performed=true\n'
printf 'remote_deploy_performed=false\n'
printf 'infrastructure_mutation_performed=false\n'
printf 'local_host_mutation_performed=false\n'
SSH
chmod +x "$fake_ssh"

env \
  FISHYSTUFF_OPERATOR_SECRETSPEC_PROFILE=beta-deploy \
  FISHYSTUFF_GITOPS_ENABLE_BETA_REMOTE_TLS_RESIDENT_INSTALL=1 \
  FISHYSTUFF_GITOPS_ENABLE_BETA_REMOTE_TLS_RESIDENT_RESTART=1 \
  FISHYSTUFF_GITOPS_BETA_REMOTE_TLS_RESIDENT_TARGET=root@203.0.113.20 \
  FISHYSTUFF_GITOPS_BETA_TLS_DESIRED_SHA256="$desired_sha256" \
  FISHYSTUFF_GITOPS_BETA_TLS_RESIDENT_UNIT_SHA256="$unit_sha256" \
  FISHYSTUFF_GITOPS_BETA_TLS_CLOUDFLARE_TOKEN_SHA256="$token_sha256" \
  CLOUDFLARE_API_TOKEN=fake-cloudflare-token \
  HETZNER_SSH_PRIVATE_KEY='fixture-private-key' \
  FISHYSTUFF_FAKE_SCP_LOG="${root}/scp.log" \
  FISHYSTUFF_FAKE_REMOTE_LOG="${root}/remote.log" \
  FISHYSTUFF_FAKE_REMOTE_STDIN="${root}/remote.sh" \
  bash scripts/recipes/gitops-beta-remote-install-tls-resident.sh \
    root@203.0.113.20 \
    site-nbg1-beta \
    "$desired" \
    "$unit" \
    env:CLOUDFLARE_API_TOKEN \
    "$fake_ssh" \
    "$fake_scp" >"${root}/remote-install.out"

grep -F "gitops_beta_remote_install_tls_resident_checked=true" "${root}/remote-install.out" >/dev/null
grep -F "gitops_beta_remote_install_tls_resident_ok=true" "${root}/remote-install.out" >/dev/null
grep -F "remote_tls_resident_install_ok=fishystuff-beta-tls-reconciler.service" "${root}/remote-install.out" >/dev/null
grep -F "remote_host_mutation_performed=true" "${root}/remote-install.out" >/dev/null
grep -F "root@203.0.113.20:/tmp/fishystuff-beta-tls-resident-desired.json" "${root}/scp.log" >/dev/null
grep -F "root@203.0.113.20:/tmp/fishystuff-beta-tls-resident.service" "${root}/scp.log" >/dev/null
grep -F "root@203.0.113.20:/tmp/fishystuff-beta-tls-resident-cloudflare-api-token" "${root}/scp.log" >/dev/null
grep -F "systemctl enable --now \"\$unit_name\"" "${root}/remote.sh" >/dev/null
grep -F "LoadCredential=cloudflare-api-token:\${cloudflare_token_target}" "${root}/remote.sh" >/dev/null
if grep -F "fake-cloudflare-token" "${root}/remote-install.out" "${root}/scp.log" "${root}/remote.log" "${root}/remote.sh" >/dev/null; then
  printf '[gitops-beta-remote-install-tls-resident-test] remote install path leaked token value\n' >&2
  exit 1
fi
pass "remote TLS resident install validates and emits guarded remote script"

base_env=(
  FISHYSTUFF_OPERATOR_SECRETSPEC_PROFILE=beta-deploy
  FISHYSTUFF_GITOPS_ENABLE_BETA_REMOTE_TLS_RESIDENT_INSTALL=1
  FISHYSTUFF_GITOPS_ENABLE_BETA_REMOTE_TLS_RESIDENT_RESTART=1
  FISHYSTUFF_GITOPS_BETA_REMOTE_TLS_RESIDENT_TARGET=root@203.0.113.20
  FISHYSTUFF_GITOPS_BETA_TLS_DESIRED_SHA256="$desired_sha256"
  FISHYSTUFF_GITOPS_BETA_TLS_RESIDENT_UNIT_SHA256="$unit_sha256"
  FISHYSTUFF_GITOPS_BETA_TLS_CLOUDFLARE_TOKEN_SHA256="$token_sha256"
  CLOUDFLARE_API_TOKEN=fake-cloudflare-token
  HETZNER_SSH_PRIVATE_KEY=fixture-private-key
  FISHYSTUFF_FAKE_SCP_LOG="${root}/scp-fail.log"
  FISHYSTUFF_FAKE_REMOTE_LOG="${root}/remote-fail.log"
  FISHYSTUFF_FAKE_REMOTE_STDIN="${root}/remote-fail.sh"
)

expect_fail_contains \
  "requires install opt-in" \
  "gitops-beta-remote-install-tls-resident requires FISHYSTUFF_GITOPS_ENABLE_BETA_REMOTE_TLS_RESIDENT_INSTALL=1" \
  env \
    FISHYSTUFF_OPERATOR_SECRETSPEC_PROFILE=beta-deploy \
    HETZNER_SSH_PRIVATE_KEY=fixture-private-key \
    bash scripts/recipes/gitops-beta-remote-install-tls-resident.sh root@203.0.113.20 site-nbg1-beta "$desired" "$unit" env:CLOUDFLARE_API_TOKEN "$fake_ssh" "$fake_scp"

expect_fail_contains \
  "refuses production profile" \
  "gitops-beta-remote-install-tls-resident must not run with production SecretSpec profile active" \
  env \
    "${base_env[@]}" \
    FISHYSTUFF_OPERATOR_SECRETSPEC_PROFILE=production-deploy \
    bash scripts/recipes/gitops-beta-remote-install-tls-resident.sh root@203.0.113.20 site-nbg1-beta "$desired" "$unit" env:CLOUDFLARE_API_TOKEN "$fake_ssh" "$fake_scp"

expect_fail_contains \
  "refuses previous beta host" \
  "target points at the previous beta host" \
  env \
    "${base_env[@]}" \
    FISHYSTUFF_GITOPS_BETA_REMOTE_TLS_RESIDENT_TARGET=root@178.104.230.121 \
    bash scripts/recipes/gitops-beta-remote-install-tls-resident.sh root@178.104.230.121 site-nbg1-beta "$desired" "$unit" env:CLOUDFLARE_API_TOKEN "$fake_ssh" "$fake_scp"

expect_fail_contains \
  "refuses stale token hash" \
  "cloudflare_token_source sha256 mismatch" \
  env \
    "${base_env[@]}" \
    FISHYSTUFF_GITOPS_BETA_TLS_CLOUDFLARE_TOKEN_SHA256=0000000000000000000000000000000000000000000000000000000000000000 \
    bash scripts/recipes/gitops-beta-remote-install-tls-resident.sh root@203.0.113.20 site-nbg1-beta "$desired" "$unit" env:CLOUDFLARE_API_TOKEN "$fake_ssh" "$fake_scp"

printf '[gitops-beta-remote-install-tls-resident-test] %s checks passed\n' "$pass_count"
