#!/usr/bin/env bash
set -euo pipefail

SCRIPT_PATH="$(readlink -f "${BASH_SOURCE[0]}")"
SCRIPT_DIR="$(cd "$(dirname "$SCRIPT_PATH")" && pwd)"
source "${SCRIPT_DIR}/lib/common.sh"

cd "$RECIPE_REPO_ROOT"

pass_count=0

pass() {
  printf '[gitops-production-edge-handoff-bundle-test] pass: %s\n' "$1"
  pass_count="$((pass_count + 1))"
}

make_bundle() {
  local bundle="$1"
  local caddy_bin_real=""
  local caddyfile_real=""
  local systemd_unit_real=""

  mkdir -p "${bundle}/artifacts/exe" "${bundle}/artifacts/config" "${bundle}/artifacts/systemd"
  cat >"${bundle}/artifacts/exe/main" <<'EOF'
#!/usr/bin/env bash
if [[ "${FISHYSTUFF_FAKE_CADDY_VALIDATE_FAIL:-}" == "1" && "${1:-}" == "validate" ]]; then
  echo "fake caddy validate failure" >&2
  exit 17
fi
exit 0
EOF
  chmod +x "${bundle}/artifacts/exe/main"
  cat >"${bundle}/artifacts/config/base" <<'EOF'
{
  auto_https off
}

https://fishystuff.fish {
  tls {$CREDENTIALS_DIRECTORY}/fullchain.pem {$CREDENTIALS_DIRECTORY}/privkey.pem
  root * /var/lib/fishystuff/gitops/served/production/site
  header Cache-Control "no-store"
  header Cache-Control "public, max-age=31536000, immutable"
}

https://api.fishystuff.fish {
  tls {$CREDENTIALS_DIRECTORY}/fullchain.pem {$CREDENTIALS_DIRECTORY}/privkey.pem
  reverse_proxy 127.0.0.1:18092
}

https://cdn.fishystuff.fish {
  tls {$CREDENTIALS_DIRECTORY}/fullchain.pem {$CREDENTIALS_DIRECTORY}/privkey.pem
  root * /var/lib/fishystuff/gitops/served/production/cdn
  @runtime_manifest path /map/runtime-manifest.json
  header Cache-Control "no-store"
  header Cache-Control "public, max-age=31536000, immutable"
}

