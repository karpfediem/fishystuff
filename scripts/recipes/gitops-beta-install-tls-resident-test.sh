#!/usr/bin/env bash
set -euo pipefail

SCRIPT_PATH="$(readlink -f "${BASH_SOURCE[0]}")"
SCRIPT_DIR="$(cd "$(dirname "$SCRIPT_PATH")" && pwd)"
source "${SCRIPT_DIR}/lib/common.sh"

cd "$RECIPE_REPO_ROOT"

pass_count=0

pass() {
  printf '[gitops-beta-install-tls-resident-test] pass: %s\n' "$1"
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
    printf '[gitops-beta-install-tls-resident-test] expected failure: %s\n' "$name" >&2
    exit 1
  fi
  if ! grep -F "$expected" "$stderr" >/dev/null; then
    printf '[gitops-beta-install-tls-resident-test] expected %s stderr to contain %q\n' "$name" "$expected" >&2
    cat "$stderr" >&2
    exit 1
  fi
  pass "$name"
}

root="$(mktemp -d)"
fake_bin="${root}/bin"
fake_install_root="${root}/install-root"
fake_install_log="${root}/install.log"
fake_systemctl_log="${root}/systemctl.log"
mgmt_dir="${root}/mgmt/bin"
gitops_dir="${root}/gitops"
desired="${root}/beta-tls.staging.desired.json"
unit="${root}/fishystuff-beta-tls-reconciler.service"
token="${root}/cloudflare-api-token"
mkdir -p "$fake_bin" "$fake_install_root" "$mgmt_dir" "$gitops_dir"

cat >"${fake_bin}/hostname" <<'EOF'
#!/usr/bin/env bash
if [[ "${1-}" == "-f" ]]; then
  printf '%s\n' "${FISHYSTUFF_FAKE_HOSTNAME:-site-nbg1-beta}"
else
  printf '%s\n' "${FISHYSTUFF_FAKE_HOSTNAME:-site-nbg1-beta}"
fi
EOF
chmod +x "${fake_bin}/hostname"

cat >"${fake_bin}/install" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
root="${FISHYSTUFF_FAKE_INSTALL_ROOT:?}"
log="${FISHYSTUFF_FAKE_INSTALL_LOG:?}"
mode=""
src=""
dst=""
while [[ $# -gt 0 ]]; do
  case "$1" in
    -D)
      shift
      ;;
    -m)
      mode="$2"
      shift 2
      ;;
    *)
      if [[ -z "$src" ]]; then
        src="$1"
      elif [[ -z "$dst" ]]; then
        dst="$1"
      else
        exit 2
      fi
      shift
      ;;
  esac
done
target="${root}${dst}"
mkdir -p "$(dirname "$target")"
cp "$src" "$target"
chmod "$mode" "$target"
printf 'install mode=%s src=%s dst=%s\n' "$mode" "$src" "$dst" >>"$log"
EOF
chmod +x "${fake_bin}/install"

cat >"${fake_bin}/systemctl" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
printf 'systemctl %s\n' "$*" >>"${FISHYSTUFF_FAKE_SYSTEMCTL_LOG:?}"
if [[ "${1-}" == "is-active" ]]; then
  exit 0
fi
exit 0
EOF
chmod +x "${fake_bin}/systemctl"

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

base_env=(
  PATH="${fake_bin}:${PATH}"
  FISHYSTUFF_FAKE_INSTALL_ROOT="$fake_install_root"
  FISHYSTUFF_FAKE_INSTALL_LOG="$fake_install_log"
  FISHYSTUFF_FAKE_SYSTEMCTL_LOG="$fake_systemctl_log"
  CLOUDFLARE_API_TOKEN=fake-cloudflare-token
)

expect_fail_contains \
  "refuse without install opt-in" \
  "gitops-beta-install-tls-resident requires FISHYSTUFF_GITOPS_ENABLE_BETA_TLS_RESIDENT_INSTALL=1" \
  env "${base_env[@]}" \
    bash scripts/recipes/gitops-beta-install-tls-resident.sh "$desired" "$unit" env:CLOUDFLARE_API_TOKEN install systemctl

expect_fail_contains \
  "refuse without restart opt-in" \
  "gitops-beta-install-tls-resident requires FISHYSTUFF_GITOPS_ENABLE_BETA_TLS_RESIDENT_RESTART=1" \
  env "${base_env[@]}" FISHYSTUFF_GITOPS_ENABLE_BETA_TLS_RESIDENT_INSTALL=1 \
    bash scripts/recipes/gitops-beta-install-tls-resident.sh "$desired" "$unit" env:CLOUDFLARE_API_TOKEN install systemctl

expect_fail_contains \
  "refuse without reviewed desired hash" \
  "gitops-beta-install-tls-resident requires FISHYSTUFF_GITOPS_BETA_TLS_DESIRED_SHA256" \
  env "${base_env[@]}" FISHYSTUFF_GITOPS_ENABLE_BETA_TLS_RESIDENT_INSTALL=1 FISHYSTUFF_GITOPS_ENABLE_BETA_TLS_RESIDENT_RESTART=1 \
    bash scripts/recipes/gitops-beta-install-tls-resident.sh "$desired" "$unit" env:CLOUDFLARE_API_TOKEN install systemctl

