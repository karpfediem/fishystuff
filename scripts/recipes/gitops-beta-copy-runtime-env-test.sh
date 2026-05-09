#!/usr/bin/env bash
set -euo pipefail

SCRIPT_PATH="$(readlink -f "${BASH_SOURCE[0]}")"
SCRIPT_DIR="$(cd "$(dirname "$SCRIPT_PATH")" && pwd)"
source "${SCRIPT_DIR}/lib/common.sh"

cd "$RECIPE_REPO_ROOT"

pass_count=0

pass() {
  printf '[gitops-beta-copy-runtime-env-test] pass: %s\n' "$1"
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
    printf '[gitops-beta-copy-runtime-env-test] expected failure: %s\n' "$name" >&2
    exit 1
  fi
  if ! grep -F "$expected" "$stderr" >/dev/null; then
    printf '[gitops-beta-copy-runtime-env-test] expected %s stderr to contain %q\n' "$name" "$expected" >&2
    cat "$stderr" >&2
    exit 1
  fi
  pass "$name"
}

root="$(mktemp -d)"
api_env="${root}/api.env"
dolt_env="${root}/dolt.env"
fake_ssh="${root}/ssh"
fake_scp="${root}/scp"

cat >"$api_env" <<'EOF'
FISHYSTUFF_DATABASE_URL='mysql://fishy:secret@127.0.0.1:3316/fishystuff'
FISHYSTUFF_CORS_ALLOWED_ORIGINS='https://beta.fishystuff.fish'
FISHYSTUFF_PUBLIC_SITE_BASE_URL='https://beta.fishystuff.fish'
FISHYSTUFF_PUBLIC_CDN_BASE_URL='https://cdn.beta.fishystuff.fish'
FISHYSTUFF_RUNTIME_CDN_BASE_URL='https://cdn.beta.fishystuff.fish'
EOF
cat >"$dolt_env" <<'EOF'
# FishyStuff beta Dolt runtime configuration.
EOF

cat >"$fake_ssh" <<'SSH'
#!/usr/bin/env bash
set -euo pipefail
printf 'ssh %s\n' "$*" >>"${FISHYSTUFF_FAKE_REMOTE_LOG:?}"
SSH
chmod +x "$fake_ssh"

cat >"$fake_scp" <<'SCP'
#!/usr/bin/env bash
set -euo pipefail
printf 'scp %s\n' "$*" >>"${FISHYSTUFF_FAKE_REMOTE_LOG:?}"
SCP
chmod +x "$fake_scp"

env \
  FISHYSTUFF_OPERATOR_SECRETSPEC_PROFILE=beta-deploy \
  FISHYSTUFF_GITOPS_ENABLE_BETA_REMOTE_RUNTIME_ENV_COPY=1 \
  FISHYSTUFF_GITOPS_BETA_REMOTE_RUNTIME_ENV_TARGET=root@203.0.113.20 \
  HETZNER_SSH_PRIVATE_KEY='fixture-private-key' \
  FISHYSTUFF_FAKE_REMOTE_LOG="${root}/remote.log" \
  bash scripts/recipes/gitops-beta-copy-runtime-env.sh root@203.0.113.20 "$api_env" "$dolt_env" "$fake_ssh" "$fake_scp" >"${root}/copy.out"
grep -F "gitops_beta_copy_runtime_env_ok=true" "${root}/copy.out" >/dev/null
grep -F "resident_target=root@203.0.113.20" "${root}/copy.out" >/dev/null
grep -F "api_runtime_env_path=/var/lib/fishystuff/gitops-beta/api/runtime.env" "${root}/copy.out" >/dev/null
grep -F "remote_host_mutation_performed=true" "${root}/copy.out" >/dev/null
grep -F "root@203.0.113.20" "${root}/remote.log" >/dev/null
grep -F "/var/lib/fishystuff/gitops-beta/api/runtime.env" "${root}/remote.log" >/dev/null
pass "copies checked beta runtime env files to remote paths"

expect_fail_contains \
  "requires opt-in" \
  "gitops-beta-copy-runtime-env requires FISHYSTUFF_GITOPS_ENABLE_BETA_REMOTE_RUNTIME_ENV_COPY=1" \
  env FISHYSTUFF_OPERATOR_SECRETSPEC_PROFILE=beta-deploy HETZNER_SSH_PRIVATE_KEY='fixture-private-key' \
    bash scripts/recipes/gitops-beta-copy-runtime-env.sh root@203.0.113.20 "$api_env" "$dolt_env" "$fake_ssh" "$fake_scp"

expect_fail_contains \
  "requires target acknowledgement" \
  "gitops-beta-copy-runtime-env requires FISHYSTUFF_GITOPS_BETA_REMOTE_RUNTIME_ENV_TARGET=root@203.0.113.20" \
  env \
    FISHYSTUFF_OPERATOR_SECRETSPEC_PROFILE=beta-deploy \
    FISHYSTUFF_GITOPS_ENABLE_BETA_REMOTE_RUNTIME_ENV_COPY=1 \
    FISHYSTUFF_GITOPS_BETA_REMOTE_RUNTIME_ENV_TARGET=root@203.0.113.21 \
    HETZNER_SSH_PRIVATE_KEY='fixture-private-key' \
    bash scripts/recipes/gitops-beta-copy-runtime-env.sh root@203.0.113.20 "$api_env" "$dolt_env" "$fake_ssh" "$fake_scp"

expect_fail_contains \
  "rejects production profile" \
  "must not run with production SecretSpec profile active" \
  env \
    FISHYSTUFF_OPERATOR_SECRETSPEC_PROFILE=production-deploy \
    FISHYSTUFF_GITOPS_ENABLE_BETA_REMOTE_RUNTIME_ENV_COPY=1 \
    FISHYSTUFF_GITOPS_BETA_REMOTE_RUNTIME_ENV_TARGET=root@203.0.113.20 \
    HETZNER_SSH_PRIVATE_KEY='fixture-private-key' \
    bash scripts/recipes/gitops-beta-copy-runtime-env.sh root@203.0.113.20 "$api_env" "$dolt_env" "$fake_ssh" "$fake_scp"

expect_fail_contains \
  "rejects dns target" \
  "target host must be an IPv4 address" \
  env \
    FISHYSTUFF_OPERATOR_SECRETSPEC_PROFILE=beta-deploy \
    FISHYSTUFF_GITOPS_ENABLE_BETA_REMOTE_RUNTIME_ENV_COPY=1 \
    FISHYSTUFF_GITOPS_BETA_REMOTE_RUNTIME_ENV_TARGET=root@beta.fishystuff.fish \
    HETZNER_SSH_PRIVATE_KEY='fixture-private-key' \
    bash scripts/recipes/gitops-beta-copy-runtime-env.sh root@beta.fishystuff.fish "$api_env" "$dolt_env" "$fake_ssh" "$fake_scp"

printf '[gitops-beta-copy-runtime-env-test] %s checks passed\n' "$pass_count"
