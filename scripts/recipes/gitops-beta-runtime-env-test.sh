#!/usr/bin/env bash
set -euo pipefail

SCRIPT_PATH="$(readlink -f "${BASH_SOURCE[0]}")"
SCRIPT_DIR="$(cd "$(dirname "$SCRIPT_PATH")" && pwd)"
source "${SCRIPT_DIR}/lib/common.sh"

cd "$RECIPE_REPO_ROOT"

pass_count=0

pass() {
  printf '[gitops-beta-runtime-env-test] pass: %s\n' "$1"
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
    printf '[gitops-beta-runtime-env-test] expected failure: %s\n' "$name" >&2
    exit 1
  fi
  if ! grep -F "$expected" "$stderr" >/dev/null; then
    printf '[gitops-beta-runtime-env-test] expected %s stderr to contain %q\n' "$name" "$expected" >&2
    cat "$stderr" >&2
    exit 1
  fi
  pass "$name"
}

root="$(mktemp -d)"
api_env="${root}/api/runtime.env"
dolt_env="${root}/dolt/beta.env"
wrong_host_bin="${root}/wrong-host-bin"
mkdir -p "$wrong_host_bin"
cat >"${wrong_host_bin}/hostname" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail

printf 'operator-dev\n'
EOF
chmod +x "${wrong_host_bin}/hostname"

expect_fail_contains \
  "refuse API runtime env write without opt-in" \
  "gitops-beta-write-runtime-env requires FISHYSTUFF_GITOPS_ENABLE_BETA_API_RUNTIME_ENV_WRITE=1" \
  env \
    FISHYSTUFF_GITOPS_BETA_API_DATABASE_URL="mysql://fishy:secret@127.0.0.1:3316/fishystuff" \
    bash scripts/recipes/gitops-beta-write-runtime-env.sh api "$api_env"

expect_fail_contains \
  "refuse API runtime env write without database URL" \
  "gitops-beta-write-runtime-env requires FISHYSTUFF_GITOPS_BETA_API_DATABASE_URL" \
  env \
    FISHYSTUFF_GITOPS_ENABLE_BETA_API_RUNTIME_ENV_WRITE=1 \
    bash scripts/recipes/gitops-beta-write-runtime-env.sh api "$api_env"

expect_fail_contains \
  "refuse API runtime env remote database URL" \
  "must point at the beta loopback Dolt SQL port 3316" \
  env \
    FISHYSTUFF_GITOPS_ENABLE_BETA_API_RUNTIME_ENV_WRITE=1 \
    FISHYSTUFF_GITOPS_BETA_API_DATABASE_URL="mysql://fishy:secret@db.example.invalid:3306/fishystuff" \
    bash scripts/recipes/gitops-beta-write-runtime-env.sh api "$api_env"

expect_fail_contains \
  "refuse API runtime env production site URL" \
  "must be https://beta.fishystuff.fish" \
  env \
    FISHYSTUFF_GITOPS_ENABLE_BETA_API_RUNTIME_ENV_WRITE=1 \
    FISHYSTUFF_GITOPS_BETA_API_DATABASE_URL="mysql://fishy:secret@127.0.0.1:3316/fishystuff" \
    FISHYSTUFF_GITOPS_BETA_PUBLIC_SITE_BASE_URL="https://fishystuff.fish" \
    bash scripts/recipes/gitops-beta-write-runtime-env.sh api "$api_env"

expect_fail_contains \
  "refuse real runtime env write on wrong host" \
  "gitops-beta-write-runtime-env requires current hostname to match beta resident hostname" \
  env \
    PATH="${wrong_host_bin}:$PATH" \
    FISHYSTUFF_GITOPS_ENABLE_BETA_API_RUNTIME_ENV_WRITE=1 \
    FISHYSTUFF_GITOPS_BETA_API_DATABASE_URL="mysql://fishy:secret@127.0.0.1:3316/fishystuff" \
    bash scripts/recipes/gitops-beta-write-runtime-env.sh api

env \
  FISHYSTUFF_GITOPS_ENABLE_BETA_API_RUNTIME_ENV_WRITE=1 \
  FISHYSTUFF_GITOPS_BETA_API_DATABASE_URL="mysql://fishy:secret@127.0.0.1:3316/fishystuff" \
  bash scripts/recipes/gitops-beta-write-runtime-env.sh api "$api_env" >"${root}/write-api.stdout"
grep -F "gitops_beta_runtime_env_write_ok=${api_env}" "${root}/write-api.stdout" >/dev/null
grep -Fx "FISHYSTUFF_DATABASE_URL='mysql://fishy:secret@127.0.0.1:3316/fishystuff'" "$api_env" >/dev/null
grep -Fx "FISHYSTUFF_PUBLIC_SITE_BASE_URL='https://beta.fishystuff.fish'" "$api_env" >/dev/null
grep -Fx "FISHYSTUFF_PUBLIC_CDN_BASE_URL='https://cdn.beta.fishystuff.fish'" "$api_env" >/dev/null
pass "write beta API runtime env"

