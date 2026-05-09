#!/usr/bin/env bash
set -euo pipefail

SCRIPT_PATH="$(readlink -f "${BASH_SOURCE[0]}")"
SCRIPT_DIR="$(cd "$(dirname "$SCRIPT_PATH")" && pwd)"
source "${SCRIPT_DIR}/lib/common.sh"

cd "$RECIPE_REPO_ROOT"

pass_count=0

pass() {
  printf '[gitops-beta-remote-start-edge-test] pass: %s\n' "$1"
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
    printf '[gitops-beta-remote-start-edge-test] expected failure: %s\n' "$name" >&2
    exit 1
  fi
  if ! grep -F "$expected" "$stderr" >/dev/null; then
    printf '[gitops-beta-remote-start-edge-test] expected %s stderr to contain %q\n' "$name" "$expected" >&2
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
Description=Fishystuff public edge
After=network-online.target
Wants=network-online.target fishystuff-beta-api.service fishystuff-beta-vector.service
[Service]
Type=simple
DynamicUser=true
ExecStart=${caddy_bin_real} run --config ${caddyfile_real} --adapter caddyfile
ExecReload=${caddy_bin_real} reload --config ${caddyfile_real} --adapter caddyfile --address 127.0.0.1:2119 --force
Restart=on-failure
RestartSec=5s
LoadCredential=fullchain.pem:/var/lib/fishystuff/gitops-beta/tls/live/fullchain.pem
LoadCredential=privkey.pem:/var/lib/fishystuff/gitops-beta/tls/live/privkey.pem
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
            path: "/var/lib/fishystuff/gitops-beta/tls/live",
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
          targetPath: "/var/lib/fishystuff/gitops-beta/tls/live/fullchain.pem",
          required: true,
          secret: false,
          onChange: "restart"
        },
        {
          targetPath: "/var/lib/fishystuff/gitops-beta/tls/live/privkey.pem",
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

root="$(mktemp -d)"
edge_bundle="${root}/edge-bundle"
site_closure="${root}/site"
cdn_closure="${root}/cdn"
summary="${root}/beta-current.handoff-summary.json"
fake_push="${root}/push-closure.sh"
fake_ssh="${root}/ssh"
fake_ssh_existing="${root}/ssh-existing"
fake_scp="${root}/scp"

make_beta_edge_bundle "$edge_bundle"
mkdir -p "$site_closure" "${cdn_closure}/map"
printf '<!doctype html><title>FishyStuff beta</title>\n' >"${site_closure}/index.html"
printf '{"js":"fishystuff_ui_bevy.fixture.js"}\n' >"${cdn_closure}/map/runtime-manifest.json"
read -r edge_unit_sha256 _ < <(sha256sum "${edge_bundle}/artifacts/systemd/unit")

jq -n \
  --arg site_closure "$site_closure" \
  --arg cdn_closure "$cdn_closure" \
  '{
    schema: "fishystuff.gitops.current-handoff.v1",
    cluster: "beta",
    mode: "validate",
    environment: {
      name: "beta"
    },
    active_release: {
      release_id: "release-test",
      git_rev: "git-test",
      dolt_commit: "dolt-test",
      closures: {
        api: "/tmp/api-fixture",
        site: $site_closure,
        cdn_runtime: $cdn_closure,
        dolt_service: "/tmp/dolt-fixture"
      }
    },
    checks: {
      closure_paths_verified: true,
      gitops_unify_passed: true,
      remote_deploy_performed: false,
      infrastructure_mutation_performed: false
    }
  }' >"$summary"

cat >"$fake_push" <<'PUSH'
#!/usr/bin/env bash
set -euo pipefail
printf '%s\n' "$*" >>"${FISHYSTUFF_FAKE_PUSH_LOG:?}"
PUSH
chmod +x "$fake_push"

cat >"$fake_scp" <<'SCP'
#!/usr/bin/env bash
set -euo pipefail
printf '%s\n' "$*" >>"${FISHYSTUFF_FAKE_SCP_LOG:?}"
SCP
chmod +x "$fake_scp"

