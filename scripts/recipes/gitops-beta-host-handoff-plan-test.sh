#!/usr/bin/env bash
set -euo pipefail

SCRIPT_PATH="$(readlink -f "${BASH_SOURCE[0]}")"
SCRIPT_DIR="$(cd "$(dirname "$SCRIPT_PATH")" && pwd)"
source "${SCRIPT_DIR}/lib/common.sh"

cd "$RECIPE_REPO_ROOT"

FISHYSTUFF_GITOPS_BETA_ACTIVATION_DRAFT_TEST_SOURCE_ONLY=1
source scripts/recipes/gitops-beta-activation-draft-test.sh
unset FISHYSTUFF_GITOPS_BETA_ACTIVATION_DRAFT_TEST_SOURCE_ONLY

pass_count=0

pass() {
  printf '[gitops-beta-host-handoff-plan-test] pass: %s\n' "$1"
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
    printf '[gitops-beta-host-handoff-plan-test] expected failure: %s\n' "$name" >&2
    exit 1
  fi
  if ! grep -F "$expected" "$stderr" >/dev/null; then
    printf '[gitops-beta-host-handoff-plan-test] expected %s stderr to contain %q\n' "$name" "$expected" >&2
    cat "$stderr" >&2
    exit 1
  fi
  pass "$name"
}

make_beta_edge_bundle() {
  local bundle="$1"
  local caddy_bin_real=""
  local caddyfile_real=""
  local systemd_unit_real=""

  mkdir -p "${bundle}/artifacts/exe" "${bundle}/artifacts/config" "${bundle}/artifacts/systemd"
  cat >"${bundle}/artifacts/exe/main" <<'EOF'
#!/usr/bin/env bash
exit 0
EOF
  chmod +x "${bundle}/artifacts/exe/main"
  cat >"${bundle}/artifacts/config/base" <<'EOF'
{
  auto_https off
  admin 127.0.0.1:2119
}

https://beta.fishystuff.fish {
  tls {$CREDENTIALS_DIRECTORY}/fullchain.pem {$CREDENTIALS_DIRECTORY}/privkey.pem
  root * /var/lib/fishystuff/gitops-beta/served/beta/site
  header Cache-Control "no-store"
  header Cache-Control "public, max-age=31536000, immutable"
}

https://api.beta.fishystuff.fish {
  tls {$CREDENTIALS_DIRECTORY}/fullchain.pem {$CREDENTIALS_DIRECTORY}/privkey.pem
  reverse_proxy 127.0.0.1:18192
}

https://cdn.beta.fishystuff.fish {
  tls {$CREDENTIALS_DIRECTORY}/fullchain.pem {$CREDENTIALS_DIRECTORY}/privkey.pem
  root * /var/lib/fishystuff/gitops-beta/served/beta/cdn
  @runtime_manifest path /map/runtime-manifest.json
  header Cache-Control "no-store"
  header Cache-Control "public, max-age=31536000, immutable"
}

https://telemetry.beta.fishystuff.fish {
  tls {$CREDENTIALS_DIRECTORY}/fullchain.pem {$CREDENTIALS_DIRECTORY}/privkey.pem
  reverse_proxy 127.0.0.1:4820
}
EOF
  caddy_bin_real="$(readlink -f "${bundle}/artifacts/exe/main")"
  caddyfile_real="$(readlink -f "${bundle}/artifacts/config/base")"
  systemd_unit_real="$(readlink -f "${bundle}/artifacts/systemd/unit")"
  cat >"${bundle}/artifacts/systemd/unit" <<EOF
