#!/usr/bin/env bash
set -euo pipefail

SCRIPT_PATH="$(readlink -f "${BASH_SOURCE[0]}")"
SCRIPT_DIR="$(cd "$(dirname "$SCRIPT_PATH")" && pwd)"
source "${SCRIPT_DIR}/lib/common.sh"

cd "$RECIPE_REPO_ROOT"

pass_count=0

pass() {
  printf '[gitops-beta-remote-host-test] pass: %s\n' "$1"
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
    printf '[gitops-beta-remote-host-test] expected failure: %s\n' "$name" >&2
    exit 1
  fi
  if ! grep -F "$expected" "$stderr" >/dev/null; then
    printf '[gitops-beta-remote-host-test] expected %s stderr to contain %q\n' "$name" "$expected" >&2
    cat "$stderr" >&2
    exit 1
  fi
  pass "$name"
}

root="$(mktemp -d)"
fake_ssh="${root}/ssh"

cat >"$fake_ssh" <<'FAKE'
#!/usr/bin/env bash
set -euo pipefail
printf '%s\n' "$*" >>"${FISHYSTUFF_FAKE_SSH_LOG:?}"
cat >"${FISHYSTUFF_FAKE_SSH_STDIN:?}"
case "${FISHYSTUFF_FAKE_SSH_MODE:?}" in
  preflight)
    printf 'remote_hostname=site-nbg1-beta\n'
    printf 'expected_hostname_match=true\n'
    printf 'os_id=debian\n'
    printf 'os_version_id=13\n'
    printf 'systemd_available=true\n'
    printf 'systemd_state=running\n'
    printf 'nix_available=false\n'
    printf 'nix_path=\n'
    printf 'nix_daemon_available=false\n'
    printf 'nix_daemon_path=\n'
    printf 'beta_group_exists=false\n'
    printf 'beta_user_exists=false\n'
    printf 'beta_directory_01_exists=false\n'
    printf 'beta_directory_02_exists=false\n'
    printf 'beta_directory_03_exists=false\n'
    printf 'beta_directory_04_exists=false\n'
    printf 'beta_directory_05_exists=false\n'
    printf 'beta_directory_06_exists=false\n'
    printf 'beta_directory_07_exists=false\n'
    printf 'beta_directory_08_exists=false\n'
    ;;
  preflight-no-nix-scaffolded)
    printf 'remote_hostname=site-nbg1-beta\n'
    printf 'expected_hostname_match=true\n'
    printf 'os_id=debian\n'
    printf 'os_version_id=13\n'
    printf 'systemd_available=true\n'
    printf 'systemd_state=running\n'
    printf 'nix_available=false\n'
    printf 'nix_path=\n'
    printf 'nix_daemon_available=false\n'
    printf 'nix_daemon_path=\n'
    printf 'beta_group_exists=true\n'
    printf 'beta_user_exists=true\n'
    printf 'beta_directory_01_exists=true\n'
    printf 'beta_directory_02_exists=true\n'
    printf 'beta_directory_03_exists=true\n'
    printf 'beta_directory_04_exists=true\n'
    printf 'beta_directory_05_exists=true\n'
    printf 'beta_directory_06_exists=true\n'
    printf 'beta_directory_07_exists=true\n'
    printf 'beta_directory_08_exists=true\n'
    ;;
  bootstrap)
    printf 'remote_hostname=site-nbg1-beta\n'
    printf 'expected_hostname_match=true\n'
    printf 'beta_group=fishystuff-beta-dolt\n'
    printf 'beta_group_action=created\n'
    printf 'beta_user=fishystuff-beta-dolt\n'
    printf 'beta_user_action=created\n'
    printf 'beta_directory_01=0750:/var/lib/fishystuff/gitops-beta\n'
    printf 'beta_directory_02=0750:/var/lib/fishystuff/gitops-beta/api\n'
    printf 'beta_directory_03=0750:/var/lib/fishystuff/gitops-beta/dolt\n'
    printf 'beta_directory_04=0750:/var/lib/fishystuff/gitops-beta/dolt-cache/fishystuff\n'
    printf 'beta_directory_05=0755:/var/lib/fishystuff/gitops-beta/served/beta\n'
    printf 'beta_directory_06=0750:/run/fishystuff/gitops-beta\n'
    printf 'beta_directory_07=0700:/run/fishystuff/beta-edge/tls\n'
    printf 'beta_directory_08=0750:/var/lib/fishystuff/beta-dolt\n'
    ;;
  *)
    printf 'unsupported fake ssh mode: %s\n' "$FISHYSTUFF_FAKE_SSH_MODE" >&2
    exit 2
    ;;
esac
FAKE
chmod +x "$fake_ssh"

