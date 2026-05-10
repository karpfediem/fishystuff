#!/usr/bin/env bash
set -euo pipefail

SCRIPT_PATH="$(readlink -f "${BASH_SOURCE[0]}")"
SCRIPT_DIR="$(cd "$(dirname "$SCRIPT_PATH")" && pwd)"
source "${SCRIPT_DIR}/lib/common.sh"

cd "$RECIPE_REPO_ROOT"

pass_count=0

pass() {
  printf '[gitops-beta-remote-start-services-test] pass: %s\n' "$1"
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
    printf '[gitops-beta-remote-start-services-test] expected failure: %s\n' "$name" >&2
    exit 1
  fi
  if ! grep -F "$expected" "$stderr" >/dev/null; then
    printf '[gitops-beta-remote-start-services-test] expected %s stderr to contain %q\n' "$name" "$expected" >&2
    cat "$stderr" >&2
    exit 1
  fi
  pass "$name"
}

make_beta_service_bundle() {
  local bundle="$1"
  local service="$2"
  local service_id=""
  local unit_name=""
  local config_destination=""
  local runtime_env_target=""
  local release_env_target=""
  local exe_real=""
  local config_real=""
  local systemd_unit_real=""

  case "$service" in
    api)
      service_id="fishystuff-beta-api"
      unit_name="fishystuff-beta-api.service"
      config_destination="config.toml"
      runtime_env_target="/var/lib/fishystuff/gitops-beta/api/runtime.env"
      release_env_target="/var/lib/fishystuff/gitops-beta/api/beta.env"
      ;;
    dolt)
      service_id="fishystuff-beta-dolt"
      unit_name="fishystuff-beta-dolt.service"
      config_destination="sql-server.yaml"
      runtime_env_target="/var/lib/fishystuff/gitops-beta/dolt/beta.env"
      release_env_target=""
      ;;
    *)
      printf '[gitops-beta-remote-start-services-test] unsupported fixture service: %s\n' "$service" >&2
      exit 1
      ;;
  esac

  mkdir -p "${bundle}/artifacts/exe" "${bundle}/artifacts/config" "${bundle}/artifacts/systemd"
  cat >"${bundle}/artifacts/exe/main" <<'EOF'
#!/usr/bin/env bash
exit 0
EOF
  chmod +x "${bundle}/artifacts/exe/main"

  case "$service" in
    api)
      cat >"${bundle}/artifacts/config/base" <<'EOF'
[server]
bind = "127.0.0.1:18192"
EOF
      ;;
    dolt)
      cat >"${bundle}/artifacts/config/base" <<'EOF'
listener:
  host: 127.0.0.1
port: 3316
data_dir: /var/lib/fishystuff/beta-dolt/fishystuff
cfg_dir: /var/lib/fishystuff/beta-dolt/.doltcfg
EOF
      ;;
  esac

  exe_real="$(readlink -f "${bundle}/artifacts/exe/main")"
  config_real="$(readlink -f "${bundle}/artifacts/config/base")"
  systemd_unit_real="$(readlink -f "${bundle}/artifacts/systemd/unit")"

  case "$service" in
    api)
      cat >"${bundle}/artifacts/systemd/unit" <<EOF
[Unit]
Description=Fishystuff beta API
[Service]
Type=simple
DynamicUser=true
PrivateTmp=true
ProtectSystem=strict
NoNewPrivileges=true
Environment="FISHYSTUFF_DEPLOYMENT_ENVIRONMENT=beta"
Environment="FISHYSTUFF_OTEL_DEPLOYMENT_ENVIRONMENT=beta"
Environment="FISHYSTUFF_SECRETSPEC_PATH=/etc/fishystuff/secretspec.toml"
EnvironmentFile=-${runtime_env_target}
EnvironmentFile=-${release_env_target}
ExecStart=${exe_real} --config ${config_real} --bind 127.0.0.1:18192
Restart=on-failure
[Install]
WantedBy=multi-user.target
EOF
      ;;
    dolt)
      cat >"${bundle}/artifacts/systemd/unit" <<EOF