cat >"$fake_ssh" <<'SSH'
#!/usr/bin/env bash
set -euo pipefail
printf '%s\n' "$*" >>"${FISHYSTUFF_FAKE_REMOTE_LOG:?}"
cat >"${FISHYSTUFF_FAKE_REMOTE_STDIN:?}"
printf 'remote_hostname=site-nbg1-beta\n'
printf 'remote_edge_served_links_ok=true\n'
printf 'remote_edge_placeholder_tls_installed=true\n'
printf 'remote_edge_service_install_ok=fishystuff-beta-edge.service\n'
printf 'remote_edge_service_restart_ok=fishystuff-beta-edge.service\n'
printf 'remote_edge_api_meta_contains_dolt_commit=true\n'
printf 'remote_host_mutation_performed=true\n'
printf 'remote_deploy_performed=false\n'
printf 'infrastructure_mutation_performed=false\n'
printf 'local_host_mutation_performed=false\n'
SSH
chmod +x "$fake_ssh"

cat >"$fake_ssh_existing" <<'SSH'
#!/usr/bin/env bash
set -euo pipefail
printf '%s\n' "$*" >>"${FISHYSTUFF_FAKE_REMOTE_LOG:?}"
cat >"${FISHYSTUFF_FAKE_REMOTE_STDIN:?}"
printf 'remote_hostname=site-nbg1-beta\n'
printf 'remote_edge_served_links_ok=true\n'
printf 'remote_edge_existing_tls_preserved=true\n'
printf 'remote_edge_service_install_ok=fishystuff-beta-edge.service\n'
printf 'remote_edge_service_restart_ok=fishystuff-beta-edge.service\n'
printf 'remote_edge_api_meta_contains_dolt_commit=true\n'
printf 'remote_host_mutation_performed=true\n'
printf 'remote_deploy_performed=false\n'
printf 'infrastructure_mutation_performed=false\n'
printf 'local_host_mutation_performed=false\n'
SSH
chmod +x "$fake_ssh_existing"

env \
  FISHYSTUFF_OPERATOR_SECRETSPEC_PROFILE=beta-deploy \
  FISHYSTUFF_GITOPS_ENABLE_BETA_REMOTE_EDGE_START=1 \
  FISHYSTUFF_GITOPS_ENABLE_BETA_REMOTE_EDGE_CLOSURE_COPY=1 \
  FISHYSTUFF_GITOPS_ENABLE_BETA_REMOTE_EDGE_SERVED_LINKS=1 \
  FISHYSTUFF_GITOPS_ENABLE_BETA_REMOTE_EDGE_PLACEHOLDER_TLS=1 \
  FISHYSTUFF_GITOPS_ENABLE_BETA_REMOTE_EDGE_INSTALL=1 \
  FISHYSTUFF_GITOPS_ENABLE_BETA_REMOTE_EDGE_RESTART=1 \
  FISHYSTUFF_GITOPS_BETA_REMOTE_EDGE_TARGET=root@203.0.113.20 \
  FISHYSTUFF_GITOPS_BETA_EDGE_UNIT_SHA256="$edge_unit_sha256" \
  FISHYSTUFF_GITOPS_BETA_REMOTE_EDGE_ALLOW_FIXTURE_PATHS=1 \
  HETZNER_SSH_PRIVATE_KEY='fixture-private-key' \
  FISHYSTUFF_FAKE_PUSH_LOG="${root}/push.log" \
  FISHYSTUFF_FAKE_SCP_LOG="${root}/scp.log" \
  FISHYSTUFF_FAKE_REMOTE_LOG="${root}/remote.log" \
  FISHYSTUFF_FAKE_REMOTE_STDIN="${root}/remote.sh" \
  bash scripts/recipes/gitops-beta-remote-start-edge.sh root@203.0.113.20 site-nbg1-beta "$edge_bundle" "$summary" "$fake_push" "$fake_ssh" "$fake_scp" >"${root}/edge.out"