[Unit]
Description=Fishystuff beta public edge
After=network-online.target
Wants=network-online.target fishystuff-beta-api.service fishystuff-beta-vector.service
[Service]
Type=simple
DynamicUser=true
ExecStart=${caddy_bin_real} run --config ${caddyfile_real} --adapter caddyfile
ExecReload=${caddy_bin_real} reload --config ${caddyfile_real} --adapter caddyfile --address 127.0.0.1:2119 --force
Restart=on-failure
RestartSec=5s
LoadCredential=fullchain.pem:/run/fishystuff/beta-edge/tls/fullchain.pem
LoadCredential=privkey.pem:/run/fishystuff/beta-edge/tls/privkey.pem
AmbientCapabilities=CAP_NET_BIND_SERVICE
CapabilityBoundingSet=CAP_NET_BIND_SERVICE
ProtectSystem=strict
[Install]
WantedBy=multi-user.target
EOF
  jq -n \
    --arg caddy_bin "$caddy_bin_real" \
    --arg caddyfile "$caddyfile_real" \
    --arg systemd_unit "$systemd_unit_real" \
    '{
      id: "fishystuff-beta-edge",
      activation: {
        directories: [
          {
            path: "/run/fishystuff/beta-edge/tls",
            create: true
          }
        ],
        requiredPaths: [
          "/var/lib/fishystuff/gitops-beta/served/beta/site",
          "/var/lib/fishystuff/gitops-beta/served/beta/cdn"
        ],
        writablePaths: [],
        writable_paths: []
      },
      artifacts: {
        "exe/main": {
          storePath: $caddy_bin,
          executable: true
        },
        "config/base": {
          storePath: $caddyfile,
          destination: "Caddyfile"
        },
        "systemd/unit": {
          storePath: $systemd_unit,
          destination: "fishystuff-beta-edge.service"
        }
      },
      backends: {
        systemd: {
          daemon_reload: true,
          units: [
            {
              name: "fishystuff-beta-edge.service",
              install_path: "/etc/systemd/system/fishystuff-beta-edge.service",
              state: "running",
              startup: "enabled"
            }
          ]
        }
      },
      runtimeOverlays: [
        {
          targetPath: "/run/fishystuff/beta-edge/tls/fullchain.pem",
          required: true,
          secret: false,
          onChange: "restart"
        },
        {
          targetPath: "/run/fishystuff/beta-edge/tls/privkey.pem",
          required: true,
          secret: true,
          onChange: "restart"
        }
      ],
      supervision: {
        argv: [$caddy_bin, "run", "--config", $caddyfile, "--adapter", "caddyfile"],
        reload: {
          mode: "command",
          argv: [$caddy_bin, "reload", "--config", $caddyfile, "--adapter", "caddyfile", "--address", "127.0.0.1:2119", "--force"]
        }
      }
    }' >"${bundle}/bundle.json"
}

write_beta_activation_inputs() {
  local root="$1"
  local api_upstream="${2-http://127.0.0.1:18192}"
  local state=""
  local summary=""
  local api_meta=""
  local db_probe=""
  local site_cdn_probe=""
  local admission="${root}/beta-admission.evidence.json"
  local draft="${root}/beta-activation.draft.desired.json"
  local fake_mgmt_marker="${root}/fake-mgmt-state"

  make_fixture "$root"
  make_fake_mgmt "${root}/mgmt"
  make_fake_deploy "${root}/fishystuff_deploy"

  state="$(cat "${root}/state.path")"
  summary="$(cat "${root}/summary.path")"
  api_meta="$(cat "${root}/api-meta.path")"
  db_probe="$(cat "${root}/db-probe.path")"
  site_cdn_probe="$(cat "${root}/site-cdn-probe.path")"
  export FISHYSTUFF_FAKE_MGMT_MARKER="$fake_mgmt_marker"

  bash scripts/recipes/gitops-beta-write-activation-admission-evidence.sh \
    "$admission" \
    "$summary" \
    "$api_upstream" \
    "$api_meta" \
    "$db_probe" \
    "$site_cdn_probe" \
    >"${root}/write-admission.stdout" \
    2>"${root}/write-admission.stderr"

  bash scripts/recipes/gitops-beta-activation-draft.sh \
    "$draft" \
    "$summary" \
    "$admission" \
    "${root}/mgmt" \
    "${root}/fishystuff_deploy" \
    >"${root}/activation.stdout" \
    2>"${root}/activation.stderr"

  if [[ "$(cat "$fake_mgmt_marker")" != "$draft" ]]; then
    printf '[gitops-beta-host-handoff-plan-test] fake mgmt saw wrong state file\n' >&2
    exit 1
  fi

  printf '%s\n' "$draft" >"${root}/draft.path"
  printf '%s\n' "$summary" >"${root}/summary.path"
  printf '%s\n' "$admission" >"${root}/admission.path"
  printf '%s\n' "$state" >"${root}/state.path"
}

