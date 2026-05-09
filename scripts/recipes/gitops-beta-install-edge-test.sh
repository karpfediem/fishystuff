#!/usr/bin/env bash
set -euo pipefail

SCRIPT_PATH="$(readlink -f "${BASH_SOURCE[0]}")"
SCRIPT_DIR="$(cd "$(dirname "$SCRIPT_PATH")" && pwd)"
source "${SCRIPT_DIR}/lib/common.sh"

cd "$RECIPE_REPO_ROOT"

FISHYSTUFF_GITOPS_BETA_HOST_HANDOFF_PLAN_TEST_SOURCE_ONLY=1
source scripts/recipes/gitops-beta-host-handoff-plan-test.sh
unset FISHYSTUFF_GITOPS_BETA_HOST_HANDOFF_PLAN_TEST_SOURCE_ONLY

FISHYSTUFF_GITOPS_BETA_VERIFY_ACTIVATION_SERVED_TEST_SOURCE_ONLY=1
source scripts/recipes/gitops-beta-verify-activation-served-test.sh
unset FISHYSTUFF_GITOPS_BETA_VERIFY_ACTIVATION_SERVED_TEST_SOURCE_ONLY

pass_count=0

pass() {
  printf '[gitops-beta-install-edge-test] pass: %s\n' "$1"
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
    printf '[gitops-beta-install-edge-test] expected failure: %s\n' "$name" >&2
    exit 1
  fi
  if ! grep -F "$expected" "$stderr" >/dev/null; then
    printf '[gitops-beta-install-edge-test] expected %s stderr to contain %q\n' "$name" "$expected" >&2
    cat "$stderr" >&2
    exit 1
  fi
  pass "$name"
}

write_beta_placeholder_tls() {
  local credentials_dir="$1"

  mkdir -p "$credentials_dir"
  openssl req \
    -x509 \
    -newkey rsa:2048 \
    -nodes \
    -keyout "${credentials_dir}/privkey.pem" \
    -out "${credentials_dir}/fullchain.pem" \
    -days 1 \
    -subj "/CN=beta.fishystuff.fish" \
    -addext "subjectAltName=DNS:beta.fishystuff.fish,DNS:api.beta.fishystuff.fish,DNS:cdn.beta.fishystuff.fish,DNS:telemetry.beta.fishystuff.fish" \
    >"${credentials_dir}/openssl.log" 2>&1
}

write_fake_install() {
  local path="$1"

  cat >"$path" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail

root="${FISHYSTUFF_FAKE_INSTALL_ROOT:?}"
log="${FISHYSTUFF_FAKE_INSTALL_LOG:?}"
printf '%s\n' "$*" >>"$log"
if [[ "$#" -ne 5 || "$1" != "-D" || "$2" != "-m" || "$3" != "0644" ]]; then
  echo "unexpected fake install args: $*" >&2
  exit 2
fi
source_path="$4"
target_path="$5"
if [[ "$target_path" != "/etc/systemd/system/fishystuff-beta-edge.service" ]]; then
  echo "fake install saw non-beta target: ${target_path}" >&2
  exit 2
fi
mkdir -p "${root}$(dirname "$target_path")"
cp "$source_path" "${root}${target_path}"
EOF
  chmod +x "$path"
}

write_fake_systemctl() {
  local path="$1"

  cat >"$path" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail

log="${FISHYSTUFF_FAKE_SYSTEMCTL_LOG:?}"
printf '%s\n' "$*" >>"$log"
case "$*" in
  daemon-reload | \
  "restart fishystuff-beta-edge.service" | \
  "is-active --quiet fishystuff-beta-edge.service")
    ;;
  *)
    echo "unexpected fake systemctl args: $*" >&2
    exit 2
    ;;
esac
EOF
  chmod +x "$path"
}

root="$(mktemp -d)"
deploy_bin="$(require_deploy_bin)"
write_beta_activation_inputs "$root"
make_beta_edge_bundle "${root}/edge-bundle"
make_fake_served_deploy "${root}/fishystuff_deploy_served" "$deploy_bin"
write_beta_placeholder_tls "${root}/tls"
cp "${root}/edge-bundle/artifacts/systemd/unit" "${root}/fishystuff-beta-edge.service"

