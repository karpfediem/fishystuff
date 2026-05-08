#!/usr/bin/env bash
set -euo pipefail

SCRIPT_PATH="$(readlink -f "${BASH_SOURCE[0]}")"
SCRIPT_DIR="$(cd "$(dirname "$SCRIPT_PATH")" && pwd)"
source "${SCRIPT_DIR}/lib/common.sh"

cd "$RECIPE_REPO_ROOT"

pass_count=0

pass() {
  printf '[gitops-production-host-handoff-plan-test] pass: %s\n' "$1"
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
    printf '[gitops-production-host-handoff-plan-test] expected failure: %s\n' "$name" >&2
    exit 1
  fi
  if ! grep -F "$expected" "$stderr" >/dev/null; then
    printf '[gitops-production-host-handoff-plan-test] expected %s stderr to contain %q\n' "$name" "$expected" >&2
    cat "$stderr" >&2
    exit 1
  fi
  pass "$name"
}

release_identity_from_state() {
  local state_file="$1"
  local release_id="$2"

  jq -er \
    --arg release_id "$release_id" \
    '(.releases[$release_id] // error("release is missing")) as $release
    | "release=\($release_id);generation=\($release.generation);git_rev=\($release.git_rev);dolt_commit=\($release.dolt_commit);dolt_repository=\($release.dolt.repository);dolt_branch_context=\($release.dolt.branch_context);dolt_mode=\($release.dolt.mode);api=\($release.closures.api.store_path);site=\($release.closures.site.store_path);cdn_runtime=\($release.closures.cdn_runtime.store_path);dolt_service=\($release.closures.dolt_service.store_path)"' \
    "$state_file"
}

make_fake_deploy() {
  local path="$1"
  cat >"$path" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
if [[ "$*" != gitops\ check-desired-serving\ --state\ *\ --environment\ production ]]; then
  echo "unexpected fake fishystuff_deploy args: $*" >&2
  exit 2
fi
printf 'fake_desired_serving_ok\n'
EOF
  chmod +x "$path"
}