if [[ "${FISHYSTUFF_GITOPS_BETA_HOST_HANDOFF_PLAN_TEST_SOURCE_ONLY:-}" == "1" ]]; then
  return 0 2>/dev/null || exit 0
fi

root="$(mktemp -d)"
write_beta_activation_inputs "$root"
make_beta_edge_bundle "${root}/edge-bundle"

draft="$(cat "${root}/draft.path")"
summary="$(cat "${root}/summary.path")"
admission="$(cat "${root}/admission.path")"

bash scripts/recipes/gitops-beta-host-handoff-plan.sh \
  "$draft" \
  "$summary" \
  "$admission" \
  "${root}/edge-bundle" \
  "${root}/fishystuff_deploy" >"${root}/plan.stdout"
read -r unit_sha256 _ < <(sha256sum "${root}/edge-bundle/artifacts/systemd/unit")

grep -F "gitops_host_handoff_plan_ok=${draft}" "${root}/plan.stdout" >/dev/null
grep -F "gitops_beta_host_handoff_plan_ok=${draft}" "${root}/plan.stdout" >/dev/null
grep -F "environment=beta" "${root}/plan.stdout" >/dev/null
grep -F "edge_bundle=${root}/edge-bundle" "${root}/plan.stdout" >/dev/null
grep -F "edge_caddy_validate=true" "${root}/plan.stdout" >/dev/null
grep -F "served_site_link=/var/lib/fishystuff/gitops-beta/served/beta/site" "${root}/plan.stdout" >/dev/null
grep -F "served_cdn_link=/var/lib/fishystuff/gitops-beta/served/beta/cdn" "${root}/plan.stdout" >/dev/null
grep -F "tls_fullchain=/run/fishystuff/beta-edge/tls/fullchain.pem" "${root}/plan.stdout" >/dev/null
grep -F "systemd_unit_install_path=/etc/systemd/system/fishystuff-beta-edge.service" "${root}/plan.stdout" >/dev/null
grep -F "systemd_unit_sha256=${unit_sha256}" "${root}/plan.stdout" >/dev/null
grep -F "read_only_readiness_check_04=just gitops-beta-edge-handoff-bundle bundle=${root}/edge-bundle" "${root}/plan.stdout" >/dev/null
grep -F "beta_apply_gate_available=true" "${root}/plan.stdout" >/dev/null
grep -F "guarded_host_action_01=FISHYSTUFF_GITOPS_ENABLE_BETA_APPLY=1 FISHYSTUFF_GITOPS_ENABLE_LOCAL_APPLY=1 FISHYSTUFF_GITOPS_BETA_APPLY_OPERATOR_PROOF_SHA256=<checked beta operator proof sha256> just gitops-beta-apply-activation-draft draft_file=${draft} summary_file=${summary} admission_file=${admission} proof_file=<checked beta operator proof file>" "${root}/plan.stdout" >/dev/null
grep -F "guarded_host_action_02=FISHYSTUFF_GITOPS_ENABLE_BETA_EDGE_INSTALL=1 FISHYSTUFF_GITOPS_ENABLE_BETA_EDGE_RESTART=1 FISHYSTUFF_GITOPS_BETA_EDGE_SERVED_PROOF_SHA256=<checked beta served proof sha256> FISHYSTUFF_GITOPS_BETA_EDGE_UNIT_SHA256=${unit_sha256} just gitops-beta-install-edge edge_bundle=${root}/edge-bundle proof_dir=data/gitops" "${root}/plan.stdout" >/dev/null
grep -F "planned_host_step_01=FISHYSTUFF_GITOPS_ENABLE_BETA_APPLY=1 FISHYSTUFF_GITOPS_ENABLE_LOCAL_APPLY=1 FISHYSTUFF_GITOPS_BETA_APPLY_OPERATOR_PROOF_SHA256=<checked beta operator proof sha256> just gitops-beta-apply-activation-draft draft_file=${draft} summary_file=${summary} admission_file=${admission} proof_file=<checked beta operator proof file>" "${root}/plan.stdout" >/dev/null
grep -F "planned_host_step_04=just gitops-beta-proof-index proof_dir=data/gitops require_complete=true" "${root}/plan.stdout" >/dev/null
grep -F "planned_host_step_05=FISHYSTUFF_GITOPS_ENABLE_BETA_EDGE_INSTALL=1 FISHYSTUFF_GITOPS_ENABLE_BETA_EDGE_RESTART=1 FISHYSTUFF_GITOPS_BETA_EDGE_SERVED_PROOF_SHA256=<checked beta served proof sha256> FISHYSTUFF_GITOPS_BETA_EDGE_UNIT_SHA256=${unit_sha256} just gitops-beta-install-edge edge_bundle=${root}/edge-bundle proof_dir=data/gitops" "${root}/plan.stdout" >/dev/null
grep -F "remote_deploy_performed=false" "${root}/plan.stdout" >/dev/null
grep -F "infrastructure_mutation_performed=false" "${root}/plan.stdout" >/dev/null
pass "valid beta host handoff plan"