fake_install="${root}/install"
fake_systemctl="${root}/systemctl"
fake_install_root="${root}/fake-install-root"
fake_install_log="${root}/fake-install.log"
fake_systemctl_log="${root}/fake-systemctl.log"
write_fake_install "$fake_install"
write_fake_systemctl "$fake_systemctl"
touch "$fake_install_log" "$fake_systemctl_log"

draft="$(cat "${root}/draft.path")"
summary="$(cat "${root}/summary.path")"
admission="$(cat "${root}/admission.path")"
release_id="$(jq -er '.environments.beta.active_release' "$draft")"
proof_dir="${root}/proofs"

write_beta_served_state "${root}/state" "${root}/run" "$draft" "$release_id"

bash scripts/recipes/gitops-beta-operator-proof.sh \
  "$proof_dir" \
  "$draft" \
  "$summary" \
  "$admission" \
  "${root}/edge-bundle" \
  "${root}/fishystuff_deploy" \
  false \
  "" \
  "" \
  "${root}/state" \
  "${root}/run" \
  "${root}/fishystuff-beta-edge.service" \
  "${root}/tls/fullchain.pem" \
  "${root}/tls/privkey.pem" >"${root}/operator-proof.stdout"

operator_proof="$(awk -F= '$1 == "gitops_beta_operator_proof_ok" { print $2 }' "${root}/operator-proof.stdout")"
test -n "$operator_proof"
test -f "$operator_proof"

bash scripts/recipes/gitops-beta-served-proof.sh \
  "$proof_dir" \
  "$draft" \
  "$summary" \
  "$admission" \
  "$operator_proof" \
  "${root}/fishystuff_deploy_served" \
  "${root}/state" \
  "${root}/run" \
  86400 >"${root}/served-proof.stdout"

served_proof="$(awk -F= '$1 == "gitops_beta_served_proof_ok" { print $2 }' "${root}/served-proof.stdout")"
test -n "$served_proof"
test -f "$served_proof"

bash scripts/recipes/gitops-beta-proof-index.sh "$proof_dir" 86400 true >"${root}/proof-index.stdout"
served_proof_sha256="$(awk -F= '$1 == "gitops_beta_proof_index_served_proof_sha256" { print $2 }' "${root}/proof-index.stdout")"
test -n "$served_proof_sha256"
read -r unit_sha256 _ < <(sha256sum "${root}/edge-bundle/artifacts/systemd/unit")

expect_fail_contains \
  "refuse without beta edge install opt-in" \
  "gitops-beta-install-edge requires FISHYSTUFF_GITOPS_ENABLE_BETA_EDGE_INSTALL=1" \
  env FISHYSTUFF_FAKE_INSTALL_ROOT="$fake_install_root" FISHYSTUFF_FAKE_INSTALL_LOG="$fake_install_log" FISHYSTUFF_FAKE_SYSTEMCTL_LOG="$fake_systemctl_log" \
    bash scripts/recipes/gitops-beta-install-edge.sh \
      "${root}/edge-bundle" \
      "$proof_dir" \
      86400 \
      "$fake_install" \
      "$fake_systemctl"

expect_fail_contains \
  "refuse without beta edge restart opt-in" \
  "gitops-beta-install-edge requires FISHYSTUFF_GITOPS_ENABLE_BETA_EDGE_RESTART=1" \
  env FISHYSTUFF_GITOPS_ENABLE_BETA_EDGE_INSTALL=1 FISHYSTUFF_FAKE_INSTALL_ROOT="$fake_install_root" FISHYSTUFF_FAKE_INSTALL_LOG="$fake_install_log" FISHYSTUFF_FAKE_SYSTEMCTL_LOG="$fake_systemctl_log" \
    bash scripts/recipes/gitops-beta-install-edge.sh \
      "${root}/edge-bundle" \
      "$proof_dir" \
      86400 \
      "$fake_install" \
      "$fake_systemctl"

expect_fail_contains \
  "refuse without reviewed served proof hash" \
  "gitops-beta-install-edge requires FISHYSTUFF_GITOPS_BETA_EDGE_SERVED_PROOF_SHA256" \
  env FISHYSTUFF_GITOPS_ENABLE_BETA_EDGE_INSTALL=1 FISHYSTUFF_GITOPS_ENABLE_BETA_EDGE_RESTART=1 FISHYSTUFF_FAKE_INSTALL_ROOT="$fake_install_root" FISHYSTUFF_FAKE_INSTALL_LOG="$fake_install_log" FISHYSTUFF_FAKE_SYSTEMCTL_LOG="$fake_systemctl_log" \
    bash scripts/recipes/gitops-beta-install-edge.sh \
      "${root}/edge-bundle" \
      "$proof_dir" \
      86400 \
      "$fake_install" \
      "$fake_systemctl"

