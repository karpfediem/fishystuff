#!/usr/bin/env bash
set -euo pipefail

SCRIPT_PATH="$(readlink -f "${BASH_SOURCE[0]}")"
SCRIPT_DIR="$(cd "$(dirname "$SCRIPT_PATH")" && pwd)"
source "${SCRIPT_DIR}/lib/common.sh"

cd "$RECIPE_REPO_ROOT"

pass_count=0

pass() {
  printf '[gitops-beta-tls-resident-unit-test] pass: %s\n' "$1"
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
    printf '[gitops-beta-tls-resident-unit-test] expected failure: %s\n' "$name" >&2
    exit 1
  fi
  if ! grep -F "$expected" "$stderr" >/dev/null; then
    printf '[gitops-beta-tls-resident-unit-test] expected %s stderr to contain %q\n' "$name" "$expected" >&2
    cat "$stderr" >&2
    exit 1
  fi
  pass "$name"
}

root="$(mktemp -d)"
mgmt_dir="${root}/mgmt/bin"
gitops_dir="${root}/gitops"
unit="${root}/fishystuff-beta-tls-reconciler.service"
mkdir -p "$mgmt_dir" "$gitops_dir"

cat >"${mgmt_dir}/mgmt" <<'EOF'
#!/usr/bin/env bash
printf 'fake mgmt\n'
EOF
chmod +x "${mgmt_dir}/mgmt"
printf '# fake main\n' >"${gitops_dir}/main.mcl"

bash scripts/recipes/gitops-beta-tls-resident-unit.sh \
  "$unit" \
  /var/lib/fishystuff/gitops-beta/desired/beta-tls.desired.json \
  "${mgmt_dir}/mgmt" \
  "$gitops_dir" \
  /var/lib/fishystuff/gitops-beta/secrets/cloudflare-api-token \
  -1 >"${root}/unit.stdout" 2>"${root}/unit.stderr"

grep -F "Description=FishyStuff beta GitOps TLS ACME reconciler" "$unit" >/dev/null
grep -F "WorkingDirectory=$(readlink -f "$gitops_dir")" "$unit" >/dev/null
grep -F "Environment=FISHYSTUFF_GITOPS_ENABLE_LOCAL_APPLY=1" "$unit" >/dev/null
grep -F "Environment=FISHYSTUFF_GITOPS_STATE_FILE=/var/lib/fishystuff/gitops-beta/desired/beta-tls.desired.json" "$unit" >/dev/null
grep -F "LoadCredential=cloudflare-api-token:/var/lib/fishystuff/gitops-beta/secrets/cloudflare-api-token" "$unit" >/dev/null
grep -F "ExecStart=/bin/sh -ceu 'export CLOUDFLARE_API_TOKEN=\"\$(cat \"\$CREDENTIALS_DIRECTORY/cloudflare-api-token\")\"; exec $(readlink -f "${mgmt_dir}/mgmt") run --tmp-prefix --no-pgp lang --converged-timeout -1 main.mcl'" "$unit" >/dev/null
grep -F "Nice=10" "$unit" >/dev/null
grep -F "IOSchedulingClass=idle" "$unit" >/dev/null
grep -F "CPUQuota=50%" "$unit" >/dev/null
grep -F "MemoryMax=1536M" "$unit" >/dev/null
grep -F "TasksMax=256" "$unit" >/dev/null
grep -F "ReadWritePaths=/var/lib/fishystuff/gitops-beta" "$unit" >/dev/null
grep -F "ProtectSystem=strict" "$unit" >/dev/null
grep -F "gitops_beta_tls_resident_unit_ok=true" "${root}/unit.stderr" >/dev/null
grep -F "remote_deploy_performed=false" "${root}/unit.stderr" >/dev/null
pass "generate beta TLS resident unit"

if grep -F "fishystuff.fish" "$unit" | grep -v -F "beta.fishystuff" >/dev/null; then
  printf '[gitops-beta-tls-resident-unit-test] unit contains a non-beta production hostname\n' >&2
  cat "$unit" >&2
  exit 1
fi
pass "no production hostnames in unit"

stdout_unit="${root}/stdout.service"
bash scripts/recipes/gitops-beta-tls-resident-unit.sh \
  - \
  /var/lib/fishystuff/gitops-beta/desired/beta-tls.desired.json \
  "${mgmt_dir}/mgmt" \
  "$gitops_dir" \
  /var/lib/fishystuff/gitops-beta/secrets/cloudflare-api-token \
  600 >"$stdout_unit" 2>"${root}/stdout.stderr"
grep -F "ExecStart=/bin/sh -ceu 'export CLOUDFLARE_API_TOKEN=\"\$(cat \"\$CREDENTIALS_DIRECTORY/cloudflare-api-token\")\"; exec $(readlink -f "${mgmt_dir}/mgmt") run --tmp-prefix --no-pgp lang --converged-timeout 600 main.mcl'" "$stdout_unit" >/dev/null
pass "write unit to stdout"

default_timeout_unit="${root}/default-timeout.service"
bash scripts/recipes/gitops-beta-tls-resident-unit.sh \
  "$default_timeout_unit" \
  /var/lib/fishystuff/gitops-beta/desired/beta-tls.desired.json \
  "${mgmt_dir}/mgmt" \
  "$gitops_dir" \
  /var/lib/fishystuff/gitops-beta/secrets/cloudflare-api-token \
  >"${root}/default-timeout.stdout" 2>"${root}/default-timeout.stderr"
grep -F " --converged-timeout 600 main.mcl" "$default_timeout_unit" >/dev/null
pass "default convergence timeout is finite"

expect_fail_contains \
  "refuse non-beta desired state path" \
  "state_file must stay under /var/lib/fishystuff/gitops-beta/desired/" \
  bash scripts/recipes/gitops-beta-tls-resident-unit.sh \
    - \
    /var/lib/fishystuff/gitops/desired/prod-tls.desired.json \
    "${mgmt_dir}/mgmt" \
    "$gitops_dir" \
    /var/lib/fishystuff/gitops-beta/secrets/cloudflare-api-token

expect_fail_contains \
  "refuse non-beta Cloudflare token credential path" \
  "cloudflare_token_credential must stay under /var/lib/fishystuff/gitops-beta/secrets/" \
  bash scripts/recipes/gitops-beta-tls-resident-unit.sh \
    - \
    /var/lib/fishystuff/gitops-beta/desired/beta-tls.desired.json \
    "${mgmt_dir}/mgmt" \
    "$gitops_dir" \
    /var/lib/fishystuff/gitops/secrets/cloudflare-api-token

expect_fail_contains \
  "refuse missing gitops main" \
  "gitops_dir does not contain main.mcl" \
  bash scripts/recipes/gitops-beta-tls-resident-unit.sh \
    - \
    /var/lib/fishystuff/gitops-beta/desired/beta-tls.desired.json \
    "${mgmt_dir}/mgmt" \
    "$root" \
    /var/lib/fishystuff/gitops-beta/secrets/cloudflare-api-token

printf '[gitops-beta-tls-resident-unit-test] %s checks passed\n' "$pass_count"