bash scripts/recipes/gitops-check-beta-runtime-env.sh api "$api_env" >"${root}/check-api.stdout"
grep -F "gitops_beta_runtime_env_ok=${api_env}" "${root}/check-api.stdout" >/dev/null
grep -F "gitops_beta_runtime_env_database=loopback-dolt-beta" "${root}/check-api.stdout" >/dev/null
grep -F "gitops_beta_runtime_env_public_cdn_base_url=https://cdn.beta.fishystuff.fish" "${root}/check-api.stdout" >/dev/null
pass "check beta API runtime env"

fake_bin="${root}/fake-bin"
mkdir -p "$fake_bin"
cat >"${fake_bin}/secretspec" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail

if [[ "$#" -lt 5 || "$1" != "run" || "$2" != "--profile" || "$3" != "beta-runtime" || "$4" != "--" ]]; then
  echo "unexpected fake secretspec invocation: $*" >&2
  exit 89
fi

shift 4
export FISHYSTUFF_GITOPS_BETA_API_DATABASE_URL="mysql://fishy:secret@127.0.0.1:3316/fishystuff"
exec "$@"
EOF
chmod +x "${fake_bin}/secretspec"

secret_api_env="${root}/api/runtime-from-secretspec.env"
env \
  PATH="${fake_bin}:$PATH" \
  FISHYSTUFF_GITOPS_ENABLE_BETA_API_RUNTIME_ENV_WRITE=1 \
  bash scripts/recipes/gitops-beta-write-runtime-env-secretspec.sh api "$secret_api_env" beta-runtime >"${root}/write-api-secretspec.stdout"
grep -F "gitops_beta_runtime_env_write_ok=${secret_api_env}" "${root}/write-api-secretspec.stdout" >/dev/null
grep -Fx "FISHYSTUFF_DATABASE_URL='mysql://fishy:secret@127.0.0.1:3316/fishystuff'" "$secret_api_env" >/dev/null
pass "write beta API runtime env through SecretSpec wrapper"

expect_fail_contains \
  "reject broad SecretSpec profile for beta API runtime env" \
  "gitops-beta-write-runtime-env-secretspec requires profile=beta-runtime" \
  bash scripts/recipes/gitops-beta-write-runtime-env-secretspec.sh api "$secret_api_env" beta-deploy

expect_fail_contains \
  "reject SecretSpec wrapper for Dolt runtime env" \
  "gitops-beta-write-runtime-env-secretspec only supports service=api" \
  bash scripts/recipes/gitops-beta-write-runtime-env-secretspec.sh dolt "$dolt_env" beta-runtime

bad_api_env="${root}/bad-api.env"
cat >"$bad_api_env" <<'EOF'
FISHYSTUFF_DATABASE_URL='mysql://fishy:secret@127.0.0.1:3316/fishystuff'
FISHYSTUFF_CORS_ALLOWED_ORIGINS='https://fishystuff.fish'
FISHYSTUFF_PUBLIC_SITE_BASE_URL='https://beta.fishystuff.fish'
FISHYSTUFF_PUBLIC_CDN_BASE_URL='https://cdn.beta.fishystuff.fish'
FISHYSTUFF_RUNTIME_CDN_BASE_URL='https://cdn.beta.fishystuff.fish'
EOF
expect_fail_contains \
  "reject API runtime env production CORS origin" \
  "production or shared deployment material" \
  bash scripts/recipes/gitops-check-beta-runtime-env.sh api "$bad_api_env"

bad_format_env="${root}/bad-format.env"
cat >"$bad_format_env" <<'EOF'
not an assignment
EOF
expect_fail_contains \
  "reject malformed beta runtime env file" \
  "invalid env assignment" \
  bash scripts/recipes/gitops-check-beta-runtime-env.sh dolt "$bad_format_env"

env \
  FISHYSTUFF_GITOPS_ENABLE_BETA_DOLT_RUNTIME_ENV_WRITE=1 \
  bash scripts/recipes/gitops-beta-write-runtime-env.sh dolt "$dolt_env" >"${root}/write-dolt.stdout"
grep -F "gitops_beta_runtime_env_write_ok=${dolt_env}" "${root}/write-dolt.stdout" >/dev/null
bash scripts/recipes/gitops-check-beta-runtime-env.sh dolt "$dolt_env" >"${root}/check-dolt.stdout"
grep -F "gitops_beta_runtime_env_service=dolt" "${root}/check-dolt.stdout" >/dev/null
pass "write and check beta Dolt runtime env"

bad_dolt_env="${root}/bad-dolt.env"
cat >"$bad_dolt_env" <<'EOF'
DOLT_REMOTE_BRANCH=main
EOF
expect_fail_contains \
  "reject non-beta Dolt remote branch" \
  "DOLT_REMOTE_BRANCH must be beta" \
  bash scripts/recipes/gitops-check-beta-runtime-env.sh dolt "$bad_dolt_env"

printf '[gitops-beta-runtime-env-test] %s checks passed\n' "$pass_count"
