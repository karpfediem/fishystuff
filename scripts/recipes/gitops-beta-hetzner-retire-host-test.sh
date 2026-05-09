#!/usr/bin/env bash
set -euo pipefail

SCRIPT_PATH="$(readlink -f "${BASH_SOURCE[0]}")"
SCRIPT_DIR="$(cd "$(dirname "$SCRIPT_PATH")" && pwd)"
source "${SCRIPT_DIR}/lib/common.sh"

cd "$RECIPE_REPO_ROOT"

pass_count=0

pass() {
  printf '[gitops-beta-hetzner-retire-host-test] pass: %s\n' "$1"
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
    printf '[gitops-beta-hetzner-retire-host-test] expected failure: %s\n' "$name" >&2
    exit 1
  fi
  if ! grep -F "$expected" "$stderr" >/dev/null; then
    printf '[gitops-beta-hetzner-retire-host-test] expected %s stderr to contain %q\n' "$name" "$expected" >&2
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

method=GET
url=""
while (($# > 0)); do
  case "$1" in
    -X)
      method="$2"
      shift 2
      ;;
    -H)
      shift 2
      ;;
    -fsS | -sS)
      shift
      ;;
    https://*)
      url="$1"
      shift
      ;;
    *)
      shift
      ;;
  esac
done

printf '%s %s\n' "$method" "$url" >>"${FISHYSTUFF_FAKE_HETZNER_LOG:?}"

case "$method $url" in
  'GET https://api.hetzner.cloud/v1/servers?per_page=50')
    if [[ -f "${FISHYSTUFF_FAKE_HETZNER_DELETED_MARKER:?}" ]]; then
      cat "${FISHYSTUFF_FAKE_HETZNER_ACTIVE_ONLY_JSON:?}"
    else
      cat "${FISHYSTUFF_FAKE_HETZNER_SERVERS_JSON:?}"
    fi
    ;;
  'DELETE https://api.hetzner.cloud/v1/servers/128075021')
    touch "${FISHYSTUFF_FAKE_HETZNER_DELETED_MARKER:?}"
    jq -cn '{action: {id: 987, command: "delete_server", status: "running"}}'
    ;;
  *)
    printf 'unexpected fake Hetzner request: %s %s\n' "$method" "$url" >&2
    exit 2
    ;;
esac
EOF
  chmod +x "$path"
}

root="$(mktemp -d)"
fake_curl="${root}/curl"
write_fake_curl "$fake_curl"

cat >"${root}/servers.json" <<'EOF'
{
  "servers": [
    {
      "id": 128075021,
      "name": "site-nbg1-beta",
      "status": "running",
      "public_net": { "ipv4": { "ip": "178.104.230.121" } },
      "server_type": { "name": "cx33" },
      "datacenter": { "name": "nbg1-dc3" },
      "image": { "name": "debian-13" },
      "labels": {}
    },
    {
      "id": 130153860,
      "name": "site-nbg1-beta-v2",
      "status": "running",
      "public_net": { "ipv4": { "ip": "49.13.192.24" } },
      "server_type": { "name": "cx33" },
      "datacenter": { "name": "nbg1-dc3" },
      "image": { "name": "debian-13" },
      "labels": {
        "fishystuff.deployment": "beta",
        "fishystuff.role": "resident",
        "fishystuff.gitops_service_set": "true"
      }
    }
  ]
}
EOF

cat >"${root}/active-only.json" <<'EOF'
{
  "servers": [
    {
      "id": 130153860,
      "name": "site-nbg1-beta-v2",
      "status": "running",
      "public_net": { "ipv4": { "ip": "49.13.192.24" } },
      "labels": {
        "fishystuff.deployment": "beta",
        "fishystuff.role": "resident",
        "fishystuff.gitops_service_set": "true"
      }
    }
  ]
}
EOF

cat >"${root}/target-labelled.json" <<'EOF'
{
  "servers": [
    {
      "id": 128075021,
      "name": "site-nbg1-beta",
      "status": "running",
      "public_net": { "ipv4": { "ip": "178.104.230.121" } },
      "labels": {
        "fishystuff.gitops_service_set": "true"
      }
    },
    {
      "id": 130153860,
      "name": "site-nbg1-beta-v2",
      "status": "running",
      "public_net": { "ipv4": { "ip": "49.13.192.24" } },
      "labels": {
        "fishystuff.deployment": "beta",
        "fishystuff.role": "resident",
        "fishystuff.gitops_service_set": "true"
      }
    }
  ]
}
EOF