https://telemetry.fishystuff.fish {
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
Wants=network-online.target fishystuff-api.service fishystuff-vector.service
[Service]
Type=simple
DynamicUser=true
ExecStart=${caddy_bin_real} run --config ${caddyfile_real} --adapter caddyfile
ExecReload=${caddy_bin_real} reload --config ${caddyfile_real} --adapter caddyfile --address 127.0.0.1:2019 --force
Restart=on-failure
RestartSec=5s
LoadCredential=fullchain.pem:/run/fishystuff/edge/tls/fullchain.pem
LoadCredential=privkey.pem:/run/fishystuff/edge/tls/privkey.pem
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
      id: "fishystuff-edge",
      activation: {
        directories: [
          {
            path: "/run/fishystuff/edge/tls",
            create: true
          }
        ],
        requiredPaths: [
          "/var/lib/fishystuff/gitops/served/production/site",
          "/var/lib/fishystuff/gitops/served/production/cdn"
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
          destination: "fishystuff-edge.service"
        }
      },
      backends: {
        systemd: {
          daemon_reload: true,
          units: [
            {
              name: "fishystuff-edge.service",
              install_path: "/etc/systemd/system/fishystuff-edge.service",
              state: "running",
              startup: "enabled"
            }
          ]
        }
      },
      runtimeOverlays: [
        {
          targetPath: "/run/fishystuff/edge/tls/fullchain.pem",
          required: true,
          secret: false,
          onChange: "restart"
        },
        {
          targetPath: "/run/fishystuff/edge/tls/privkey.pem",
          required: true,
          secret: true,
          onChange: "restart"
        }
      ],
      supervision: {
        argv: [$caddy_bin, "run", "--config", $caddyfile, "--adapter", "caddyfile"],
        reload: {
          mode: "command",
          argv: [$caddy_bin, "reload", "--config", $caddyfile, "--adapter", "caddyfile", "--address", "127.0.0.1:2019", "--force"]
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
    printf '[gitops-production-edge-handoff-bundle-test] expected failure: %s\n' "$name" >&2
    exit 1
  fi
  if ! grep -F "$expected" "$stderr" >/dev/null; then
    printf '[gitops-production-edge-handoff-bundle-test] expected %s stderr to contain %q\n' "$name" "$expected" >&2
    cat "$stderr" >&2
    exit 1
  fi
  pass "$name"
}

root="$(mktemp -d)"
valid="${root}/valid"
make_bundle "$valid"

bash scripts/recipes/gitops-check-edge-handoff-bundle.sh "$valid" >"${root}/valid.stdout"
grep -F "gitops_edge_handoff_bundle_ok=${valid}" "${root}/valid.stdout" >/dev/null
grep -F "gitops_edge_handoff_site_root=/var/lib/fishystuff/gitops/served/production/site" "${root}/valid.stdout" >/dev/null
grep -F "gitops_edge_handoff_cdn_root=/var/lib/fishystuff/gitops/served/production/cdn" "${root}/valid.stdout" >/dev/null
grep -F "gitops_edge_handoff_api_upstream=127.0.0.1:18092" "${root}/valid.stdout" >/dev/null
grep -F "gitops_edge_handoff_caddy_validate=true" "${root}/valid.stdout" >/dev/null
pass "valid bundle"

beta="${root}/beta"
make_bundle "$beta"
printf '\n# beta.fishystuff.fish must never be present here\n' >>"${beta}/artifacts/config/base"
expect_fail_contains \
  "reject beta hostname" \
  "must not contain forbidden production Caddy fragment: beta.fishystuff.fish" \
  bash scripts/recipes/gitops-check-edge-handoff-bundle.sh "$beta"

legacy="${root}/legacy"
make_bundle "$legacy"
printf '\n# /srv/fishystuff must never be present here\n' >>"${legacy}/artifacts/config/base"
expect_fail_contains \
  "reject legacy serving root" \
  "must not contain forbidden production Caddy fragment: /srv/fishystuff" \
  bash scripts/recipes/gitops-check-edge-handoff-bundle.sh "$legacy"

store_root="${root}/store-root"
make_bundle "$store_root"
printf '\nroot * /nix/store/example-site\n' >>"${store_root}/artifacts/config/base"
expect_fail_contains \
  "reject fixed store serving root" \
  "must not contain forbidden production Caddy fragment: root * /nix/store/" \
  bash scripts/recipes/gitops-check-edge-handoff-bundle.sh "$store_root"

wrong_api="${root}/wrong-api"
make_bundle "$wrong_api"
perl -0pi -e 's/reverse_proxy 127\.0\.0\.1:18092/reverse_proxy 127.0.0.1:18091/' "${wrong_api}/artifacts/config/base"
expect_fail_contains \
  "reject wrong API upstream" \
  "missing loopback candidate API upstream" \
  bash scripts/recipes/gitops-check-edge-handoff-bundle.sh "$wrong_api"

missing_exe="${root}/missing-exe"
make_bundle "$missing_exe"
rm -f "${missing_exe}/artifacts/exe/main"
expect_fail_contains \
  "reject missing executable" \
  "Caddy executable is missing or not executable" \
  bash scripts/recipes/gitops-check-edge-handoff-bundle.sh "$missing_exe"

bad_metadata="${root}/bad-metadata"
make_bundle "$bad_metadata"
jq '.artifacts."exe/main".storePath = "/tmp/wrong-caddy"' "${bad_metadata}/bundle.json" >"${bad_metadata}/bundle.json.tmp"
mv "${bad_metadata}/bundle.json.tmp" "${bad_metadata}/bundle.json"
expect_fail_contains \
  "reject artifact metadata mismatch" \
  "Caddy executable artifact path mismatch" \
  bash scripts/recipes/gitops-check-edge-handoff-bundle.sh "$bad_metadata"

bad_reload="${root}/bad-reload"
make_bundle "$bad_reload"
perl -0pi -e 's/--address 127\.0\.0\.1:2019 --force/--address 127.0.0.1:2020 --force/' "${bad_reload}/artifacts/systemd/unit"
expect_fail_contains \
  "reject systemd reload mismatch" \
  "systemd unit is missing Caddy ExecReload" \
  bash scripts/recipes/gitops-check-edge-handoff-bundle.sh "$bad_reload"

bad_required_path="${root}/bad-required-path"
make_bundle "$bad_required_path"
jq 'del(.activation.requiredPaths)' "${bad_required_path}/bundle.json" >"${bad_required_path}/bundle.json.tmp"
mv "${bad_required_path}/bundle.json.tmp" "${bad_required_path}/bundle.json"
expect_fail_contains \
  "reject missing GitOps required paths" \
  "bundle metadata is missing GitOps site required path" \
  bash scripts/recipes/gitops-check-edge-handoff-bundle.sh "$bad_required_path"

bad_caddy_validate="${root}/bad-caddy-validate"
make_bundle "$bad_caddy_validate"
expect_fail_contains \
  "reject Caddy validation failure" \
  "Caddyfile failed caddy validate" \
  env FISHYSTUFF_FAKE_CADDY_VALIDATE_FAIL=1 \
    bash scripts/recipes/gitops-check-edge-handoff-bundle.sh "$bad_caddy_validate"

printf '[gitops-production-edge-handoff-bundle-test] %s checks passed\n' "$pass_count"
