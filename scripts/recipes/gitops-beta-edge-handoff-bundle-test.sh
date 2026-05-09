#!/usr/bin/env bash
set -euo pipefail

SCRIPT_PATH="$(readlink -f "${BASH_SOURCE[0]}")"
SCRIPT_DIR="$(cd "$(dirname "$SCRIPT_PATH")" && pwd)"
source "${SCRIPT_DIR}/lib/common.sh"

cd "$RECIPE_REPO_ROOT"

pass_count=0

pass() {
  printf '[gitops-beta-edge-handoff-bundle-test] pass: %s\n' "$1"
  pass_count="$((pass_count + 1))"
}

make_beta_bundle() {
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

expect_fail_contains() {
  local name="$1"
  local expected="$2"
  shift 2
  local root=""
  local stderr=""

  root="$(mktemp -d)"
  stderr="${root}/stderr"
  if "$@" >"${root}/stdout" 2>"$stderr"; then
    printf '[gitops-beta-edge-handoff-bundle-test] expected failure: %s\n' "$name" >&2
    exit 1
  fi
  if ! grep -F "$expected" "$stderr" >/dev/null; then
    printf '[gitops-beta-edge-handoff-bundle-test] expected %s stderr to contain %q\n' "$name" "$expected" >&2
    cat "$stderr" >&2
    exit 1
  fi
  pass "$name"
}

root="$(mktemp -d)"
valid="${root}/valid"
make_beta_bundle "$valid"

bash scripts/recipes/gitops-check-edge-handoff-bundle.sh "$valid" beta >"${root}/valid.stdout"
grep -F "gitops_edge_handoff_environment=beta" "${root}/valid.stdout" >/dev/null
grep -F "gitops_edge_handoff_service_id=fishystuff-beta-edge" "${root}/valid.stdout" >/dev/null
grep -F "gitops_edge_handoff_unit_name=fishystuff-beta-edge.service" "${root}/valid.stdout" >/dev/null
grep -F "gitops_edge_handoff_site_root=/var/lib/fishystuff/gitops-beta/served/beta/site" "${root}/valid.stdout" >/dev/null
grep -F "gitops_edge_handoff_cdn_root=/var/lib/fishystuff/gitops-beta/served/beta/cdn" "${root}/valid.stdout" >/dev/null
grep -F "gitops_edge_handoff_api_upstream=127.0.0.1:18192" "${root}/valid.stdout" >/dev/null
grep -F "gitops_edge_handoff_tls_dir=/var/lib/fishystuff/gitops-beta/tls/live" "${root}/valid.stdout" >/dev/null
pass "valid beta bundle"

prod_hostname="${root}/prod-hostname"
make_beta_bundle "$prod_hostname"
printf '\n# https://fishystuff.fish must never be present here\n' >>"${prod_hostname}/artifacts/config/base"
expect_fail_contains \
  "reject production hostname" \
  "must not contain forbidden beta Caddy fragment: https://fishystuff.fish" \
  bash scripts/recipes/gitops-check-edge-handoff-bundle.sh "$prod_hostname" beta

prod_root="${root}/prod-root"
make_beta_bundle "$prod_root"
printf '\nroot * /var/lib/fishystuff/gitops/served/production/site\n' >>"${prod_root}/artifacts/config/base"
expect_fail_contains \
  "reject production served root" \
  "must not contain forbidden beta Caddy fragment: /var/lib/fishystuff/gitops/served/production" \
  bash scripts/recipes/gitops-check-edge-handoff-bundle.sh "$prod_root" beta

prod_dependency="${root}/prod-dependency"
make_beta_bundle "$prod_dependency"
printf '\nWants=network-online.target fishystuff-api.service fishystuff-vector.service\n' >>"${prod_dependency}/artifacts/systemd/unit"
expect_fail_contains \
  "reject production service dependency" \
  "must not contain forbidden beta unit fragment: Wants=network-online.target fishystuff-api.service fishystuff-vector.service" \
  bash scripts/recipes/gitops-check-edge-handoff-bundle.sh "$prod_dependency" beta

prod_tls="${root}/prod-tls"
make_beta_bundle "$prod_tls"
printf '\nLoadCredential=fullchain.pem:/run/fishystuff/edge/tls/fullchain.pem\n' >>"${prod_tls}/artifacts/systemd/unit"
expect_fail_contains \
  "reject production TLS path" \
  "must not contain forbidden beta unit fragment: LoadCredential=fullchain.pem:/run/fishystuff/edge/tls/fullchain.pem" \
  bash scripts/recipes/gitops-check-edge-handoff-bundle.sh "$prod_tls" beta

wrong_api="${root}/wrong-api"
make_beta_bundle "$wrong_api"
perl -0pi -e 's/reverse_proxy 127\.0\.0\.1:18192/reverse_proxy 127.0.0.1:18092/' "${wrong_api}/artifacts/config/base"
expect_fail_contains \
  "reject wrong API upstream" \
  "missing loopback candidate API upstream" \
  bash scripts/recipes/gitops-check-edge-handoff-bundle.sh "$wrong_api" beta

printf '[gitops-beta-edge-handoff-bundle-test] %s checks passed\n' "$pass_count"