expect_fail_contains \
  "refuse stale served proof hash" \
  "FISHYSTUFF_GITOPS_BETA_EDGE_SERVED_PROOF_SHA256 does not match latest beta served proof" \
  env FISHYSTUFF_GITOPS_ENABLE_BETA_EDGE_INSTALL=1 FISHYSTUFF_GITOPS_ENABLE_BETA_EDGE_RESTART=1 FISHYSTUFF_GITOPS_BETA_EDGE_SERVED_PROOF_SHA256=0000000000000000000000000000000000000000000000000000000000000000 FISHYSTUFF_GITOPS_BETA_EDGE_UNIT_SHA256="$unit_sha256" FISHYSTUFF_FAKE_INSTALL_ROOT="$fake_install_root" FISHYSTUFF_FAKE_INSTALL_LOG="$fake_install_log" FISHYSTUFF_FAKE_SYSTEMCTL_LOG="$fake_systemctl_log" \
    bash scripts/recipes/gitops-beta-install-edge.sh \
      "${root}/edge-bundle" \
      "$proof_dir" \
      86400 \
      "$fake_install" \
      "$fake_systemctl"

expect_fail_contains \
  "refuse without reviewed unit hash" \
  "gitops-beta-install-edge requires FISHYSTUFF_GITOPS_BETA_EDGE_UNIT_SHA256" \
  env FISHYSTUFF_GITOPS_ENABLE_BETA_EDGE_INSTALL=1 FISHYSTUFF_GITOPS_ENABLE_BETA_EDGE_RESTART=1 FISHYSTUFF_GITOPS_BETA_EDGE_SERVED_PROOF_SHA256="$served_proof_sha256" FISHYSTUFF_FAKE_INSTALL_ROOT="$fake_install_root" FISHYSTUFF_FAKE_INSTALL_LOG="$fake_install_log" FISHYSTUFF_FAKE_SYSTEMCTL_LOG="$fake_systemctl_log" \
    bash scripts/recipes/gitops-beta-install-edge.sh \
      "${root}/edge-bundle" \
      "$proof_dir" \
      86400 \
      "$fake_install" \
      "$fake_systemctl"

expect_fail_contains \
  "refuse stale unit hash" \
  "FISHYSTUFF_GITOPS_BETA_EDGE_UNIT_SHA256 does not match beta edge systemd unit" \
  env FISHYSTUFF_GITOPS_ENABLE_BETA_EDGE_INSTALL=1 FISHYSTUFF_GITOPS_ENABLE_BETA_EDGE_RESTART=1 FISHYSTUFF_GITOPS_BETA_EDGE_SERVED_PROOF_SHA256="$served_proof_sha256" FISHYSTUFF_GITOPS_BETA_EDGE_UNIT_SHA256=0000000000000000000000000000000000000000000000000000000000000000 FISHYSTUFF_FAKE_INSTALL_ROOT="$fake_install_root" FISHYSTUFF_FAKE_INSTALL_LOG="$fake_install_log" FISHYSTUFF_FAKE_SYSTEMCTL_LOG="$fake_systemctl_log" \
    bash scripts/recipes/gitops-beta-install-edge.sh \
      "${root}/edge-bundle" \
      "$proof_dir" \
      86400 \
      "$fake_install" \
      "$fake_systemctl"

operator_only_dir="${root}/operator-only-proofs"
mkdir -p "$operator_only_dir"
cp "$operator_proof" "${operator_only_dir}/$(basename "$operator_proof")"
expect_fail_contains \
  "refuse incomplete beta proof index" \
  "gitops_beta_proof_index_status=missing_served_proof" \
  env FISHYSTUFF_GITOPS_ENABLE_BETA_EDGE_INSTALL=1 FISHYSTUFF_GITOPS_ENABLE_BETA_EDGE_RESTART=1 FISHYSTUFF_GITOPS_BETA_EDGE_SERVED_PROOF_SHA256="$served_proof_sha256" FISHYSTUFF_GITOPS_BETA_EDGE_UNIT_SHA256="$unit_sha256" FISHYSTUFF_FAKE_INSTALL_ROOT="$fake_install_root" FISHYSTUFF_FAKE_INSTALL_LOG="$fake_install_log" FISHYSTUFF_FAKE_SYSTEMCTL_LOG="$fake_systemctl_log" \
    bash scripts/recipes/gitops-beta-install-edge.sh \
      "${root}/edge-bundle" \
      "$operator_only_dir" \
      86400 \
      "$fake_install" \
      "$fake_systemctl"

