#!/usr/bin/env bash
set -euo pipefail

SCRIPT_PATH="$(readlink -f "${BASH_SOURCE[0]}")"
SCRIPT_DIR="$(cd "$(dirname "$SCRIPT_PATH")" && pwd)"
source "${SCRIPT_DIR}/lib/common.sh"

cd "$RECIPE_REPO_ROOT"

pass_count=0

pass() {
  printf '[gitops-beta-host-replacement-plan-test] pass: %s\n' "$1"
  pass_count="$((pass_count + 1))"
}

write_fake_curl() {
  local path="$1"

  cat >"$path" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
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

FISHYSTUFF_FAKE_HETZNER_SERVERS_JSON="${root}/old-only.json" \
  HETZNER_API_TOKEN=fake-token \
  bash scripts/recipes/gitops-beta-host-replacement-plan.sh >"${root}/old-only.out"
grep -F "inventory_status=ready" "${root}/old-only.out" >/dev/null
grep -F "old_server_status=present" "${root}/old-only.out" >/dev/null
grep -F "replacement_server_status=missing" "${root}/old-only.out" >/dev/null
grep -F "next_required_action=create_replacement_beta_host_after_confirmation" "${root}/old-only.out" >/dev/null
grep -F "guarded_create_command_01=FISHYSTUFF_OPERATOR_SECRETSPEC_PROFILE=beta-deploy FISHYSTUFF_GITOPS_ENABLE_BETA_HETZNER_CREATE=1 FISHYSTUFF_GITOPS_BETA_HETZNER_CREATE_SERVER_NAME=site-nbg1-beta-v2" "${root}/old-only.out" >/dev/null
grep -F "hcloud_create_command_emitted=false" "${root}/old-only.out" >/dev/null
grep -F "hetzner_api_create_command_available=true" "${root}/old-only.out" >/dev/null
grep -F "hcloud_delete_command_emitted=false" "${root}/old-only.out" >/dev/null
pass "old-only replacement plan"

cat >"${root}/both.json" <<'EOF'
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
    },
    {
      "id": 11,
      "name": "site-nbg1-beta-v2",
      "status": "running",
      "public_net": { "ipv4": { "ip": "203.0.113.20" } },
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

FISHYSTUFF_FAKE_HETZNER_SERVERS_JSON="${root}/both.json" \
  HETZNER_API_TOKEN=fake-token \
  bash scripts/recipes/gitops-beta-host-replacement-plan.sh >"${root}/both.out"
grep -F "replacement_server_status=present" "${root}/both.out" >/dev/null
grep -F "replacement_server_public_ipv4=203.0.113.20" "${root}/both.out" >/dev/null
grep -F "next_required_action=bootstrap_and_prove_replacement_beta_host" "${root}/both.out" >/dev/null
grep -F "read_only_step_04=just gitops-beta-host-selection-packet public_ipv4=203.0.113.20 host_name=site-nbg1-beta-v2" "${root}/both.out" >/dev/null
pass "replacement-present replacement plan"

bash scripts/recipes/gitops-beta-host-replacement-plan.sh >"${root}/unavailable.out"
grep -F "inventory_status=unavailable" "${root}/unavailable.out" >/dev/null
grep -F "next_required_action=load_beta_deploy_credentials_for_inventory" "${root}/unavailable.out" >/dev/null
grep -F "remote_deploy_performed=false" "${root}/unavailable.out" >/dev/null
pass "credential-unavailable replacement plan"

printf '[gitops-beta-host-replacement-plan-test] %s checks passed\n' "$pass_count"
