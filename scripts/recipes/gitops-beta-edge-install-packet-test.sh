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
  printf '[gitops-beta-edge-install-packet-test] pass: %s\n' "$1"
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
    printf '[gitops-beta-edge-install-packet-test] expected failure: %s\n' "$name" >&2
    exit 1
  fi
  if ! grep -F "$expected" "$stderr" >/dev/null; then
    printf '[gitops-beta-edge-install-packet-test] expected %s stderr to contain %q\n' "$name" "$expected" >&2
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

root="$(mktemp -d)"
fake_bin="${root}/fake-bin"
mkdir -p "$fake_bin"
cat >"${fake_bin}/hostname" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail

printf 'operator-dev\n'
EOF
chmod +x "${fake_bin}/hostname"
PATH="${fake_bin}:$PATH"
deploy_bin="$(require_deploy_bin)"
write_beta_activation_inputs "$root"
make_beta_edge_bundle "${root}/edge-bundle"
make_fake_served_deploy "${root}/fishystuff_deploy_served" "$deploy_bin"
write_beta_placeholder_tls "${root}/tls"
cp "${root}/edge-bundle/artifacts/systemd/unit" "${root}/fishystuff-beta-edge.service"

draft="$(cat "${root}/draft.path")"
summary="$(cat "${root}/summary.path")"
admission="$(cat "${root}/admission.path")"
release_id="$(jq -er '.environments.beta.active_release' "$draft")"
proof_dir="${root}/proofs"
api_upstream="http://127.0.0.1:18192"

bash scripts/recipes/gitops-beta-edge-install-packet.sh \
  "${root}/edge-bundle" \
  "${root}/missing-proofs" \
  86400 \
  "$draft" \
  "$summary" \
  "$admission" \
  "" \
  "${root}/fishystuff_deploy_served" \
  "${root}/state" \
  "${root}/run" \
  "$api_upstream" \
  "${root}/observations" >"${root}/missing-chain.stdout"

grep -F "gitops_beta_edge_install_packet_ok=true" "${root}/missing-chain.stdout" >/dev/null
grep -F "edge_install_packet_status=missing_complete_proof_chain" "${root}/missing-chain.stdout" >/dev/null
grep -F "edge_install_packet_proof_index_status=missing_proof_dir" "${root}/missing-chain.stdout" >/dev/null
grep -F "edge_install_packet_next_command_01=just gitops-beta-served-proof-packet" "${root}/missing-chain.stdout" >/dev/null
grep -F "remote_deploy_performed=false" "${root}/missing-chain.stdout" >/dev/null
grep -F "infrastructure_mutation_performed=false" "${root}/missing-chain.stdout" >/dev/null
grep -F "local_host_mutation_performed=false" "${root}/missing-chain.stdout" >/dev/null
pass "missing proof chain edge packet"

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
read -r served_proof_sha256 _ < <(sha256sum "$served_proof")
read -r unit_sha256 _ < <(sha256sum "${root}/edge-bundle/artifacts/systemd/unit")

bash scripts/recipes/gitops-beta-edge-install-packet.sh \
  "${root}/edge-bundle" \
  "$proof_dir" \
  86400 \
  "$draft" \
  "$summary" \
  "$admission" \
  "$operator_proof" \
  "${root}/fishystuff_deploy_served" \
  "${root}/state" \
  "${root}/run" \
  "$api_upstream" \
  "${root}/observations" >"${root}/ready.stdout"

grep -F "edge_install_packet_status=ready" "${root}/ready.stdout" >/dev/null
grep -F "edge_install_packet_current_hostname=operator-dev" "${root}/ready.stdout" >/dev/null
grep -F "edge_install_packet_expected_hostname=site-nbg1-beta" "${root}/ready.stdout" >/dev/null
grep -F "edge_install_packet_expected_hostname_match=false" "${root}/ready.stdout" >/dev/null
grep -F "edge_install_packet_resident_target=root@beta.fishystuff.fish" "${root}/ready.stdout" >/dev/null
grep -F "edge_install_packet_proof_index_complete=true" "${root}/ready.stdout" >/dev/null
grep -F "edge_install_packet_served_proof=${served_proof}" "${root}/ready.stdout" >/dev/null
grep -F "edge_install_packet_served_proof_sha256=${served_proof_sha256}" "${root}/ready.stdout" >/dev/null
grep -F "edge_install_packet_unit_name=fishystuff-beta-edge.service" "${root}/ready.stdout" >/dev/null
grep -F "edge_install_packet_unit_target=/etc/systemd/system/fishystuff-beta-edge.service" "${root}/ready.stdout" >/dev/null
grep -F "edge_install_packet_unit_sha256=${unit_sha256}" "${root}/ready.stdout" >/dev/null
grep -F "edge_install_packet_caddy_validate=true" "${root}/ready.stdout" >/dev/null
grep -F "edge_install_packet_next_command_01=FISHYSTUFF_GITOPS_ENABLE_BETA_EDGE_INSTALL=1 FISHYSTUFF_GITOPS_ENABLE_BETA_EDGE_RESTART=1 FISHYSTUFF_GITOPS_BETA_EDGE_SERVED_PROOF_SHA256=${served_proof_sha256} FISHYSTUFF_GITOPS_BETA_EDGE_UNIT_SHA256=${unit_sha256} just gitops-beta-install-edge edge_bundle=${root}/edge-bundle proof_dir=${proof_dir} max_age_seconds=86400" "${root}/ready.stdout" >/dev/null
pass "ready edge packet"

perl -0pi -e 's#/var/lib/fishystuff/gitops-beta/served/beta/site#/var/lib/fishystuff/gitops/served/production/site#g' "${root}/edge-bundle/artifacts/config/base"
expect_fail_contains \
  "reject bad edge bundle" \
  "beta GitOps edge handoff Caddyfile is missing GitOps site root" \
  bash scripts/recipes/gitops-beta-edge-install-packet.sh \
    "${root}/edge-bundle" \
    "$proof_dir" \
    86400 \
    "$draft" \
    "$summary" \
    "$admission" \
    "$operator_proof" \
    "${root}/fishystuff_deploy_served" \
    "${root}/state" \
    "${root}/run" \
    "$api_upstream" \
    "${root}/observations"

printf '[gitops-beta-edge-install-packet-test] %s checks passed\n' "$pass_count"