[Unit]
Description=Fishystuff beta Dolt
[Service]
Type=simple
User=fishystuff-beta-dolt
Group=fishystuff-beta-dolt
StateDirectory=fishystuff/beta-dolt
StateDirectoryMode=0750
WorkingDirectory=/var/lib/fishystuff/beta-dolt
Environment="FISHYSTUFF_DEPLOYMENT_ENVIRONMENT=beta"
Environment="HOME=/var/lib/fishystuff/beta-dolt/home"
EnvironmentFile=${runtime_env_target}
ExecStart=${exe_real} --config ${config_real}
ExecReload=${exe_real} refresh
Restart=on-failure
[Install]
WantedBy=multi-user.target
EOF
      ;;
  esac

  jq -n \
    --arg service_id "$service_id" \
    --arg unit_name "$unit_name" \
    --arg config_destination "$config_destination" \
    --arg runtime_env_target "$runtime_env_target" \
    --arg release_env_target "$release_env_target" \
    --arg exe_real "$exe_real" \
    --arg config_real "$config_real" \
    --arg systemd_unit_real "$systemd_unit_real" \
    --arg service "$service" \
    '{
      id: $service_id,
      roots: {
        store: [$exe_real, $config_real, $systemd_unit_real]
      },
      artifacts: {
        "exe/main": {
          kind: "binary",
          storePath: $exe_real
        },
        "config/base": {
          kind: "config",
          storePath: $config_real,
          destination: $config_destination
        },
        "systemd/unit": {
          kind: "systemd-unit",
          storePath: $systemd_unit_real,
          destination: $unit_name,
          bundle_path: "artifacts/systemd/unit"
        }
      },
      runtimeOverlays: [
        {
          targetPath: $runtime_env_target,
          secret: true,
          onChange: "restart"
        }
      ],
      supervision: {
        environmentFiles: (
          if $release_env_target != "" then
            [$runtime_env_target, ("-" + $release_env_target)]
          else
            [$runtime_env_target]
          end
        ),
        environment: (
          if $service == "api" then
            {
              FISHYSTUFF_DEPLOYMENT_ENVIRONMENT: "beta",
              FISHYSTUFF_OTEL_DEPLOYMENT_ENVIRONMENT: "beta",
              FISHYSTUFF_SECRETSPEC_PATH: "/etc/fishystuff/secretspec.toml"
            }
          else
            {
              FISHYSTUFF_DEPLOYMENT_ENVIRONMENT: "beta",
              HOME: "/var/lib/fishystuff/beta-dolt/home"
            }
          end
        ),
        workingDirectory: (if $service == "dolt" then "/var/lib/fishystuff/beta-dolt" else null end),
        restart: {
          policy: "on-failure"
        },
        reload: {
          mode: (if $service == "dolt" then "command" else "restart" end)
        }
      },
      backends: {
        systemd: {
          daemon_reload: true,
          units: [
            {
              name: $unit_name,
              install_path: ("/etc/systemd/system/" + $unit_name),
              artifact: "systemd/unit",
              startup: "enabled",
              state: "running"
            }
          ]
        }
      }
    }' >"${bundle}/bundle.json"
  {
    printf '%s\n' "$exe_real"
    printf '%s\n' "$config_real"
    printf '%s\n' "$systemd_unit_real"
  } >"${bundle}/store-paths"
}

root="$(mktemp -d)"
api_bundle="${root}/api-bundle"
dolt_bundle="${root}/dolt-bundle"
summary="${root}/beta-current.handoff-summary.json"
fake_ssh="${root}/ssh"

make_beta_service_bundle "$api_bundle" api
make_beta_service_bundle "$dolt_bundle" dolt
read -r api_unit_sha256 _ < <(sha256sum "${api_bundle}/artifacts/systemd/unit")
read -r dolt_unit_sha256 _ < <(sha256sum "${dolt_bundle}/artifacts/systemd/unit")