expect_fail_contains \
  "refuse stale unit hash" \
  "unit_file sha256 mismatch" \
  env "${base_env[@]}" \
    FISHYSTUFF_GITOPS_ENABLE_BETA_TLS_RESIDENT_INSTALL=1 \
    FISHYSTUFF_GITOPS_ENABLE_BETA_TLS_RESIDENT_RESTART=1 \
    FISHYSTUFF_GITOPS_BETA_TLS_DESIRED_SHA256="$desired_sha256" \
    FISHYSTUFF_GITOPS_BETA_TLS_RESIDENT_UNIT_SHA256=0000000000000000000000000000000000000000000000000000000000000000 \
    FISHYSTUFF_GITOPS_BETA_TLS_CLOUDFLARE_TOKEN_SHA256="$token_sha256" \
    bash scripts/recipes/gitops-beta-install-tls-resident.sh "$desired" "$unit" env:CLOUDFLARE_API_TOKEN install systemctl

expect_fail_contains \
  "refuse wrong host" \
  "gitops-beta-install-tls-resident requires current hostname to match beta resident hostname" \
  env "${base_env[@]}" \
    FISHYSTUFF_FAKE_HOSTNAME=operator-dev \
    FISHYSTUFF_GITOPS_ENABLE_BETA_TLS_RESIDENT_INSTALL=1 \
    FISHYSTUFF_GITOPS_ENABLE_BETA_TLS_RESIDENT_RESTART=1 \
    FISHYSTUFF_GITOPS_BETA_TLS_DESIRED_SHA256="$desired_sha256" \
    FISHYSTUFF_GITOPS_BETA_TLS_RESIDENT_UNIT_SHA256="$unit_sha256" \
    FISHYSTUFF_GITOPS_BETA_TLS_CLOUDFLARE_TOKEN_SHA256="$token_sha256" \
    bash scripts/recipes/gitops-beta-install-tls-resident.sh "$desired" "$unit" env:CLOUDFLARE_API_TOKEN install systemctl

expect_fail_contains \
  "refuse production profile" \
  "gitops-beta-install-tls-resident must not run with a production SecretSpec profile" \
  env "${base_env[@]}" \
    FISHYSTUFF_OPERATOR_SECRETSPEC_PROFILE=production-deploy \
    FISHYSTUFF_GITOPS_ENABLE_BETA_TLS_RESIDENT_INSTALL=1 \
    FISHYSTUFF_GITOPS_ENABLE_BETA_TLS_RESIDENT_RESTART=1 \
    FISHYSTUFF_GITOPS_BETA_TLS_DESIRED_SHA256="$desired_sha256" \
    FISHYSTUFF_GITOPS_BETA_TLS_RESIDENT_UNIT_SHA256="$unit_sha256" \
    FISHYSTUFF_GITOPS_BETA_TLS_CLOUDFLARE_TOKEN_SHA256="$token_sha256" \
    bash scripts/recipes/gitops-beta-install-tls-resident.sh "$desired" "$unit" env:CLOUDFLARE_API_TOKEN install systemctl

env "${base_env[@]}" \
  FISHYSTUFF_GITOPS_ENABLE_BETA_TLS_RESIDENT_INSTALL=1 \
  FISHYSTUFF_GITOPS_ENABLE_BETA_TLS_RESIDENT_RESTART=1 \
  FISHYSTUFF_GITOPS_BETA_TLS_DESIRED_SHA256="$desired_sha256" \
  FISHYSTUFF_GITOPS_BETA_TLS_RESIDENT_UNIT_SHA256="$unit_sha256" \
  FISHYSTUFF_GITOPS_BETA_TLS_CLOUDFLARE_TOKEN_SHA256="$token_sha256" \
  bash scripts/recipes/gitops-beta-install-tls-resident.sh "$desired" "$unit" env:CLOUDFLARE_API_TOKEN install systemctl >"${root}/install.stdout"

grep -F "gitops_beta_tls_resident_install_ok=fishystuff-beta-tls-reconciler.service" "${root}/install.stdout" >/dev/null
grep -F "local_host_mutation_performed=true" "${root}/install.stdout" >/dev/null
grep -F "install mode=0644 src=${desired} dst=/var/lib/fishystuff/gitops-beta/desired/beta-tls.staging.desired.json" "$fake_install_log" >/dev/null
grep -F "install mode=0600" "$fake_install_log" | grep -F "dst=/var/lib/fishystuff/gitops-beta/secrets/cloudflare-api-token" >/dev/null
grep -F "install mode=0644 src=${unit} dst=/etc/systemd/system/fishystuff-beta-tls-reconciler.service" "$fake_install_log" >/dev/null
grep -F "systemctl daemon-reload" "$fake_systemctl_log" >/dev/null
grep -F "systemctl enable --now fishystuff-beta-tls-reconciler.service" "$fake_systemctl_log" >/dev/null
grep -F "systemctl restart fishystuff-beta-tls-reconciler.service" "$fake_systemctl_log" >/dev/null
grep -F "systemctl is-active --quiet fishystuff-beta-tls-reconciler.service" "$fake_systemctl_log" >/dev/null
test "$(stat -c '%a' "${fake_install_root}/var/lib/fishystuff/gitops-beta/secrets/cloudflare-api-token")" = "600"
grep -Fx "fake-cloudflare-token" "${fake_install_root}/var/lib/fishystuff/gitops-beta/secrets/cloudflare-api-token" >/dev/null
pass "valid beta TLS resident install gate"

printf '[gitops-beta-install-tls-resident-test] %s checks passed\n' "$pass_count"
