#!/usr/bin/env bash
set -euo pipefail

SCRIPT_PATH="$(readlink -f "${BASH_SOURCE[0]}")"
SCRIPT_DIR="$(cd "$(dirname "$SCRIPT_PATH")" && pwd)"
source "${SCRIPT_DIR}/lib/common.sh"

cd "$RECIPE_REPO_ROOT"

pass_count=0

pass() {
  printf '[gitops-beta-host-bootstrap-apply-test] pass: %s\n' "$1"
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
    printf '[gitops-beta-host-bootstrap-apply-test] expected failure: %s\n' "$name" >&2
    exit 1
  fi
  if ! grep -F "$expected" "$stderr" >/dev/null; then
    printf '[gitops-beta-host-bootstrap-apply-test] expected %s stderr to contain %q\n' "$name" "$expected" >&2
    cat "$stderr" >&2
    exit 1
  fi
  pass "$name"
}

write_fake_install() {
  local path="$1"

  cat >"$path" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail

root="${FISHYSTUFF_FAKE_INSTALL_ROOT:?}"
log="${FISHYSTUFF_FAKE_INSTALL_LOG:?}"
printf '%s\n' "$*" >>"$log"
if [[ "$#" -ne 4 || "$1" != "-d" || "$2" != "-m" ]]; then
  echo "unexpected fake install args: $*" >&2
  exit 2
fi
mode="$3"
target_path="$4"
case "$target_path" in
  /var/lib/fishystuff/gitops-beta | \
  /var/lib/fishystuff/gitops-beta/* | \
  /var/lib/fishystuff/beta-dolt | \
  /run/fishystuff/gitops-beta | \
  /run/fishystuff/gitops-beta/* | \
  /run/fishystuff/beta-edge/tls)
    ;;
  *)
    echo "fake install saw non-beta bootstrap directory: ${target_path}" >&2
    exit 2
    ;;
esac
mkdir -p "${root}${target_path}"
chmod "$mode" "${root}${target_path}"
EOF
  chmod +x "$path"
}

write_fake_getent() {
  local path="$1"

  cat >"$path" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail

log="${FISHYSTUFF_FAKE_GETENT_LOG:?}"
printf '%s\n' "$*" >>"$log"
if [[ "$#" -ne 2 ]]; then
  echo "unexpected fake getent args: $*" >&2
  exit 2
fi
case "$1:$2" in
  group:fishystuff-beta-dolt)
    [[ "${FISHYSTUFF_FAKE_GETENT_GROUP_EXISTS:-0}" == "1" ]]
    ;;
  passwd:fishystuff-beta-dolt)
    [[ "${FISHYSTUFF_FAKE_GETENT_USER_EXISTS:-0}" == "1" ]]
    ;;
  *)
    echo "unexpected fake getent lookup: $*" >&2
    exit 2
    ;;
esac
EOF
  chmod +x "$path"
}

write_fake_groupadd() {
  local path="$1"

  cat >"$path" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail

log="${FISHYSTUFF_FAKE_GROUPADD_LOG:?}"
printf '%s\n' "$*" >>"$log"
if [[ "$#" -ne 2 || "$1" != "--system" || "$2" != "fishystuff-beta-dolt" ]]; then
  echo "unexpected fake groupadd args: $*" >&2
  exit 2
fi
EOF
  chmod +x "$path"
}

write_fake_useradd() {
  local path="$1"

  cat >"$path" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail

log="${FISHYSTUFF_FAKE_USERADD_LOG:?}"
printf '%s\n' "$*" >>"$log"
if [[ "$#" -ne 7 ]]; then
  echo "unexpected fake useradd arg count: $*" >&2
  exit 2
fi
if [[ "$1" != "--system" || "$2" != "--gid" || "$3" != "fishystuff-beta-dolt" ]]; then
  echo "unexpected fake useradd identity args: $*" >&2
  exit 2
fi
if [[ "$4" != "--home-dir" || "$5" != "/var/lib/fishystuff/beta-dolt/home" ]]; then
  echo "unexpected fake useradd home args: $*" >&2
  exit 2
fi
if [[ "$6" != "--no-create-home" || "$7" != "fishystuff-beta-dolt" ]]; then
  echo "unexpected fake useradd tail args: $*" >&2
  exit 2
fi
EOF
  chmod +x "$path"
}

make_fake_commands() {
  local bin_dir="$1"

  mkdir -p "$bin_dir"
  write_fake_install "${bin_dir}/install"
  write_fake_getent "${bin_dir}/getent"
  write_fake_groupadd "${bin_dir}/groupadd"
  write_fake_useradd "${bin_dir}/useradd"
}

expect_fail_contains \
  "reject missing bootstrap opt-in" \
  "gitops-beta-host-bootstrap-apply requires FISHYSTUFF_GITOPS_ENABLE_BETA_HOST_BOOTSTRAP=1" \
  bash scripts/recipes/gitops-beta-host-bootstrap-apply.sh

root="$(mktemp -d)"
make_fake_commands "${root}/bin"
stdout="${root}/stdout"

env \
  FISHYSTUFF_GITOPS_ENABLE_BETA_HOST_BOOTSTRAP=1 \
  FISHYSTUFF_GITOPS_ENABLE_BETA_HOST_DIRECTORIES=1 \
  FISHYSTUFF_GITOPS_ENABLE_BETA_HOST_USER_GROUPS=1 \
  FISHYSTUFF_FAKE_INSTALL_ROOT="${root}/fs" \
  FISHYSTUFF_FAKE_INSTALL_LOG="${root}/install.log" \
  FISHYSTUFF_FAKE_GETENT_LOG="${root}/getent.log" \
  FISHYSTUFF_FAKE_GROUPADD_LOG="${root}/groupadd.log" \
  FISHYSTUFF_FAKE_USERADD_LOG="${root}/useradd.log" \
  bash scripts/recipes/gitops-beta-host-bootstrap-apply.sh \
    "${root}/bin/install" \
    "${root}/bin/groupadd" \
    "${root}/bin/useradd" \
    "${root}/bin/getent" \
  >"$stdout"

grep -F "gitops_beta_host_bootstrap_apply_ok=true" "$stdout" >/dev/null
grep -F "gitops_beta_host_bootstrap_group_action=created" "$stdout" >/dev/null
grep -F "gitops_beta_host_bootstrap_user_action=created" "$stdout" >/dev/null
grep -F "gitops_beta_host_bootstrap_directory_01=0750:/var/lib/fishystuff/gitops-beta" "$stdout" >/dev/null
grep -F "gitops_beta_host_bootstrap_directory_08=0750:/var/lib/fishystuff/beta-dolt" "$stdout" >/dev/null
grep -F "local_host_mutation_performed=true" "$stdout" >/dev/null
grep -F "remote_deploy_performed=false" "$stdout" >/dev/null
grep -F "infrastructure_mutation_performed=false" "$stdout" >/dev/null
grep -F -- "--system fishystuff-beta-dolt" "${root}/groupadd.log" >/dev/null
grep -F -- "--system --gid fishystuff-beta-dolt --home-dir /var/lib/fishystuff/beta-dolt/home --no-create-home fishystuff-beta-dolt" "${root}/useradd.log" >/dev/null
test -d "${root}/fs/var/lib/fishystuff/gitops-beta/api"
test -d "${root}/fs/var/lib/fishystuff/gitops-beta/dolt-cache/fishystuff"
test -d "${root}/fs/run/fishystuff/beta-edge/tls"
pass "create beta bootstrap scaffolding through fake commands"

root_existing="$(mktemp -d)"
make_fake_commands "${root_existing}/bin"
stdout_existing="${root_existing}/stdout"
touch "${root_existing}/groupadd.log" "${root_existing}/useradd.log"

env \
  FISHYSTUFF_GITOPS_ENABLE_BETA_HOST_BOOTSTRAP=1 \
  FISHYSTUFF_GITOPS_ENABLE_BETA_HOST_DIRECTORIES=1 \
  FISHYSTUFF_GITOPS_ENABLE_BETA_HOST_USER_GROUPS=1 \
  FISHYSTUFF_FAKE_GETENT_GROUP_EXISTS=1 \
  FISHYSTUFF_FAKE_GETENT_USER_EXISTS=1 \
  FISHYSTUFF_FAKE_INSTALL_ROOT="${root_existing}/fs" \
  FISHYSTUFF_FAKE_INSTALL_LOG="${root_existing}/install.log" \
  FISHYSTUFF_FAKE_GETENT_LOG="${root_existing}/getent.log" \
  FISHYSTUFF_FAKE_GROUPADD_LOG="${root_existing}/groupadd.log" \
  FISHYSTUFF_FAKE_USERADD_LOG="${root_existing}/useradd.log" \
  bash scripts/recipes/gitops-beta-host-bootstrap-apply.sh \
    "${root_existing}/bin/install" \
    "${root_existing}/bin/groupadd" \
    "${root_existing}/bin/useradd" \
    "${root_existing}/bin/getent" \
  >"$stdout_existing"

grep -F "gitops_beta_host_bootstrap_group_action=existing" "$stdout_existing" >/dev/null
grep -F "gitops_beta_host_bootstrap_user_action=existing" "$stdout_existing" >/dev/null
if [[ -s "${root_existing}/groupadd.log" || -s "${root_existing}/useradd.log" ]]; then
  printf '[gitops-beta-host-bootstrap-apply-test] expected existing user/group run to skip add commands\n' >&2
  exit 1
fi
pass "skip existing beta user and group"

root_reject="$(mktemp -d)"
make_fake_commands "${root_reject}/bin"
expect_fail_contains \
  "reject production SecretSpec profile" \
  "must not run with production SecretSpec profile" \
  env \
    FISHYSTUFF_GITOPS_ENABLE_BETA_HOST_BOOTSTRAP=1 \
    FISHYSTUFF_GITOPS_ENABLE_BETA_HOST_DIRECTORIES=1 \
    FISHYSTUFF_GITOPS_ENABLE_BETA_HOST_USER_GROUPS=1 \
    FISHYSTUFF_OPERATOR_SECRETSPEC_PROFILE=production-deploy \
    FISHYSTUFF_FAKE_INSTALL_ROOT="${root_reject}/fs" \
    FISHYSTUFF_FAKE_INSTALL_LOG="${root_reject}/install.log" \
    FISHYSTUFF_FAKE_GETENT_LOG="${root_reject}/getent.log" \
    FISHYSTUFF_FAKE_GROUPADD_LOG="${root_reject}/groupadd.log" \
    FISHYSTUFF_FAKE_USERADD_LOG="${root_reject}/useradd.log" \
    bash scripts/recipes/gitops-beta-host-bootstrap-apply.sh \
      "${root_reject}/bin/install" \
      "${root_reject}/bin/groupadd" \
      "${root_reject}/bin/useradd" \
      "${root_reject}/bin/getent"

printf '[gitops-beta-host-bootstrap-apply-test] %s checks passed\n' "$pass_count"