grep -F "gitops_beta_remote_start_edge_checked=true" "${root}/edge.out" >/dev/null
grep -F "gitops_beta_remote_start_edge_ok=true" "${root}/edge.out" >/dev/null
grep -F "resident_target=root@203.0.113.20" "${root}/edge.out" >/dev/null
grep -F "tls_mode=placeholder_self_signed" "${root}/edge.out" >/dev/null
grep -F "remote_edge_service_restart_ok=fishystuff-beta-edge.service" "${root}/edge.out" >/dev/null
grep -F "root@203.0.113.20 ${edge_bundle}" "${root}/push.log" >/dev/null
grep -F "root@203.0.113.20:/tmp/fishystuff-beta-edge-fullchain.pem" "${root}/scp.log" >/dev/null
grep -F "root@203.0.113.20:/tmp/fishystuff-beta-edge-privkey.pem" "${root}/scp.log" >/dev/null
grep -F "install -d -m 0711 /var/lib/fishystuff/gitops-beta" "${root}/remote.sh" >/dev/null
grep -F "ln -sfn \"\$target_path\" \"\${link_path}.next\"" "${root}/remote.sh" >/dev/null
grep -F "systemctl restart fishystuff-beta-edge.service" "${root}/remote.sh" >/dev/null
grep -F -- "--resolve \"\${host}:443:127.0.0.1\"" "${root}/remote.sh" >/dev/null
grep -F "https://api.beta.fishystuff.fish/api/v1/meta" "${root}/remote.sh" >/dev/null
pass "remote edge start is explicit, beta-targeted, and origin-smoke only"

env \
  FISHYSTUFF_OPERATOR_SECRETSPEC_PROFILE=beta-deploy \
  FISHYSTUFF_GITOPS_ENABLE_BETA_REMOTE_EDGE_START=1 \
  FISHYSTUFF_GITOPS_ENABLE_BETA_REMOTE_EDGE_CLOSURE_COPY=1 \
  FISHYSTUFF_GITOPS_ENABLE_BETA_REMOTE_EDGE_SERVED_LINKS=1 \
  FISHYSTUFF_GITOPS_BETA_REMOTE_EDGE_TLS_MODE=existing \
  FISHYSTUFF_GITOPS_ENABLE_BETA_REMOTE_EDGE_EXISTING_TLS=1 \
  FISHYSTUFF_GITOPS_ENABLE_BETA_REMOTE_EDGE_INSTALL=1 \
  FISHYSTUFF_GITOPS_ENABLE_BETA_REMOTE_EDGE_RESTART=1 \
  FISHYSTUFF_GITOPS_BETA_REMOTE_EDGE_TARGET=root@203.0.113.20 \
  FISHYSTUFF_GITOPS_BETA_EDGE_UNIT_SHA256="$edge_unit_sha256" \
  FISHYSTUFF_GITOPS_BETA_REMOTE_EDGE_ALLOW_FIXTURE_PATHS=1 \
  HETZNER_SSH_PRIVATE_KEY='fixture-private-key' \
  FISHYSTUFF_FAKE_PUSH_LOG="${root}/push-existing.log" \
  FISHYSTUFF_FAKE_SCP_LOG="${root}/scp-existing.log" \
  FISHYSTUFF_FAKE_REMOTE_LOG="${root}/remote-existing.log" \
  FISHYSTUFF_FAKE_REMOTE_STDIN="${root}/remote-existing.sh" \
  bash scripts/recipes/gitops-beta-remote-start-edge.sh root@203.0.113.20 site-nbg1-beta "$edge_bundle" "$summary" "$fake_push" "$fake_ssh_existing" "$fake_scp" >"${root}/edge-existing.out"
