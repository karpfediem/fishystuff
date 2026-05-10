#!/usr/bin/env bash
set -euo pipefail

SCRIPT_PATH="$(readlink -f "${BASH_SOURCE[0]}")"
SCRIPT_DIR="$(cd "$(dirname "$SCRIPT_PATH")" && pwd)"
source "${SCRIPT_DIR}/lib/common.sh"

cd "$RECIPE_REPO_ROOT"

pass_count=0

pass() {
  printf '[gitops-beta-reconcile-tls-test] pass: %s\n' "$1"
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
    printf '[gitops-beta-reconcile-tls-test] expected failure: %s\n' "$name" >&2
    exit 1
  fi
  if ! grep -F "$expected" "$stderr" >/dev/null; then
    printf '[gitops-beta-reconcile-tls-test] expected %s stderr to contain %q\n' "$name" "$expected" >&2
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

cat >"${fake_bin}/hostname" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
printf '%s\n' "${FISHYSTUFF_FAKE_HOSTNAME:-operator-dev}"
EOF
chmod +x "${fake_bin}/hostname"
PATH="${fake_bin}:${PATH}"

state="${root}/beta-tls.desired.json"
bash scripts/recipes/gitops-beta-tls-desired.sh \
  "$state" \
  staging \
  ops@fishystuff.invalid >/dev/null 2>"${root}/desired.stderr"

expect_fail_contains \
  "refuse without beta TLS apply opt-in" \
  "gitops-beta-reconcile-tls requires FISHYSTUFF_GITOPS_ENABLE_BETA_TLS_APPLY=1" \
  bash scripts/recipes/gitops-beta-reconcile-tls.sh "$state" staging /tmp/mgmt

expect_fail_contains \
  "refuse without local apply opt-in" \
  "gitops-beta-reconcile-tls requires FISHYSTUFF_GITOPS_ENABLE_LOCAL_APPLY=1" \
  env FISHYSTUFF_GITOPS_ENABLE_BETA_TLS_APPLY=1 \
    bash scripts/recipes/gitops-beta-reconcile-tls.sh "$state" staging /tmp/mgmt

expect_fail_contains \
  "refuse without Cloudflare token" \
  "gitops-beta-reconcile-tls requires CLOUDFLARE_API_TOKEN from beta-deploy SecretSpec" \
  env FISHYSTUFF_GITOPS_ENABLE_BETA_TLS_APPLY=1 FISHYSTUFF_GITOPS_ENABLE_LOCAL_APPLY=1 \
    bash scripts/recipes/gitops-beta-reconcile-tls.sh "$state" staging /tmp/mgmt

expect_fail_contains \
  "refuse wrong beta resident hostname" \
  "gitops-beta-reconcile-tls requires current hostname to match beta resident hostname" \
  env FISHYSTUFF_GITOPS_ENABLE_BETA_TLS_APPLY=1 FISHYSTUFF_GITOPS_ENABLE_LOCAL_APPLY=1 CLOUDFLARE_API_TOKEN=fake \
    bash scripts/recipes/gitops-beta-reconcile-tls.sh "$state" staging /tmp/mgmt

expect_fail_contains \
  "refuse production ACME without production opt-in" \
  "gitops-beta-reconcile-tls requires FISHYSTUFF_GITOPS_ENABLE_BETA_TLS_PRODUCTION_ACME=1" \
  env FISHYSTUFF_GITOPS_ENABLE_BETA_TLS_APPLY=1 FISHYSTUFF_GITOPS_ENABLE_LOCAL_APPLY=1 CLOUDFLARE_API_TOKEN=fake \
    bash scripts/recipes/gitops-beta-reconcile-tls.sh "$state" production /tmp/mgmt

printf '[gitops-beta-reconcile-tls-test] %s checks passed\n' "$pass_count"