: >"$fake_install_log"
: >"$fake_systemctl_log"
env \
  FISHYSTUFF_GITOPS_ENABLE_BETA_EDGE_INSTALL=1 \
  FISHYSTUFF_GITOPS_ENABLE_BETA_EDGE_RESTART=1 \
  FISHYSTUFF_GITOPS_BETA_EDGE_SERVED_PROOF_SHA256="$served_proof_sha256" \
  FISHYSTUFF_GITOPS_BETA_EDGE_UNIT_SHA256="$unit_sha256" \
  FISHYSTUFF_FAKE_INSTALL_ROOT="$fake_install_root" \
  FISHYSTUFF_FAKE_INSTALL_LOG="$fake_install_log" \
  FISHYSTUFF_FAKE_SYSTEMCTL_LOG="$fake_systemctl_log" \
  bash scripts/recipes/gitops-beta-install-edge.sh \
    "${root}/edge-bundle" \
    "$proof_dir" \
    86400 \
    "$fake_install" \
    "$fake_systemctl" \
    >"${root}/install-edge.stdout" \
    2>"${root}/install-edge.stderr"

grep -F "gitops_beta_edge_install_ok=fishystuff-beta-edge.service" "${root}/install-edge.stdout" >/dev/null
grep -F "gitops_beta_edge_install_environment=beta" "${root}/install-edge.stdout" >/dev/null
grep -F "gitops_beta_edge_install_bundle=${root}/edge-bundle" "${root}/install-edge.stdout" >/dev/null
grep -F "gitops_beta_edge_install_unit_target=/etc/systemd/system/fishystuff-beta-edge.service" "${root}/install-edge.stdout" >/dev/null
grep -F "gitops_beta_edge_install_unit_sha256=${unit_sha256}" "${root}/install-edge.stdout" >/dev/null
grep -F "gitops_beta_edge_install_served_proof=${served_proof}" "${root}/install-edge.stdout" >/dev/null
grep -F "gitops_beta_edge_install_served_proof_sha256=${served_proof_sha256}" "${root}/install-edge.stdout" >/dev/null
grep -F "gitops_beta_edge_restart_ok=fishystuff-beta-edge.service" "${root}/install-edge.stdout" >/dev/null
grep -F "local_host_mutation_performed=true" "${root}/install-edge.stdout" >/dev/null
grep -F "remote_deploy_performed=false" "${root}/install-edge.stdout" >/dev/null
grep -F "infrastructure_mutation_performed=false" "${root}/install-edge.stdout" >/dev/null
cmp "${root}/edge-bundle/artifacts/systemd/unit" "${fake_install_root}/etc/systemd/system/fishystuff-beta-edge.service" >/dev/null
grep -F -- "-D -m 0644 ${root}/edge-bundle/artifacts/systemd/unit /etc/systemd/system/fishystuff-beta-edge.service" "$fake_install_log" >/dev/null
grep -Fx "daemon-reload" "$fake_systemctl_log" >/dev/null
grep -Fx "restart fishystuff-beta-edge.service" "$fake_systemctl_log" >/dev/null
grep -Fx "is-active --quiet fishystuff-beta-edge.service" "$fake_systemctl_log" >/dev/null
pass "valid beta edge install gate"

if grep -F "fishystuff-edge.service" "$fake_systemctl_log" "${root}/install-edge.stdout" >/dev/null; then
  printf '[gitops-beta-install-edge-test] beta edge install unexpectedly mentioned the production edge unit\n' >&2
  cat "$fake_systemctl_log" >&2
  cat "${root}/install-edge.stdout" >&2
  exit 1
fi
pass "no production edge unit in beta edge install"

printf '[gitops-beta-install-edge-test] %s checks passed\n' "$pass_count"