base_env=(
  FISHYSTUFF_OPERATOR_SECRETSPEC_PROFILE=beta-deploy
  FISHYSTUFF_GITOPS_ENABLE_BETA_HETZNER_RETIRE=1
  FISHYSTUFF_GITOPS_BETA_HETZNER_RETIRE_SERVER_NAME=site-nbg1-beta
  FISHYSTUFF_GITOPS_BETA_HETZNER_RETIRE_SERVER_ID=128075021
  FISHYSTUFF_GITOPS_BETA_HETZNER_RETIRE_SERVER_IPV4=178.104.230.121
  FISHYSTUFF_GITOPS_BETA_HETZNER_ACTIVE_SERVER_IPV4=49.13.192.24
  FISHYSTUFF_FAKE_HETZNER_SERVERS_JSON="${root}/servers.json"
  FISHYSTUFF_FAKE_HETZNER_ACTIVE_ONLY_JSON="${root}/active-only.json"
  FISHYSTUFF_FAKE_HETZNER_DELETED_MARKER="${root}/deleted"
  FISHYSTUFF_FAKE_HETZNER_LOG="${root}/hetzner.log"
  HETZNER_API_TOKEN=fake-token
)

env \
  "${base_env[@]}" \
  bash scripts/recipes/gitops-beta-hetzner-retire-host.sh \
    site-nbg1-beta \
    128075021 \
    178.104.230.121 \
    site-nbg1-beta-v2 \
    49.13.192.24 \
    "$fake_curl" >"${root}/retire.out"
grep -F "gitops_beta_hetzner_retire_host_ok=true" "${root}/retire.out" >/dev/null
grep -F "retire_status=deleted" "${root}/retire.out" >/dev/null
grep -F "retire_server_id=128075021" "${root}/retire.out" >/dev/null
grep -F "active_server_ipv4=49.13.192.24" "${root}/retire.out" >/dev/null
grep -F "infrastructure_mutation_performed=true" "${root}/retire.out" >/dev/null
grep -F "production_mutation_performed=false" "${root}/retire.out" >/dev/null
grep -F "DELETE https://api.hetzner.cloud/v1/servers/128075021" "${root}/hetzner.log" >/dev/null
pass "guarded old beta Hetzner retire"

rm -f "${root}/deleted"

expect_fail_contains \
  "requires retire opt-in" \
  "gitops-beta-hetzner-retire-host requires FISHYSTUFF_GITOPS_ENABLE_BETA_HETZNER_RETIRE=1" \
  env \
    FISHYSTUFF_OPERATOR_SECRETSPEC_PROFILE=beta-deploy \
    HETZNER_API_TOKEN=fake-token \
    bash scripts/recipes/gitops-beta-hetzner-retire-host.sh \
      site-nbg1-beta 128075021 178.104.230.121 site-nbg1-beta-v2 49.13.192.24 "$fake_curl"

expect_fail_contains \
  "rejects production profile" \
  "must not run with production SecretSpec profile active" \
  env \
    "${base_env[@]}" \
    FISHYSTUFF_OPERATOR_SECRETSPEC_PROFILE=production-deploy \
    bash scripts/recipes/gitops-beta-hetzner-retire-host.sh \
      site-nbg1-beta 128075021 178.104.230.121 site-nbg1-beta-v2 49.13.192.24 "$fake_curl"

expect_fail_contains \
  "requires server id acknowledgement" \
  "gitops-beta-hetzner-retire-host requires FISHYSTUFF_GITOPS_BETA_HETZNER_RETIRE_SERVER_ID=128075021" \
  env \
    "${base_env[@]}" \
    FISHYSTUFF_GITOPS_BETA_HETZNER_RETIRE_SERVER_ID=130153860 \
    bash scripts/recipes/gitops-beta-hetzner-retire-host.sh \
      site-nbg1-beta 128075021 178.104.230.121 site-nbg1-beta-v2 49.13.192.24 "$fake_curl"

expect_fail_contains \
  "refuses active service-set label" \
  "refusing to retire a server labelled as the GitOps service set" \
  env \
    "${base_env[@]}" \
    FISHYSTUFF_FAKE_HETZNER_SERVERS_JSON="${root}/target-labelled.json" \
    bash scripts/recipes/gitops-beta-hetzner-retire-host.sh \
      site-nbg1-beta 128075021 178.104.230.121 site-nbg1-beta-v2 49.13.192.24 "$fake_curl"

printf '[gitops-beta-hetzner-retire-host-test] %s checks passed\n' "$pass_count"