env \
  FISHYSTUFF_OPERATOR_SECRETSPEC_PROFILE=beta-deploy \
  HETZNER_SSH_PRIVATE_KEY='fixture-private-key' \
  FISHYSTUFF_FAKE_SSH_MODE=preflight \
  FISHYSTUFF_FAKE_SSH_LOG="${root}/preflight-ssh.log" \
  FISHYSTUFF_FAKE_SSH_STDIN="${root}/preflight-remote.sh" \
  bash scripts/recipes/gitops-beta-remote-host-preflight.sh root@203.0.113.20 site-nbg1-beta "$fake_ssh" >"${root}/preflight.out"
grep -F "gitops_beta_remote_host_preflight_ok=true" "${root}/preflight.out" >/dev/null
grep -F "resident_target=root@203.0.113.20" "${root}/preflight.out" >/dev/null
grep -F "remote_hostname=site-nbg1-beta" "${root}/preflight.out" >/dev/null
grep -F "nix_available=false" "${root}/preflight.out" >/dev/null
grep -F "remote_host_mutation_performed=false" "${root}/preflight.out" >/dev/null
grep -F "next_required_action=bootstrap_remote_beta_host" "${root}/preflight.out" >/dev/null
grep -F "next_command_01=FISHYSTUFF_OPERATOR_SECRETSPEC_PROFILE=beta-deploy FISHYSTUFF_GITOPS_ENABLE_BETA_REMOTE_BOOTSTRAP=1" "${root}/preflight.out" >/dev/null
grep -F "just gitops-beta-remote-host-bootstrap target=root@203.0.113.20" "${root}/preflight.out" >/dev/null
grep -F "root@203.0.113.20" "${root}/preflight-ssh.log" >/dev/null
grep -F "/var/lib/fishystuff/gitops-beta" "${root}/preflight-remote.sh" >/dev/null
pass "remote host preflight uses beta target and reports no mutation"

env \
  FISHYSTUFF_OPERATOR_SECRETSPEC_PROFILE=beta-deploy \
  HETZNER_SSH_PRIVATE_KEY='fixture-private-key' \
  FISHYSTUFF_FAKE_SSH_MODE=preflight-no-nix-scaffolded \
  FISHYSTUFF_FAKE_SSH_LOG="${root}/preflight-no-nix-ssh.log" \
  FISHYSTUFF_FAKE_SSH_STDIN="${root}/preflight-no-nix-remote.sh" \
  bash scripts/recipes/gitops-beta-remote-host-preflight.sh root@203.0.113.23 site-nbg1-beta "$fake_ssh" >"${root}/preflight-no-nix.out"
grep -F "next_required_action=install_remote_nix" "${root}/preflight-no-nix.out" >/dev/null
grep -F "next_note_01=Nix and nix-daemon must exist before beta closure transfer can use nix copy" "${root}/preflight-no-nix.out" >/dev/null
grep -F "remote_host_mutation_performed=false" "${root}/preflight-no-nix.out" >/dev/null
pass "remote host preflight distinguishes missing Nix after scaffold"

env \
  FISHYSTUFF_OPERATOR_SECRETSPEC_PROFILE=beta-deploy \
  FISHYSTUFF_BETA_RESIDENT_TARGET=root@203.0.113.21 \
  HETZNER_SSH_PRIVATE_KEY='fixture-private-key' \
  FISHYSTUFF_FAKE_SSH_MODE=preflight \
  FISHYSTUFF_FAKE_SSH_LOG="${root}/preflight-env-ssh.log" \
  FISHYSTUFF_FAKE_SSH_STDIN="${root}/preflight-env-remote.sh" \
  FISHYSTUFF_GITOPS_SSH_BIN="$fake_ssh" \
  bash scripts/recipes/gitops-beta-remote-host-preflight.sh "" "" "" >"${root}/preflight-env.out"
grep -F "resident_target=root@203.0.113.21" "${root}/preflight-env.out" >/dev/null
grep -F "root@203.0.113.21" "${root}/preflight-env-ssh.log" >/dev/null
pass "remote host preflight accepts env target through empty Just defaults"

env \
  FISHYSTUFF_OPERATOR_SECRETSPEC_PROFILE=beta-deploy \
  FISHYSTUFF_GITOPS_ENABLE_BETA_REMOTE_BOOTSTRAP=1 \
  FISHYSTUFF_GITOPS_ENABLE_BETA_REMOTE_DIRECTORIES=1 \
  FISHYSTUFF_GITOPS_ENABLE_BETA_REMOTE_USER_GROUPS=1 \
  HETZNER_SSH_PRIVATE_KEY='fixture-private-key' \
  FISHYSTUFF_FAKE_SSH_MODE=bootstrap \
  FISHYSTUFF_FAKE_SSH_LOG="${root}/bootstrap-ssh.log" \
  FISHYSTUFF_FAKE_SSH_STDIN="${root}/bootstrap-remote.sh" \
  bash scripts/recipes/gitops-beta-remote-host-bootstrap.sh root@203.0.113.20 site-nbg1-beta "$fake_ssh" >"${root}/bootstrap.out"