jq -n \
  --arg api_bundle "$api_bundle" \
  --arg dolt_bundle "$dolt_bundle" \
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
        api: $api_bundle,
        site: "/tmp/site-fixture",
        cdn_runtime: "/tmp/cdn-fixture",
        dolt_service: $dolt_bundle
      }
    },
    checks: {
      closure_paths_verified: true,
      gitops_unify_passed: true,
      remote_deploy_performed: false,
      infrastructure_mutation_performed: false
    }
  }' >"$summary"

cat >"$fake_ssh" <<'SSH'
#!/usr/bin/env bash
set -euo pipefail
printf '%s\n' "$*" >>"${FISHYSTUFF_FAKE_REMOTE_LOG:?}"
cat >"${FISHYSTUFF_FAKE_REMOTE_STDIN:?}"
printf 'remote_hostname=site-nbg1-beta\n'
printf 'remote_dolt_service_install_ok=fishystuff-beta-dolt.service\n'
printf 'remote_api_service_install_ok=fishystuff-beta-api.service\n'
printf 'remote_dolt_service_restart_ok=fishystuff-beta-dolt.service\n'
printf 'remote_api_service_restart_ok=fishystuff-beta-api.service\n'
printf 'remote_host_mutation_performed=true\n'
printf 'remote_deploy_performed=false\n'
printf 'infrastructure_mutation_performed=false\n'
printf 'local_host_mutation_performed=false\n'
SSH
chmod +x "$fake_ssh"

env \
  FISHYSTUFF_OPERATOR_SECRETSPEC_PROFILE=beta-deploy \
  FISHYSTUFF_GITOPS_ENABLE_BETA_REMOTE_SERVICE_START=1 \
  FISHYSTUFF_GITOPS_ENABLE_BETA_REMOTE_DOLT_INSTALL=1 \
  FISHYSTUFF_GITOPS_ENABLE_BETA_REMOTE_DOLT_RESTART=1 \
  FISHYSTUFF_GITOPS_ENABLE_BETA_REMOTE_API_INSTALL=1 \
  FISHYSTUFF_GITOPS_ENABLE_BETA_REMOTE_API_RESTART=1 \
  FISHYSTUFF_GITOPS_BETA_REMOTE_SERVICE_TARGET=root@203.0.113.20 \
  FISHYSTUFF_GITOPS_BETA_DOLT_UNIT_SHA256="$dolt_unit_sha256" \
  FISHYSTUFF_GITOPS_BETA_API_UNIT_SHA256="$api_unit_sha256" \
  FISHYSTUFF_GITOPS_BETA_REMOTE_SERVICE_START_ALLOW_BUNDLE_FIXTURE=1 \
  HETZNER_SSH_PRIVATE_KEY='fixture-private-key' \
  FISHYSTUFF_FAKE_REMOTE_LOG="${root}/remote.log" \
  FISHYSTUFF_FAKE_REMOTE_STDIN="${root}/remote.sh" \
  bash scripts/recipes/gitops-beta-remote-start-services.sh root@203.0.113.20 site-nbg1-beta "$summary" "$fake_ssh" >"${root}/start.out"
grep -F "gitops_beta_remote_start_services_checked=true" "${root}/start.out" >/dev/null
grep -F "gitops_beta_remote_start_services_ok=true" "${root}/start.out" >/dev/null
grep -F "resident_target=root@203.0.113.20" "${root}/start.out" >/dev/null
grep -F "remote_dolt_service_restart_ok=fishystuff-beta-dolt.service" "${root}/start.out" >/dev/null
grep -F "root@203.0.113.20" "${root}/remote.log" >/dev/null
grep -F "systemctl restart fishystuff-beta-dolt.service" "${root}/remote.sh" >/dev/null
grep -F "systemctl restart fishystuff-beta-api.service" "${root}/remote.sh" >/dev/null
grep -F "http://127.0.0.1:18192/api/v1/meta" "${root}/remote.sh" >/dev/null
pass "remote service start is explicit, beta-targeted, and Dolt-before-API"

