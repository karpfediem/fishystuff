#!/usr/bin/env bash
set -euo pipefail

SCRIPT_PATH="$(readlink -f "${BASH_SOURCE[0]}")"
SCRIPT_DIR="$(cd "$(dirname "$SCRIPT_PATH")" && pwd)"
source "${SCRIPT_DIR}/lib/common.sh"

cd "$RECIPE_REPO_ROOT"

pass_count=0

pass() {
  printf '[gitops-beta-remote-install-nix-test] pass: %s\n' "$1"
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
    printf '[gitops-beta-remote-install-nix-test] expected failure: %s\n' "$name" >&2
    exit 1
  fi
  if ! grep -F "$expected" "$stderr" >/dev/null; then
    printf '[gitops-beta-remote-install-nix-test] expected %s stderr to contain %q\n' "$name" "$expected" >&2
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
printf 'remote_hostname=site-nbg1-beta\n'
printf 'remote_nix_install_action=installed\n'
printf 'nix_path=/nix/var/nix/profiles/default/bin/nix\n'
printf 'nix_version=nix (Nix) fixture\n'
printf 'nix_daemon_path=/nix/var/nix/profiles/default/bin/nix-daemon\n'
printf 'nix_daemon_service_state=active\n'
printf 'nix_daemon_socket_state=active\n'
printf 'remote_host_mutation_performed=true\n'
printf 'remote_deploy_performed=false\n'
printf 'infrastructure_mutation_performed=false\n'
printf 'local_host_mutation_performed=false\n'
FAKE
chmod +x "$fake_ssh"

env \
  FISHYSTUFF_OPERATOR_SECRETSPEC_PROFILE=beta-deploy \
  FISHYSTUFF_GITOPS_ENABLE_BETA_REMOTE_NIX_INSTALL=1 \
  FISHYSTUFF_GITOPS_ENABLE_BETA_REMOTE_NIX_APT_PREREQS=1 \
  HETZNER_SSH_PRIVATE_KEY='fixture-private-key' \
  FISHYSTUFF_FAKE_SSH_LOG="${root}/ssh.log" \
  FISHYSTUFF_FAKE_SSH_STDIN="${root}/remote.sh" \
  bash scripts/recipes/gitops-beta-remote-install-nix.sh root@203.0.113.20 site-nbg1-beta "$fake_ssh" >"${root}/install.out"
grep -F "gitops_beta_remote_install_nix_ok=true" "${root}/install.out" >/dev/null
grep -F "resident_target=root@203.0.113.20" "${root}/install.out" >/dev/null
grep -F "remote_nix_install_action=installed" "${root}/install.out" >/dev/null
grep -F "remote_host_mutation_performed=true" "${root}/install.out" >/dev/null
grep -F "root@203.0.113.20" "${root}/ssh.log" >/dev/null
grep -F "apt-get install -y --no-install-recommends ca-certificates curl xz-utils" "${root}/remote.sh" >/dev/null
grep -F "https://nixos.org/nix/install" "${root}/remote.sh" >/dev/null
grep -F 'sh "$installer" --daemon --yes' "${root}/remote.sh" >/dev/null
pass "remote Nix install is explicit and beta-targeted"

expect_fail_contains \
  "requires install opt-in" \
  "gitops-beta-remote-install-nix requires FISHYSTUFF_GITOPS_ENABLE_BETA_REMOTE_NIX_INSTALL=1" \
  env FISHYSTUFF_OPERATOR_SECRETSPEC_PROFILE=beta-deploy HETZNER_SSH_PRIVATE_KEY='fixture-private-key' \
    bash scripts/recipes/gitops-beta-remote-install-nix.sh root@203.0.113.20 site-nbg1-beta "$fake_ssh"

expect_fail_contains \
  "requires apt prereq opt-in" \
  "gitops-beta-remote-install-nix requires FISHYSTUFF_GITOPS_ENABLE_BETA_REMOTE_NIX_APT_PREREQS=1" \
  env \
    FISHYSTUFF_OPERATOR_SECRETSPEC_PROFILE=beta-deploy \
    FISHYSTUFF_GITOPS_ENABLE_BETA_REMOTE_NIX_INSTALL=1 \
    HETZNER_SSH_PRIVATE_KEY='fixture-private-key' \
    bash scripts/recipes/gitops-beta-remote-install-nix.sh root@203.0.113.20 site-nbg1-beta "$fake_ssh"

expect_fail_contains \
  "rejects production profile" \
  "must not run with production SecretSpec profile active" \
  env \
    FISHYSTUFF_OPERATOR_SECRETSPEC_PROFILE=production-deploy \
    FISHYSTUFF_GITOPS_ENABLE_BETA_REMOTE_NIX_INSTALL=1 \
    FISHYSTUFF_GITOPS_ENABLE_BETA_REMOTE_NIX_APT_PREREQS=1 \
    HETZNER_SSH_PRIVATE_KEY='fixture-private-key' \
    bash scripts/recipes/gitops-beta-remote-install-nix.sh root@203.0.113.20 site-nbg1-beta "$fake_ssh"

expect_fail_contains \
  "rejects dns target" \
  "target host must be an IPv4 address" \
  env \
    FISHYSTUFF_OPERATOR_SECRETSPEC_PROFILE=beta-deploy \
    FISHYSTUFF_GITOPS_ENABLE_BETA_REMOTE_NIX_INSTALL=1 \
    FISHYSTUFF_GITOPS_ENABLE_BETA_REMOTE_NIX_APT_PREREQS=1 \
    HETZNER_SSH_PRIVATE_KEY='fixture-private-key' \
    bash scripts/recipes/gitops-beta-remote-install-nix.sh root@beta.fishystuff.fish site-nbg1-beta "$fake_ssh"

printf '[gitops-beta-remote-install-nix-test] %s checks passed\n' "$pass_count"
