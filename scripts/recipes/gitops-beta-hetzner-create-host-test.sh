#!/usr/bin/env bash
set -euo pipefail

SCRIPT_PATH="$(readlink -f "${BASH_SOURCE[0]}")"
SCRIPT_DIR="$(cd "$(dirname "$SCRIPT_PATH")" && pwd)"
source "${SCRIPT_DIR}/lib/common.sh"

cd "$RECIPE_REPO_ROOT"

pass_count=0

pass() {
  printf '[gitops-beta-hetzner-create-host-test] pass: %s\n' "$1"
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
    printf '[gitops-beta-hetzner-create-host-test] expected failure: %s\n' "$name" >&2
    exit 1
  fi
  if ! grep -F "$expected" "$stderr" >/dev/null; then
    printf '[gitops-beta-hetzner-create-host-test] expected %s stderr to contain %q\n' "$name" "$expected" >&2
    cat "$stderr" >&2
    exit 1
  fi
  pass "$name"
}

write_fake_curl() {
  local path="$1"

  cat >"$path" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail

is_post=false
payload=""
while (( $# > 0 )); do
  case "$1" in
    -X)
      shift
      if [[ "${1-}" == "POST" ]]; then
        is_post=true
      fi
      ;;
    -d)
      shift
      payload="${1#@}"
      ;;
  esac
  shift || true
done

if [[ "$is_post" == "true" ]]; then
  cp "$payload" "${FISHYSTUFF_FAKE_HETZNER_CREATE_PAYLOAD:?}"
  cat <<'JSON'
{
  "server": {
    "id": 11,
    "name": "site-nbg1-beta-v2",
    "status": "initializing",
    "public_net": { "ipv4": { "ip": "203.0.113.20" } }
  },
  "action": { "id": 1000, "status": "running" }
}
JSON
  exit 0
fi

cat "${FISHYSTUFF_FAKE_HETZNER_SERVERS_JSON:?}"
EOF
  chmod +x "$path"
}

root="$(mktemp -d)"
fake_bin="${root}/bin"
mkdir -p "$fake_bin"
write_fake_curl "${fake_bin}/curl"
PATH="${fake_bin}:$PATH"

cat >"${root}/old-only.json" <<'EOF'
{
  "servers": [
    {
      "id": 10,
      "name": "site-nbg1-beta",
      "status": "running",
      "public_net": { "ipv4": { "ip": "198.51.100.10" } },
      "server_type": { "name": "cx33" },
      "datacenter": { "name": "nbg1-dc3" },
      "image": { "name": "debian-13" },
      "labels": {
        "fishystuff.deployment": "beta",
        "fishystuff.role": "resident"
      }
    }
  ]
}
EOF

create_payload="${root}/create-payload.json"
env \
  FISHYSTUFF_OPERATOR_SECRETSPEC_PROFILE=beta-deploy \
  FISHYSTUFF_GITOPS_ENABLE_BETA_HETZNER_CREATE=1 \
  FISHYSTUFF_GITOPS_BETA_HETZNER_CREATE_SERVER_NAME=site-nbg1-beta-v2 \
  FISHYSTUFF_FAKE_HETZNER_SERVERS_JSON="${root}/old-only.json" \
  FISHYSTUFF_FAKE_HETZNER_CREATE_PAYLOAD="$create_payload" \
  HETZNER_API_TOKEN=fake-token \
  HETZNER_SSH_KEY_NAME=fishystuff-beta-deploy \
  bash scripts/recipes/gitops-beta-hetzner-create-host.sh >"${root}/created.out"
grep -F "gitops_beta_hetzner_create_host_ok=true" "${root}/created.out" >/dev/null
grep -F "server_name=site-nbg1-beta-v2" "${root}/created.out" >/dev/null
grep -F "resident_hostname=site-nbg1-beta" "${root}/created.out" >/dev/null
grep -F "server_public_ipv4=203.0.113.20" "${root}/created.out" >/dev/null
grep -F "infrastructure_mutation_performed=true" "${root}/created.out" >/dev/null
jq -e '
  .name == "site-nbg1-beta-v2"
  and .server_type == "cx33"
  and .image == "debian-13"
  and .datacenter == "nbg1-dc3"
  and .ssh_keys == ["fishystuff-beta-deploy"]
  and (.user_data | contains("hostname: site-nbg1-beta"))
  and .labels["fishystuff.deployment"] == "beta"
  and .labels["fishystuff.role"] == "resident"
' "$create_payload" >/dev/null
pass "guarded beta Hetzner create"

expect_fail_contains \
  "reject missing opt-in" \
  "requires FISHYSTUFF_GITOPS_ENABLE_BETA_HETZNER_CREATE=1" \
  env \
    FISHYSTUFF_OPERATOR_SECRETSPEC_PROFILE=beta-deploy \
    FISHYSTUFF_GITOPS_BETA_HETZNER_CREATE_SERVER_NAME=site-nbg1-beta-v2 \
    FISHYSTUFF_FAKE_HETZNER_SERVERS_JSON="${root}/old-only.json" \
    FISHYSTUFF_FAKE_HETZNER_CREATE_PAYLOAD="${root}/unused.json" \
    HETZNER_API_TOKEN=fake-token \
    bash scripts/recipes/gitops-beta-hetzner-create-host.sh

expect_fail_contains \
  "reject old beta server name" \
  "refusing to create replacement with old beta server name" \
  env \
    FISHYSTUFF_OPERATOR_SECRETSPEC_PROFILE=beta-deploy \
    FISHYSTUFF_GITOPS_ENABLE_BETA_HETZNER_CREATE=1 \
    FISHYSTUFF_GITOPS_BETA_HETZNER_CREATE_SERVER_NAME=site-nbg1-beta \
    FISHYSTUFF_FAKE_HETZNER_SERVERS_JSON="${root}/old-only.json" \
    FISHYSTUFF_FAKE_HETZNER_CREATE_PAYLOAD="${root}/unused.json" \
    HETZNER_API_TOKEN=fake-token \
    bash scripts/recipes/gitops-beta-hetzner-create-host.sh site-nbg1-beta

printf '[gitops-beta-hetzner-create-host-test] %s checks passed\n' "$pass_count"
