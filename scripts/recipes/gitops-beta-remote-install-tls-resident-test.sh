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
fake_push="${root}/push-closure.sh"
desired="${root}/beta-tls.desired.json"
unit="${root}/fishystuff-beta-tls-reconciler.service"
store_root="$(readlink -f /bin/sh | awk -F/ '{ print "/" $2 "/" $3 "/" $4 }')"

env FISHYSTUFF_GITOPS_BETA_ACME_CONTACT_EMAIL=ops@fishystuff.invalid \
  bash scripts/recipes/gitops-beta-tls-desired.sh "$desired" staging "" >/dev/null 2>"${root}/desired.stderr"
cat >"$unit" <<EOF
[Unit]
Description=FishyStuff beta GitOps TLS ACME reconciler

[Service]
Type=simple
WorkingDirectory=${store_root}
Environment=FISHYSTUFF_GITOPS_ENABLE_LOCAL_APPLY=1
Environment=FISHYSTUFF_GITOPS_STATE_FILE=/var/lib/fishystuff/gitops-beta/desired/beta-tls.desired.json
LoadCredential=cloudflare-api-token:/var/lib/fishystuff/gitops-beta/secrets/cloudflare-api-token
ExecStart=/bin/sh -ceu 'export CLOUDFLARE_API_TOKEN="\$(cat "\$CREDENTIALS_DIRECTORY/cloudflare-api-token")"; exec ${store_root}/bin/mgmt run --tmp-prefix --no-pgp lang --converged-timeout -1 main.mcl'
ReadWritePaths=/var/lib/fishystuff/gitops-beta

[Install]
WantedBy=multi-user.target
EOF

read -r desired_sha256 _ < <(sha256sum "$desired")
read -r unit_sha256 _ < <(sha256sum "$unit")
token_sha256="$(printf '%s\n' "fake-cloudflare-token" | sha256sum | awk '{ print $1 }')"

cat >"$fake_scp" <<'SCP'
#!/usr/bin/env bash
set -euo pipefail
printf '%s\n' "$*" >>"${FISHYSTUFF_FAKE_SCP_LOG:?}"
SCP
chmod +x "$fake_scp"

cat >"$fake_push" <<'PUSH'
#!/usr/bin/env bash
set -euo pipefail
printf '%s\n' "$*" >>"${FISHYSTUFF_FAKE_PUSH_LOG:?}"
PUSH
chmod +x "$fake_push"

cat >"$fake_ssh" <<'SSH'
#!/usr/bin/env bash
set -euo pipefail
printf '%s\n' "$*" >>"${FISHYSTUFF_FAKE_REMOTE_LOG:?}"
cat >"${FISHYSTUFF_FAKE_REMOTE_STDIN:?}"
printf 'remote_hostname=site-nbg1-beta\n'
printf 'remote_tls_resident_install_ok=fishystuff-beta-tls-reconciler.service\n'
printf 'remote_tls_resident_desired_target=/var/lib/fishystuff/gitops-beta/desired/beta-tls.desired.json\n'
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
  FISHYSTUFF_GITOPS_ENABLE_BETA_REMOTE_TLS_RESIDENT_CLOSURE_COPY=1 \
  FISHYSTUFF_GITOPS_ENABLE_BETA_REMOTE_TLS_RESIDENT_RESTART=1 \
  FISHYSTUFF_GITOPS_BETA_REMOTE_TLS_RESIDENT_TARGET=root@203.0.113.20 \
  FISHYSTUFF_GITOPS_BETA_TLS_DESIRED_SHA256="$desired_sha256" \
  FISHYSTUFF_GITOPS_BETA_TLS_RESIDENT_UNIT_SHA256="$unit_sha256" \
  FISHYSTUFF_GITOPS_BETA_TLS_CLOUDFLARE_TOKEN_SHA256="$token_sha256" \
  CLOUDFLARE_API_TOKEN=fake-cloudflare-token \
  HETZNER_SSH_PRIVATE_KEY='fixture-private-key' \
  FISHYSTUFF_FAKE_PUSH_LOG="${root}/push.log" \
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
    "$fake_scp" \
    "$fake_push" >"${root}/remote-install.out"