make_edge_bundle() {
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

make_fixture() {
  local root="$1"
  local api_upstream="${2-http://127.0.0.1:18092}"
  local active_id="production-release"
  local retained_id="previous-production-release"
  local active_api="${root}/active-api"
  local active_site="${root}/active-site"
  local active_cdn="${root}/active-cdn"
  local active_cdn_current="${root}/active-cdn-current"
  local active_dolt="${root}/active-dolt-service"
  local retained_api="${root}/retained-api"
  local retained_site="${root}/retained-site"
  local retained_cdn="${root}/retained-cdn"
  local retained_cdn_current="${root}/retained-cdn-current"
  local retained_dolt="${root}/retained-dolt-service"
  local state_file="${root}/production-current.desired.json"
  local summary_file="${root}/production-current.handoff-summary.json"
  local draft_file="${root}/production-activation.draft.desired.json"
  local admission_file="${root}/production-admission.json"
  local state_sha=""
  local summary_sha=""
  local identity=""

  mkdir -p "$active_api" "$active_site" "$active_cdn" "$active_cdn_current" "$active_dolt"
  mkdir -p "$retained_api" "$retained_site" "$retained_cdn" "$retained_cdn_current" "$retained_dolt"
  jq -n \
    --arg current_root "$retained_cdn_current" \
    '{
      current_root: $current_root,
      retained_roots: [],
      retained_root_count: 0
    }' >"${retained_cdn}/cdn-serving-manifest.json"
  jq -n \
    --arg current_root "$active_cdn_current" \
    --arg retained_root "$retained_cdn_current" \
    '{
      current_root: $current_root,
      retained_roots: [$retained_root],
      retained_root_count: 1
    }' >"${active_cdn}/cdn-serving-manifest.json"

  jq -n \
    --arg api "$active_api" \
    --arg site "$active_site" \
    --arg cdn "$active_cdn" \
    --arg dolt_service "$active_dolt" \
    --arg retained_api "$retained_api" \
    --arg retained_site "$retained_site" \
    --arg retained_cdn "$retained_cdn" \
    --arg retained_dolt_service "$retained_dolt" \
    --arg active_id "$active_id" \
    --arg retained_id "$retained_id" \
    '{
      cluster: "production",
      generation: 41,
      mode: "validate",
      hosts: {
        "production-single-host": {
          enabled: true,
          role: "single-site",
          hostname: "production-single-host"
        }
      },
      releases: {
        ($active_id): {
          generation: 7,
          git_rev: "active-git",
          dolt_commit: "active-dolt",
          closures: {
            api: { enabled: true, store_path: $api, gcroot_path: "/nix/var/nix/gcroots/fishystuff/gitops/production-release/api" },
            site: { enabled: true, store_path: $site, gcroot_path: "/nix/var/nix/gcroots/fishystuff/gitops/production-release/site" },
            cdn_runtime: { enabled: true, store_path: $cdn, gcroot_path: "/nix/var/nix/gcroots/fishystuff/gitops/production-release/cdn-runtime" },
            dolt_service: { enabled: true, store_path: $dolt_service, gcroot_path: "/nix/var/nix/gcroots/fishystuff/gitops/production-release/dolt-service" }
          },
          dolt: {
            repository: "fishystuff/fishystuff",
            commit: "active-dolt",
            branch_context: "main",
            mode: "read_only",
            materialization: "fetch_pin",
            remote_url: "file:///tmp/fishystuff-dolt-remote",
            cache_dir: "/var/lib/fishystuff/gitops/dolt-cache/fishystuff",
            release_ref: "fishystuff/gitops/production-release"
          }
        },
        ($retained_id): {
          generation: 6,
          git_rev: "retained-git",
          dolt_commit: "retained-dolt",
          closures: {
            api: { enabled: true, store_path: $retained_api, gcroot_path: "/nix/var/nix/gcroots/fishystuff/gitops/previous-production-release/api" },
            site: { enabled: true, store_path: $retained_site, gcroot_path: "/nix/var/nix/gcroots/fishystuff/gitops/previous-production-release/site" },
            cdn_runtime: { enabled: true, store_path: $retained_cdn, gcroot_path: "/nix/var/nix/gcroots/fishystuff/gitops/previous-production-release/cdn-runtime" },
            dolt_service: { enabled: true, store_path: $retained_dolt_service, gcroot_path: "/nix/var/nix/gcroots/fishystuff/gitops/previous-production-release/dolt-service" }
          },
          dolt: {
            repository: "fishystuff/fishystuff",
            commit: "retained-dolt",
            branch_context: "main",
            mode: "read_only",
            materialization: "fetch_pin",
            remote_url: "file:///tmp/fishystuff-dolt-remote",
            cache_dir: "/var/lib/fishystuff/gitops/dolt-cache/fishystuff",
            release_ref: "fishystuff/gitops/previous-production-release"
          }
        }
      },
      environments: {
        production: {
          enabled: true,
          strategy: "single_active",
          host: "production-single-host",
          active_release: $active_id,
          retained_releases: [$retained_id],
          serve: false
        }
      }
    }' >"$state_file"

  read -r state_sha _ < <(sha256sum "$state_file")
  jq -n \
    --arg state_file "$state_file" \
    --arg state_sha "$state_sha" \
    --arg active_id "$active_id" \
    --arg retained_id "$retained_id" \
    --arg active_api "$active_api" \
    --arg active_site "$active_site" \
    --arg active_cdn "$active_cdn" \
    --arg active_dolt "$active_dolt" \
    --arg retained_api "$retained_api" \
    --arg retained_site "$retained_site" \
    --arg retained_cdn "$retained_cdn" \
    --arg retained_dolt "$retained_dolt" \
    --arg active_cdn_manifest "${active_cdn}/cdn-serving-manifest.json" \
    --arg active_cdn_current "$active_cdn_current" \
    --arg retained_cdn_current "$retained_cdn_current" \
    '{
      schema: "fishystuff.gitops.production-current-handoff.v1",
      desired_state_path: $state_file,
      desired_state_sha256: $state_sha,
      cluster: "production",
      mode: "validate",
      desired_generation: 41,
      environment: {
        name: "production",
        host: "production-single-host",
        serve_requested: false,
        active_release: $active_id,
        retained_releases: [$retained_id]
      },
      active_release: {
        release_id: $active_id,
        generation: 7,
        git_rev: "active-git",
        dolt_commit: "active-dolt",
        closures: {
          api: $active_api,
          site: $active_site,
          cdn_runtime: $active_cdn,
          dolt_service: $active_dolt
        },
        dolt: {
          materialization: "fetch_pin",
          branch_context: "main",
          cache_dir: "/var/lib/fishystuff/gitops/dolt-cache/fishystuff",
          release_ref: "fishystuff/gitops/production-release"
        }
      },
      retained_release_count: 1,
      retained_releases: [
        {
          release_id: $retained_id,
          generation: 6,
          git_rev: "retained-git",
          dolt_commit: "retained-dolt",
          closures: {
            api: $retained_api,
            site: $retained_site,
            cdn_runtime: $retained_cdn,
            dolt_service: $retained_dolt
          },
          dolt: {
            materialization: "fetch_pin",
            branch_context: "main",
            cache_dir: "/var/lib/fishystuff/gitops/dolt-cache/fishystuff",
            release_ref: "fishystuff/gitops/previous-production-release"
          }
        }
      ],
      cdn_retention: {
        active_cdn_runtime: $active_cdn,
        active_manifest: $active_cdn_manifest,
        active_current_root: $active_cdn_current,
        active_retained_roots: [$retained_cdn_current],
        retained_releases: [
          {
            release_id: $retained_id,
            cdn_runtime: $retained_cdn,
            retained_cdn_runtime_is_serving_root: true,
            expected_retained_cdn_root: $retained_cdn_current,
            retained_by_active_cdn_serving_root: true
          }
        ]
      },
      checks: {
        production_current_desired_generated: true,
        desired_serving_preflight_passed: true,
        closure_paths_verified: true,
        cdn_retained_roots_verified: true,
        gitops_unify_passed: true,
        remote_deploy_performed: false,
        infrastructure_mutation_performed: false
      }
    }' >"$summary_file"

  identity="$(release_identity_from_state "$state_file" "$active_id")"
  read -r summary_sha _ < <(sha256sum "$summary_file")
  jq \
    --arg api_upstream "$api_upstream" \
    --arg admission_url "${api_upstream}/api/v1/meta" \
    '.mode = "local-apply"
      | .generation = 42
      | .environments.production.serve = true
      | .environments.production.api_upstream = $api_upstream
      | .environments.production.admission_probe = {
          kind: "api_meta",
          probe_name: "api-meta",
          url: $admission_url,
          expected_status: 200,
          timeout_ms: 2000
        }
      | .environments.production.transition = {
          kind: "activate",
          from_release: "",
          reason: "fixture"
        }' \
    "$state_file" >"$draft_file"
  jq -n \
    --arg summary_sha "$summary_sha" \
    --arg state_sha "$state_sha" \
    --arg active_id "$active_id" \
    --arg identity "$identity" \
    --arg api_upstream "$api_upstream" \
    '{
      schema: "fishystuff.gitops.activation-admission.v1",
      environment: "production",
      handoff_summary_sha256: $summary_sha,
      desired_state_sha256: $state_sha,
      release_id: $active_id,
      release_identity: $identity,
      dolt_commit: "active-dolt",
      api_upstream: $api_upstream,
      api_meta: {
        url: ($api_upstream + "/api/v1/meta"),
        observed_status: 200,
        timeout_ms: 2000,
        release_id: $active_id,
        release_identity: $identity,
        dolt_commit: "active-dolt"
      },
      db_backed_probe: {
        name: "db-fixture",
        passed: true
      },
      site_cdn_probe: {
        name: "site-cdn-fixture",
        passed: true
      }
    }' >"$admission_file"

  printf '%s\n' "$draft_file" >"${root}/draft.path"
  printf '%s\n' "$summary_file" >"${root}/summary.path"
  printf '%s\n' "$admission_file" >"${root}/admission.path"
}