if grep -F "systemctl restart fishystuff-beta-edge.service" "${root}/plan.stdout" >/dev/null; then
  printf '[gitops-beta-host-handoff-plan-test] beta host plan unexpectedly prints raw edge restart\n' >&2
  cat "${root}/plan.stdout" >&2
  exit 1
fi
pass "beta host handoff plan uses guarded edge install"

if grep -F "production" "${root}/plan.stdout" >/dev/null; then
  printf '[gitops-beta-host-handoff-plan-test] beta host plan unexpectedly mentions production\n' >&2
  cat "${root}/plan.stdout" >&2
  exit 1
fi
pass "no production strings in beta host handoff plan"

wrong_upstream="${root}/wrong-upstream"
mkdir -p "$wrong_upstream"
write_beta_activation_inputs "$wrong_upstream" "http://127.0.0.1:18193"
expect_fail_contains \
  "reject API upstream mismatch" \
  "activation draft API upstream does not match edge handoff bundle upstream" \
  bash scripts/recipes/gitops-beta-host-handoff-plan.sh \
    "$(cat "${wrong_upstream}/draft.path")" \
    "$(cat "${wrong_upstream}/summary.path")" \
    "$(cat "${wrong_upstream}/admission.path")" \
    "${root}/edge-bundle" \
    "${wrong_upstream}/fishystuff_deploy"

production_summary="${root}/production-summary.json"
jq '.environment.name = "production"' "$summary" >"$production_summary"
expect_fail_contains \
  "reject production handoff summary" \
  "requires a beta handoff summary" \
  bash scripts/recipes/gitops-beta-host-handoff-plan.sh \
    "$draft" \
    "$production_summary" \
    "$admission" \
    "${root}/edge-bundle" \
    "${root}/fishystuff_deploy"

missing_bundle_metadata="${root}/missing-bundle-metadata"
make_beta_edge_bundle "$missing_bundle_metadata"
jq 'del(.activation.requiredPaths)' "${missing_bundle_metadata}/bundle.json" >"${missing_bundle_metadata}/bundle.json.tmp"
mv "${missing_bundle_metadata}/bundle.json.tmp" "${missing_bundle_metadata}/bundle.json"
expect_fail_contains \
  "reject missing beta bundle required paths" \
  "bundle metadata is missing GitOps site required path" \
  bash scripts/recipes/gitops-beta-host-handoff-plan.sh \
    "$draft" \
    "$summary" \
    "$admission" \
    "$missing_bundle_metadata" \
    "${root}/fishystuff_deploy"

printf '[gitops-beta-host-handoff-plan-test] %s checks passed\n' "$pass_count"