grep -F "gitops_beta_remote_install_tls_resident_checked=true" "${root}/remote-install.out" >/dev/null
grep -F "gitops_beta_remote_install_tls_resident_ok=true" "${root}/remote-install.out" >/dev/null
grep -F "remote_tls_resident_install_ok=fishystuff-beta-tls-reconciler.service" "${root}/remote-install.out" >/dev/null
grep -F "remote_host_mutation_performed=true" "${root}/remote-install.out" >/dev/null
grep -F "resident_gitops_store=/nix/store/" "${root}/remote-install.out" >/dev/null
grep -F "resident_mgmt_store=/tmp/" "${root}/push.log" >/dev/null && {
  printf '[gitops-beta-remote-install-tls-resident-test] push used non-store path\n' >&2
  exit 1
}
grep -F "root@203.0.113.20 /tmp/" "${root}/push.log" >/dev/null && {
  printf '[gitops-beta-remote-install-tls-resident-test] push used fixture tmp path\n' >&2
  exit 1
}
grep -F "root@203.0.113.20 /nix/store/" "${root}/push.log" >/dev/null
grep -F "root@203.0.113.20:/tmp/fishystuff-beta-tls-resident-desired.json" "${root}/scp.log" >/dev/null
grep -F "root@203.0.113.20:/tmp/fishystuff-beta-tls-resident.service" "${root}/scp.log" >/dev/null
grep -F "root@203.0.113.20:/tmp/fishystuff-beta-tls-resident-cloudflare-api-token" "${root}/scp.log" >/dev/null
grep -F "BatchMode=yes" "${root}/scp.log" >/dev/null
grep -F "ConnectTimeout=120" "${root}/scp.log" >/dev/null
grep -F "ConnectionAttempts=1" "${root}/scp.log" >/dev/null
grep -F "ServerAliveInterval=10" "${root}/remote.log" >/dev/null
grep -F "ServerAliveCountMax=3" "${root}/remote.log" >/dev/null
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
  FISHYSTUFF_GITOPS_ENABLE_BETA_REMOTE_TLS_RESIDENT_CLOSURE_COPY=1
  FISHYSTUFF_GITOPS_ENABLE_BETA_REMOTE_TLS_RESIDENT_RESTART=1
  FISHYSTUFF_GITOPS_BETA_REMOTE_TLS_RESIDENT_TARGET=root@203.0.113.20
  FISHYSTUFF_GITOPS_BETA_TLS_DESIRED_SHA256="$desired_sha256"
  FISHYSTUFF_GITOPS_BETA_TLS_RESIDENT_UNIT_SHA256="$unit_sha256"
  FISHYSTUFF_GITOPS_BETA_TLS_CLOUDFLARE_TOKEN_SHA256="$token_sha256"
  CLOUDFLARE_API_TOKEN=fake-cloudflare-token
  HETZNER_SSH_PRIVATE_KEY=fixture-private-key
  FISHYSTUFF_FAKE_PUSH_LOG="${root}/push-fail.log"
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
    bash scripts/recipes/gitops-beta-remote-install-tls-resident.sh root@203.0.113.20 site-nbg1-beta "$desired" "$unit" env:CLOUDFLARE_API_TOKEN "$fake_ssh" "$fake_scp" "$fake_push"

expect_fail_contains \
  "refuses production profile" \
  "gitops-beta-remote-install-tls-resident must not run with production SecretSpec profile active" \
  env \
    "${base_env[@]}" \
    FISHYSTUFF_OPERATOR_SECRETSPEC_PROFILE=production-deploy \
    bash scripts/recipes/gitops-beta-remote-install-tls-resident.sh root@203.0.113.20 site-nbg1-beta "$desired" "$unit" env:CLOUDFLARE_API_TOKEN "$fake_ssh" "$fake_scp" "$fake_push"

expect_fail_contains \
  "refuses stale token hash" \
  "cloudflare_token_source sha256 mismatch" \
  env \
    "${base_env[@]}" \
    FISHYSTUFF_GITOPS_BETA_TLS_CLOUDFLARE_TOKEN_SHA256=0000000000000000000000000000000000000000000000000000000000000000 \
    bash scripts/recipes/gitops-beta-remote-install-tls-resident.sh root@203.0.113.20 site-nbg1-beta "$desired" "$unit" env:CLOUDFLARE_API_TOKEN "$fake_ssh" "$fake_scp" "$fake_push"

printf '[gitops-beta-remote-install-tls-resident-test] %s checks passed\n' "$pass_count"