if [[ "${FISHYSTUFF_GITOPS_HOST_HANDOFF_PLAN_TEST_SOURCE_ONLY:-}" == "1" ]]; then
  return 0 2>/dev/null || exit 0
fi

root="$(mktemp -d)"
make_fixture "$root"
make_edge_bundle "${root}/edge-bundle"
make_fake_deploy "${root}/fishystuff_deploy"

draft="$(cat "${root}/draft.path")"
summary="$(cat "${root}/summary.path")"
admission="$(cat "${root}/admission.path")"

bash scripts/recipes/gitops-production-host-handoff-plan.sh \
  "$draft" \
  "$summary" \
  "$admission" \
  "${root}/edge-bundle" \
  "${root}/fishystuff_deploy" >"${root}/plan.stdout"

grep -F "gitops_production_host_handoff_plan_ok=${draft}" "${root}/plan.stdout" >/dev/null
grep -F "edge_bundle=${root}/edge-bundle" "${root}/plan.stdout" >/dev/null
grep -F "systemd_unit_install_path=/etc/systemd/system/fishystuff-edge.service" "${root}/plan.stdout" >/dev/null
grep -F "planned_host_step_05=systemctl restart fishystuff-edge.service" "${root}/plan.stdout" >/dev/null
grep -F "remote_deploy_performed=false" "${root}/plan.stdout" >/dev/null
pass "valid host handoff plan"