grep -F "gitops_beta_remote_start_edge_checked=true" "${root}/edge-existing.out" >/dev/null
grep -F "gitops_beta_remote_start_edge_ok=true" "${root}/edge-existing.out" >/dev/null
grep -F "tls_mode=existing_remote" "${root}/edge-existing.out" >/dev/null
grep -F "remote_edge_existing_tls_preserved=true" "${root}/edge-existing.out" >/dev/null
grep -F "root@203.0.113.20 ${edge_bundle}" "${root}/push-existing.log" >/dev/null
grep -F "existing-tls-fullchain-not-uploaded" "${root}/remote-existing.log" >/dev/null
grep -F "existing-tls-privkey-not-uploaded" "${root}/remote-existing.log" >/dev/null
if [[ -s "${root}/scp-existing.log" ]]; then
  printf '[gitops-beta-remote-start-edge-test] existing TLS mode must not copy placeholder TLS\n' >&2
  cat "${root}/scp-existing.log" >&2
  exit 1
fi
grep -F "validate_existing_tls \"\${edge_tls_dir}/fullchain.pem\" \"\${edge_tls_dir}/privkey.pem\"" "${root}/remote-existing.sh" >/dev/null
grep -F "remote_edge_existing_tls_preserved=true" "${root}/remote-existing.sh" >/dev/null
pass "remote edge start can preserve existing trusted TLS"

base_env=(
  FISHYSTUFF_OPERATOR_SECRETSPEC_PROFILE=beta-deploy
  FISHYSTUFF_GITOPS_ENABLE_BETA_REMOTE_EDGE_START=1
  FISHYSTUFF_GITOPS_ENABLE_BETA_REMOTE_EDGE_CLOSURE_COPY=1
  FISHYSTUFF_GITOPS_ENABLE_BETA_REMOTE_EDGE_SERVED_LINKS=1
  FISHYSTUFF_GITOPS_ENABLE_BETA_REMOTE_EDGE_PLACEHOLDER_TLS=1
  FISHYSTUFF_GITOPS_ENABLE_BETA_REMOTE_EDGE_INSTALL=1
  FISHYSTUFF_GITOPS_ENABLE_BETA_REMOTE_EDGE_RESTART=1
  FISHYSTUFF_GITOPS_BETA_REMOTE_EDGE_TARGET=root@203.0.113.20
  FISHYSTUFF_GITOPS_BETA_EDGE_UNIT_SHA256="$edge_unit_sha256"
  FISHYSTUFF_GITOPS_BETA_REMOTE_EDGE_ALLOW_FIXTURE_PATHS=1
  HETZNER_SSH_PRIVATE_KEY=fixture-private-key
  FISHYSTUFF_FAKE_PUSH_LOG="${root}/push-fail.log"
  FISHYSTUFF_FAKE_SCP_LOG="${root}/scp-fail.log"
  FISHYSTUFF_FAKE_REMOTE_LOG="${root}/remote-fail.log"
  FISHYSTUFF_FAKE_REMOTE_STDIN="${root}/remote-fail.sh"
)

expect_fail_contains \
  "requires sequence opt-in" \
  "gitops-beta-remote-start-edge requires FISHYSTUFF_GITOPS_ENABLE_BETA_REMOTE_EDGE_START=1" \
  env \
    FISHYSTUFF_OPERATOR_SECRETSPEC_PROFILE=beta-deploy \
    HETZNER_SSH_PRIVATE_KEY='fixture-private-key' \
    bash scripts/recipes/gitops-beta-remote-start-edge.sh root@203.0.113.20 site-nbg1-beta "$edge_bundle" "$summary" "$fake_push" "$fake_ssh" "$fake_scp"