grep -F "gitops_beta_remote_host_bootstrap_ok=true" "${root}/bootstrap.out" >/dev/null
grep -F "resident_target=root@203.0.113.20" "${root}/bootstrap.out" >/dev/null
grep -F "beta_group_action=created" "${root}/bootstrap.out" >/dev/null
grep -F "beta_directory_08=0750:/var/lib/fishystuff/beta-dolt" "${root}/bootstrap.out" >/dev/null
grep -F "remote_host_mutation_performed=true" "${root}/bootstrap.out" >/dev/null
grep -F "remote_deploy_performed=false" "${root}/bootstrap.out" >/dev/null
grep -F "root@203.0.113.20" "${root}/bootstrap-ssh.log" >/dev/null
grep -F "groupadd --system" "${root}/bootstrap-remote.sh" >/dev/null
grep -F "fishystuff-beta-dolt" "${root}/bootstrap-remote.sh" >/dev/null
pass "remote host bootstrap is opt-in and beta-local"

env \
  FISHYSTUFF_OPERATOR_SECRETSPEC_PROFILE=beta-deploy \
  FISHYSTUFF_GITOPS_ENABLE_BETA_REMOTE_BOOTSTRAP=1 \
  FISHYSTUFF_GITOPS_ENABLE_BETA_REMOTE_DIRECTORIES=1 \
  FISHYSTUFF_GITOPS_ENABLE_BETA_REMOTE_USER_GROUPS=1 \
  FISHYSTUFF_BETA_RESIDENT_TARGET=root@203.0.113.22 \
  HETZNER_SSH_PRIVATE_KEY='fixture-private-key' \
  FISHYSTUFF_FAKE_SSH_MODE=bootstrap \
  FISHYSTUFF_FAKE_SSH_LOG="${root}/bootstrap-env-ssh.log" \
  FISHYSTUFF_FAKE_SSH_STDIN="${root}/bootstrap-env-remote.sh" \
  FISHYSTUFF_GITOPS_SSH_BIN="$fake_ssh" \
  bash scripts/recipes/gitops-beta-remote-host-bootstrap.sh "" "" "" >"${root}/bootstrap-env.out"
grep -F "resident_target=root@203.0.113.22" "${root}/bootstrap-env.out" >/dev/null
grep -F "root@203.0.113.22" "${root}/bootstrap-env-ssh.log" >/dev/null
pass "remote host bootstrap accepts env target through empty Just defaults"

expect_fail_contains \
  "preflight requires explicit target" \
  "target is required" \
  env FISHYSTUFF_OPERATOR_SECRETSPEC_PROFILE=beta-deploy HETZNER_SSH_PRIVATE_KEY='fixture-private-key' \
    bash scripts/recipes/gitops-beta-remote-host-preflight.sh

expect_fail_contains \
  "preflight rejects production profile" \
  "must not run with production SecretSpec profile active" \
  env FISHYSTUFF_OPERATOR_SECRETSPEC_PROFILE=production-deploy HETZNER_SSH_PRIVATE_KEY='fixture-private-key' \
    bash scripts/recipes/gitops-beta-remote-host-preflight.sh root@203.0.113.20

expect_fail_contains \
  "preflight rejects dns target" \
  "target host must be an IPv4 address" \
  env FISHYSTUFF_OPERATOR_SECRETSPEC_PROFILE=beta-deploy HETZNER_SSH_PRIVATE_KEY='fixture-private-key' \
    bash scripts/recipes/gitops-beta-remote-host-preflight.sh root@beta.fishystuff.fish

expect_fail_contains \
  "bootstrap requires opt-in" \
  "gitops-beta-remote-host-bootstrap requires FISHYSTUFF_GITOPS_ENABLE_BETA_REMOTE_BOOTSTRAP=1" \
  env FISHYSTUFF_OPERATOR_SECRETSPEC_PROFILE=beta-deploy HETZNER_SSH_PRIVATE_KEY='fixture-private-key' \
    bash scripts/recipes/gitops-beta-remote-host-bootstrap.sh root@203.0.113.20

expect_fail_contains \
  "bootstrap rejects previous beta host" \
  "target points at the previous beta host" \
  env \
    FISHYSTUFF_OPERATOR_SECRETSPEC_PROFILE=beta-deploy \
    FISHYSTUFF_GITOPS_ENABLE_BETA_REMOTE_BOOTSTRAP=1 \
    FISHYSTUFF_GITOPS_ENABLE_BETA_REMOTE_DIRECTORIES=1 \
    FISHYSTUFF_GITOPS_ENABLE_BETA_REMOTE_USER_GROUPS=1 \
    HETZNER_SSH_PRIVATE_KEY='fixture-private-key' \
    bash scripts/recipes/gitops-beta-remote-host-bootstrap.sh root@178.104.230.121

printf '[gitops-beta-remote-host-test] %s checks passed\n' "$pass_count"