base_env=(
  FISHYSTUFF_OPERATOR_SECRETSPEC_PROFILE=beta-deploy
  FISHYSTUFF_GITOPS_ENABLE_BETA_REMOTE_SERVICE_START=1
  FISHYSTUFF_GITOPS_ENABLE_BETA_REMOTE_DOLT_INSTALL=1
  FISHYSTUFF_GITOPS_ENABLE_BETA_REMOTE_DOLT_RESTART=1
  FISHYSTUFF_GITOPS_ENABLE_BETA_REMOTE_API_INSTALL=1
  FISHYSTUFF_GITOPS_ENABLE_BETA_REMOTE_API_RESTART=1
  FISHYSTUFF_GITOPS_BETA_REMOTE_SERVICE_TARGET=root@203.0.113.20
  FISHYSTUFF_GITOPS_BETA_DOLT_UNIT_SHA256="$dolt_unit_sha256"
  FISHYSTUFF_GITOPS_BETA_API_UNIT_SHA256="$api_unit_sha256"
  FISHYSTUFF_GITOPS_BETA_REMOTE_SERVICE_START_ALLOW_BUNDLE_FIXTURE=1
  HETZNER_SSH_PRIVATE_KEY=fixture-private-key
  FISHYSTUFF_FAKE_REMOTE_LOG="${root}/remote-fail.log"
  FISHYSTUFF_FAKE_REMOTE_STDIN="${root}/remote-fail.sh"
)

expect_fail_contains \
  "requires sequence opt-in" \
  "gitops-beta-remote-start-services requires FISHYSTUFF_GITOPS_ENABLE_BETA_REMOTE_SERVICE_START=1" \
  env \
    FISHYSTUFF_OPERATOR_SECRETSPEC_PROFILE=beta-deploy \
    HETZNER_SSH_PRIVATE_KEY='fixture-private-key' \
    bash scripts/recipes/gitops-beta-remote-start-services.sh root@203.0.113.20 site-nbg1-beta "$summary" "$fake_ssh"

expect_fail_contains \
  "requires target acknowledgement" \
  "gitops-beta-remote-start-services requires FISHYSTUFF_GITOPS_BETA_REMOTE_SERVICE_TARGET=root@203.0.113.20" \
  env \
    "${base_env[@]}" \
    FISHYSTUFF_GITOPS_BETA_REMOTE_SERVICE_TARGET=root@203.0.113.21 \
    bash scripts/recipes/gitops-beta-remote-start-services.sh root@203.0.113.20 site-nbg1-beta "$summary" "$fake_ssh"

expect_fail_contains \
  "rejects production profile" \
  "must not run with production SecretSpec profile active" \
  env \
    "${base_env[@]}" \
    FISHYSTUFF_OPERATOR_SECRETSPEC_PROFILE=production-deploy \
    bash scripts/recipes/gitops-beta-remote-start-services.sh root@203.0.113.20 site-nbg1-beta "$summary" "$fake_ssh"

expect_fail_contains \
  "rejects dns target" \
  "target host must be an IPv4 address" \
  env \
    "${base_env[@]}" \
    FISHYSTUFF_GITOPS_BETA_REMOTE_SERVICE_TARGET=root@beta.fishystuff.fish \
    bash scripts/recipes/gitops-beta-remote-start-services.sh root@beta.fishystuff.fish site-nbg1-beta "$summary" "$fake_ssh"

expect_fail_contains \
  "rejects stale reviewed API hash" \
  "FISHYSTUFF_GITOPS_BETA_API_UNIT_SHA256 does not match checked beta API unit" \
  env \
    "${base_env[@]}" \
    FISHYSTUFF_GITOPS_BETA_API_UNIT_SHA256=wrong \
    bash scripts/recipes/gitops-beta-remote-start-services.sh root@203.0.113.20 site-nbg1-beta "$summary" "$fake_ssh"

printf '[gitops-beta-remote-start-services-test] %s checks passed\n' "$pass_count"