expect_fail_contains \
  "requires placeholder TLS opt-in" \
  "gitops-beta-remote-start-edge requires FISHYSTUFF_GITOPS_ENABLE_BETA_REMOTE_EDGE_PLACEHOLDER_TLS=1" \
  env \
    FISHYSTUFF_OPERATOR_SECRETSPEC_PROFILE=beta-deploy \
    FISHYSTUFF_GITOPS_ENABLE_BETA_REMOTE_EDGE_START=1 \
    FISHYSTUFF_GITOPS_ENABLE_BETA_REMOTE_EDGE_CLOSURE_COPY=1 \
    FISHYSTUFF_GITOPS_ENABLE_BETA_REMOTE_EDGE_SERVED_LINKS=1 \
    FISHYSTUFF_GITOPS_ENABLE_BETA_REMOTE_EDGE_INSTALL=1 \
    FISHYSTUFF_GITOPS_ENABLE_BETA_REMOTE_EDGE_RESTART=1 \
    FISHYSTUFF_GITOPS_BETA_REMOTE_EDGE_TARGET=root@203.0.113.20 \
    FISHYSTUFF_GITOPS_BETA_EDGE_UNIT_SHA256="$edge_unit_sha256" \
    FISHYSTUFF_GITOPS_BETA_REMOTE_EDGE_ALLOW_FIXTURE_PATHS=1 \
    HETZNER_SSH_PRIVATE_KEY='fixture-private-key' \
    bash scripts/recipes/gitops-beta-remote-start-edge.sh root@203.0.113.20 site-nbg1-beta "$edge_bundle" "$summary" "$fake_push" "$fake_ssh" "$fake_scp"

expect_fail_contains \
  "requires existing TLS opt-in" \
  "gitops-beta-remote-start-edge requires FISHYSTUFF_GITOPS_ENABLE_BETA_REMOTE_EDGE_EXISTING_TLS=1" \
  env \
    FISHYSTUFF_OPERATOR_SECRETSPEC_PROFILE=beta-deploy \
    FISHYSTUFF_GITOPS_ENABLE_BETA_REMOTE_EDGE_START=1 \
    FISHYSTUFF_GITOPS_ENABLE_BETA_REMOTE_EDGE_CLOSURE_COPY=1 \
    FISHYSTUFF_GITOPS_ENABLE_BETA_REMOTE_EDGE_SERVED_LINKS=1 \
    FISHYSTUFF_GITOPS_BETA_REMOTE_EDGE_TLS_MODE=existing \
    FISHYSTUFF_GITOPS_ENABLE_BETA_REMOTE_EDGE_INSTALL=1 \
    FISHYSTUFF_GITOPS_ENABLE_BETA_REMOTE_EDGE_RESTART=1 \
    FISHYSTUFF_GITOPS_BETA_REMOTE_EDGE_TARGET=root@203.0.113.20 \
    FISHYSTUFF_GITOPS_BETA_EDGE_UNIT_SHA256="$edge_unit_sha256" \
    FISHYSTUFF_GITOPS_BETA_REMOTE_EDGE_ALLOW_FIXTURE_PATHS=1 \
    HETZNER_SSH_PRIVATE_KEY='fixture-private-key' \
    bash scripts/recipes/gitops-beta-remote-start-edge.sh root@203.0.113.20 site-nbg1-beta "$edge_bundle" "$summary" "$fake_push" "$fake_ssh" "$fake_scp"

expect_fail_contains \
  "rejects invalid TLS mode" \
  "FISHYSTUFF_GITOPS_BETA_REMOTE_EDGE_TLS_MODE must be placeholder or existing" \
  env \
    FISHYSTUFF_OPERATOR_SECRETSPEC_PROFILE=beta-deploy \
    FISHYSTUFF_GITOPS_ENABLE_BETA_REMOTE_EDGE_START=1 \
    FISHYSTUFF_GITOPS_ENABLE_BETA_REMOTE_EDGE_CLOSURE_COPY=1 \
    FISHYSTUFF_GITOPS_ENABLE_BETA_REMOTE_EDGE_SERVED_LINKS=1 \
    FISHYSTUFF_GITOPS_BETA_REMOTE_EDGE_TLS_MODE=invalid \
    FISHYSTUFF_GITOPS_ENABLE_BETA_REMOTE_EDGE_INSTALL=1 \
    FISHYSTUFF_GITOPS_ENABLE_BETA_REMOTE_EDGE_RESTART=1 \
    FISHYSTUFF_GITOPS_BETA_REMOTE_EDGE_TARGET=root@203.0.113.20 \
    FISHYSTUFF_GITOPS_BETA_EDGE_UNIT_SHA256="$edge_unit_sha256" \
    FISHYSTUFF_GITOPS_BETA_REMOTE_EDGE_ALLOW_FIXTURE_PATHS=1 \
    HETZNER_SSH_PRIVATE_KEY='fixture-private-key' \
    bash scripts/recipes/gitops-beta-remote-start-edge.sh root@203.0.113.20 site-nbg1-beta "$edge_bundle" "$summary" "$fake_push" "$fake_ssh" "$fake_scp"