wrong_upstream="${root}/wrong-upstream"
mkdir -p "$wrong_upstream"
make_fixture "$wrong_upstream" "http://127.0.0.1:18093"
expect_fail_contains \
  "reject API upstream mismatch" \
  "activation draft API upstream does not match edge handoff bundle upstream" \
  bash scripts/recipes/gitops-production-host-handoff-plan.sh \
    "$(cat "${wrong_upstream}/draft.path")" \
    "$(cat "${wrong_upstream}/summary.path")" \
    "$(cat "${wrong_upstream}/admission.path")" \
    "${root}/edge-bundle" \
    "${root}/fishystuff_deploy"

missing_bundle_metadata="${root}/missing-bundle-metadata"
make_edge_bundle "$missing_bundle_metadata"
jq 'del(.activation.requiredPaths)' "${missing_bundle_metadata}/bundle.json" >"${missing_bundle_metadata}/bundle.json.tmp"
mv "${missing_bundle_metadata}/bundle.json.tmp" "${missing_bundle_metadata}/bundle.json"
expect_fail_contains \
  "reject missing bundle required paths" \
  "bundle metadata is missing GitOps site required path" \
  bash scripts/recipes/gitops-production-host-handoff-plan.sh \
    "$draft" \
    "$summary" \
    "$admission" \
    "$missing_bundle_metadata" \
    "${root}/fishystuff_deploy"

missing_admission="${root}/missing-admission.json"
expect_fail_contains \
  "reject missing admission evidence" \
  "admission evidence does not exist" \
  bash scripts/recipes/gitops-production-host-handoff-plan.sh \
    "$draft" \
    "$summary" \
    "$missing_admission" \
    "${root}/edge-bundle" \
    "${root}/fishystuff_deploy"

printf '[gitops-production-host-handoff-plan-test] %s checks passed\n' "$pass_count"
