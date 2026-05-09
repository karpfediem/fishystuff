#!/usr/bin/env bash
set -euo pipefail

SCRIPT_PATH="$(readlink -f "${BASH_SOURCE[0]}")"
SCRIPT_DIR="$(cd "$(dirname "$SCRIPT_PATH")" && pwd)"
source "${SCRIPT_DIR}/lib/common.sh"

cd "$RECIPE_REPO_ROOT"

pass_count=0

pass() {
  printf '[gitops-beta-service-start-plan-test] pass: %s\n' "$1"
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
    printf '[gitops-beta-service-start-plan-test] expected failure: %s\n' "$name" >&2
    exit 1
  fi
  if ! grep -F "$expected" "$stderr" >/dev/null; then
    printf '[gitops-beta-service-start-plan-test] expected %s stderr to contain %q\n' "$name" "$expected" >&2
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
      printf '[gitops-beta-service-start-plan-test] unsupported fixture service: %s\n' "$service" >&2
      exit 1
      ;;
  esac

  mkdir -p "${bundle}/artifacts/exe" "${bundle}/artifacts/config" "${bundle}/artifacts/systemd"
  cat >"${bundle}/artifacts/exe/main" <<'EOF'
#!/usr/bin/env bash
exit 0
EOF
  chmod +x "${bundle}/artifacts/exe/main"
  exe_real="$(readlink -f "${bundle}/artifacts/exe/main")"
  config_real="$(readlink -f "${bundle}/artifacts/config/base")"
  systemd_unit_real="$(readlink -f "${bundle}/artifacts/systemd/unit")"

  case "$service" in
    api)
      cat >"${bundle}/artifacts/config/base" <<'EOF'
[server]
bind = "127.0.0.1:18192"
EOF
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
      cat >"${bundle}/artifacts/config/base" <<'EOF'
listener:
  host: 127.0.0.1
port: 3316
data_dir: /var/lib/fishystuff/beta-dolt/fishystuff
cfg_dir: /var/lib/fishystuff/beta-dolt/.doltcfg
EOF
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
api_env="${root}/api/runtime.env"
dolt_env="${root}/dolt/beta.env"
make_beta_service_bundle "$api_bundle" api
make_beta_service_bundle "$dolt_bundle" dolt
read -r api_unit_sha256 _ < <(sha256sum "${api_bundle}/artifacts/systemd/unit")
read -r dolt_unit_sha256 _ < <(sha256sum "${dolt_bundle}/artifacts/systemd/unit")

env \
  FISHYSTUFF_GITOPS_ENABLE_BETA_API_RUNTIME_ENV_WRITE=1 \
  FISHYSTUFF_GITOPS_BETA_API_DATABASE_URL="mysql://fishy:secret@127.0.0.1:3316/fishystuff" \
  bash scripts/recipes/gitops-beta-write-runtime-env.sh api "$api_env" >/dev/null
env \
  FISHYSTUFF_GITOPS_ENABLE_BETA_DOLT_RUNTIME_ENV_WRITE=1 \
  bash scripts/recipes/gitops-beta-write-runtime-env.sh dolt "$dolt_env" >/dev/null

expect_fail_contains \
  "refuse fixture env path without explicit test override" \
  "runtime env file does not match beta service bundle target" \
  bash scripts/recipes/gitops-beta-service-start-plan.sh \
    "$api_bundle" \
    "$dolt_bundle" \
    "$api_env" \
    "$dolt_env"

FISHYSTUFF_GITOPS_BETA_SERVICE_START_PLAN_ALLOW_ENV_FILE_FIXTURE=1 \
  bash scripts/recipes/gitops-beta-service-start-plan.sh \
    "$api_bundle" \
    "$dolt_bundle" \
    "$api_env" \
    "$dolt_env" >"${root}/plan.stdout"
grep -F "gitops_beta_service_start_plan_ok=true" "${root}/plan.stdout" >/dev/null
grep -F "gitops_beta_service_start_plan_api_unit_sha256=${api_unit_sha256}" "${root}/plan.stdout" >/dev/null
grep -F "gitops_beta_service_start_plan_dolt_unit_sha256=${dolt_unit_sha256}" "${root}/plan.stdout" >/dev/null
grep -F "gitops_beta_service_start_plan_api_runtime_env_target=/var/lib/fishystuff/gitops-beta/api/runtime.env" "${root}/plan.stdout" >/dev/null
grep -F "gitops_beta_service_start_plan_api_release_env_target=/var/lib/fishystuff/gitops-beta/api/beta.env" "${root}/plan.stdout" >/dev/null
grep -F "FISHYSTUFF_GITOPS_ENABLE_BETA_DOLT_INSTALL=1" "${root}/plan.stdout" >/dev/null
grep -F "FISHYSTUFF_GITOPS_ENABLE_BETA_API_INSTALL=1" "${root}/plan.stdout" >/dev/null
grep -F "systemctl is-active --quiet fishystuff-beta-dolt.service" "${root}/plan.stdout" >/dev/null
grep -F "systemctl is-active --quiet fishystuff-beta-api.service" "${root}/plan.stdout" >/dev/null
pass "valid beta service start plan"

if grep -E 'fishystuff-api\.service|fishystuff-dolt\.service|/run/fishystuff/api/env|https://api\.fishystuff\.fish|https://cdn\.fishystuff\.fish' "${root}/plan.stdout" >/dev/null; then
  printf '[gitops-beta-service-start-plan-test] beta start plan leaked production/shared service material\n' >&2
  exit 1
fi
pass "no production service material in plan"

bad_api_env="${root}/bad-api.env"
cat >"$bad_api_env" <<'EOF'
FISHYSTUFF_DATABASE_URL='mysql://fishy:secret@127.0.0.1:3316/fishystuff'
FISHYSTUFF_CORS_ALLOWED_ORIGINS='https://fishystuff.fish'
FISHYSTUFF_PUBLIC_SITE_BASE_URL='https://beta.fishystuff.fish'
FISHYSTUFF_PUBLIC_CDN_BASE_URL='https://cdn.beta.fishystuff.fish'
FISHYSTUFF_RUNTIME_CDN_BASE_URL='https://cdn.beta.fishystuff.fish'
EOF
expect_fail_contains \
  "reject bad API runtime env" \
  "production or shared deployment material" \
  env \
    FISHYSTUFF_GITOPS_BETA_SERVICE_START_PLAN_ALLOW_ENV_FILE_FIXTURE=1 \
    bash scripts/recipes/gitops-beta-service-start-plan.sh \
      "$api_bundle" \
      "$dolt_bundle" \
      "$bad_api_env" \
      "$dolt_env"

expect_fail_contains \
  "reject missing Dolt runtime env" \
  "beta dolt runtime env file does not exist" \
  env \
    FISHYSTUFF_GITOPS_BETA_SERVICE_START_PLAN_ALLOW_ENV_FILE_FIXTURE=1 \
    bash scripts/recipes/gitops-beta-service-start-plan.sh \
      "$api_bundle" \
      "$dolt_bundle" \
      "$api_env" \
      "${root}/missing-dolt.env"

printf '[gitops-beta-service-start-plan-test] %s checks passed\n' "$pass_count"