expect_fail_contains \
  "requires target acknowledgement" \
  "gitops-beta-remote-start-edge requires FISHYSTUFF_GITOPS_BETA_REMOTE_EDGE_TARGET=root@203.0.113.20" \
  env \
    "${base_env[@]}" \
    FISHYSTUFF_GITOPS_BETA_REMOTE_EDGE_TARGET=root@203.0.113.21 \
    bash scripts/recipes/gitops-beta-remote-start-edge.sh root@203.0.113.20 site-nbg1-beta "$edge_bundle" "$summary" "$fake_push" "$fake_ssh" "$fake_scp"

expect_fail_contains \
  "rejects shifted named edge bundle argument" \
  "expected_hostname must be a hostname, got edge_bundle=${edge_bundle}; pass arguments in order" \
  env \
    "${base_env[@]}" \
    bash scripts/recipes/gitops-beta-remote-start-edge.sh root@203.0.113.20 "edge_bundle=${edge_bundle}" auto "$summary" "$fake_push" "$fake_ssh" "$fake_scp"

expect_fail_contains \
  "rejects production profile" \
  "must not run with production SecretSpec profile active" \
  env \
    "${base_env[@]}" \
    FISHYSTUFF_OPERATOR_SECRETSPEC_PROFILE=production-deploy \
    bash scripts/recipes/gitops-beta-remote-start-edge.sh root@203.0.113.20 site-nbg1-beta "$edge_bundle" "$summary" "$fake_push" "$fake_ssh" "$fake_scp"

expect_fail_contains \
  "rejects dns target" \
  "target host must be an IPv4 address" \
  env \
    "${base_env[@]}" \
    FISHYSTUFF_GITOPS_BETA_REMOTE_EDGE_TARGET=root@beta.fishystuff.fish \
    bash scripts/recipes/gitops-beta-remote-start-edge.sh root@beta.fishystuff.fish site-nbg1-beta "$edge_bundle" "$summary" "$fake_push" "$fake_ssh" "$fake_scp"

expect_fail_contains \
  "rejects previous beta host" \
  "target points at the previous beta host" \
  env \
    "${base_env[@]}" \
    FISHYSTUFF_GITOPS_BETA_REMOTE_EDGE_TARGET=root@178.104.230.121 \
    bash scripts/recipes/gitops-beta-remote-start-edge.sh root@178.104.230.121 site-nbg1-beta "$edge_bundle" "$summary" "$fake_push" "$fake_ssh" "$fake_scp"

expect_fail_contains \
  "rejects stale reviewed edge hash" \
  "FISHYSTUFF_GITOPS_BETA_EDGE_UNIT_SHA256 does not match checked beta edge unit" \
  env \
    "${base_env[@]}" \
    FISHYSTUFF_GITOPS_BETA_EDGE_UNIT_SHA256=wrong \
    bash scripts/recipes/gitops-beta-remote-start-edge.sh root@203.0.113.20 site-nbg1-beta "$edge_bundle" "$summary" "$fake_push" "$fake_ssh" "$fake_scp"

printf '[gitops-beta-remote-start-edge-test] %s checks passed\n' "$pass_count"
