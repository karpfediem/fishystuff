#!/usr/bin/env bash
set -euo pipefail

SCRIPT_PATH="$(readlink -f "${BASH_SOURCE[0]}")"
SCRIPT_DIR="$(cd "$(dirname "$SCRIPT_PATH")" && pwd)"
source "${SCRIPT_DIR}/lib/common.sh"

cd "$RECIPE_REPO_ROOT"

pass_count=0

pass() {
  printf '[gitops-beta-start-services-test] pass: %s\n' "$1"
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
    printf '[gitops-beta-start-services-test] expected failure: %s\n' "$name" >&2
    exit 1
  fi
  if ! grep -F "$expected" "$stderr" >/dev/null; then
    printf '[gitops-beta-start-services-test] expected %s stderr to contain %q\n' "$name" "$expected" >&2
    cat "$stderr" >&2
    exit 1
  fi
  pass "$name"
}

write_fake_install() {
  local path="$1"

  cat >"$path" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail

root="${FISHYSTUFF_FAKE_INSTALL_ROOT:?}"
log="${FISHYSTUFF_FAKE_INSTALL_LOG:?}"
printf '%s\n' "$*" >>"$log"
if [[ "$#" -ne 5 || "$1" != "-D" || "$2" != "-m" || "$3" != "0644" ]]; then
  echo "unexpected fake install args: $*" >&2
  exit 2
fi
source_path="$4"
target_path="$5"
case "$target_path" in
  /etc/systemd/system/fishystuff-beta-dolt.service | \
  /etc/systemd/system/fishystuff-beta-api.service)
    ;;
  *)
    echo "fake install saw non-beta target: ${target_path}" >&2
    exit 2
    ;;
esac
mkdir -p "${root}$(dirname "$target_path")"
cp "$source_path" "${root}${target_path}"
EOF
  chmod +x "$path"
}

write_fake_systemctl() {
  local path="$1"

  cat >"$path" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail

log="${FISHYSTUFF_FAKE_SYSTEMCTL_LOG:?}"
printf '%s\n' "$*" >>"$log"
case "$*" in
  daemon-reload | \
  "restart fishystuff-beta-dolt.service" | \
  "is-active --quiet fishystuff-beta-dolt.service" | \
  "restart fishystuff-beta-api.service" | \
  "is-active --quiet fishystuff-beta-api.service")
    ;;
  *)
    echo "unexpected fake systemctl args: $*" >&2
    exit 2
    ;;
esac
EOF
  chmod +x "$path"
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
      printf '[gitops-beta-start-services-test] unsupported fixture service: %s\n' "$service" >&2
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
summary="${root}/beta-current.handoff-summary.json"
fake_install="${root}/install"
fake_systemctl="${root}/systemctl"

make_beta_service_bundle "$api_bundle" api
make_beta_service_bundle "$dolt_bundle" dolt
write_fake_install "$fake_install"
write_fake_systemctl "$fake_systemctl"
read -r api_unit_sha256 _ < <(sha256sum "${api_bundle}/artifacts/systemd/unit")
read -r dolt_unit_sha256 _ < <(sha256sum "${dolt_bundle}/artifacts/systemd/unit")
jq -n \
  --arg api_bundle "$api_bundle" \
  --arg dolt_bundle "$dolt_bundle" \
  '{
    environment: {
      name: "beta"
    },
    active_release: {
      closures: {
        api: $api_bundle,
        dolt_service: $dolt_bundle
      }
    }
  }' >"$summary"

env \
  FISHYSTUFF_GITOPS_ENABLE_BETA_API_RUNTIME_ENV_WRITE=1 \
  FISHYSTUFF_GITOPS_BETA_API_DATABASE_URL="mysql://fishy:secret@127.0.0.1:3316/fishystuff" \
  bash scripts/recipes/gitops-beta-write-runtime-env.sh api "$api_env" >/dev/null
env \
  FISHYSTUFF_GITOPS_ENABLE_BETA_DOLT_RUNTIME_ENV_WRITE=1 \
  bash scripts/recipes/gitops-beta-write-runtime-env.sh dolt "$dolt_env" >/dev/null

expect_fail_contains \
  "reject missing sequence opt-in" \
  "gitops-beta-start-services requires FISHYSTUFF_GITOPS_ENABLE_BETA_SERVICE_START=1" \
  bash scripts/recipes/gitops-beta-start-services.sh

expect_fail_contains \
  "reject stale reviewed API hash" \
  "FISHYSTUFF_GITOPS_BETA_API_UNIT_SHA256 does not match beta API unit hash from start plan" \
  env \
    FISHYSTUFF_GITOPS_ENABLE_BETA_SERVICE_START=1 \
    FISHYSTUFF_GITOPS_ENABLE_BETA_DOLT_INSTALL=1 \
    FISHYSTUFF_GITOPS_ENABLE_BETA_DOLT_RESTART=1 \
    FISHYSTUFF_GITOPS_ENABLE_BETA_API_INSTALL=1 \
    FISHYSTUFF_GITOPS_ENABLE_BETA_API_RESTART=1 \
    FISHYSTUFF_GITOPS_BETA_DOLT_UNIT_SHA256="$dolt_unit_sha256" \
    FISHYSTUFF_GITOPS_BETA_API_UNIT_SHA256=wrong \
    FISHYSTUFF_GITOPS_BETA_SERVICE_START_PLAN_ALLOW_ENV_FILE_FIXTURE=1 \
    bash scripts/recipes/gitops-beta-start-services.sh \
      "$api_bundle" \
      "$dolt_bundle" \
      "$api_env" \
      "$dolt_env" \
      "$fake_install" \
      "$fake_systemctl"

env \
  FISHYSTUFF_GITOPS_ENABLE_BETA_SERVICE_START=1 \
  FISHYSTUFF_GITOPS_ENABLE_BETA_DOLT_INSTALL=1 \
  FISHYSTUFF_GITOPS_ENABLE_BETA_DOLT_RESTART=1 \
  FISHYSTUFF_GITOPS_ENABLE_BETA_API_INSTALL=1 \
  FISHYSTUFF_GITOPS_ENABLE_BETA_API_RESTART=1 \
  FISHYSTUFF_GITOPS_BETA_DOLT_UNIT_SHA256="$dolt_unit_sha256" \
  FISHYSTUFF_GITOPS_BETA_API_UNIT_SHA256="$api_unit_sha256" \
  FISHYSTUFF_GITOPS_BETA_SERVICE_START_PLAN_ALLOW_ENV_FILE_FIXTURE=1 \
  FISHYSTUFF_FAKE_INSTALL_ROOT="${root}/fs" \
  FISHYSTUFF_FAKE_INSTALL_LOG="${root}/install.log" \
  FISHYSTUFF_FAKE_SYSTEMCTL_LOG="${root}/systemctl.log" \
  bash scripts/recipes/gitops-beta-start-services.sh \
    "$api_bundle" \
    "$dolt_bundle" \
    "$api_env" \
    "$dolt_env" \
    "$fake_install" \
    "$fake_systemctl" \
  >"${root}/start.stdout"

grep -F "gitops_beta_service_start_ok=true" "${root}/start.stdout" >/dev/null
grep -F "gitops_beta_service_start_step_01=dolt" "${root}/start.stdout" >/dev/null
grep -F "gitops_beta_service_start_step_02=api" "${root}/start.stdout" >/dev/null
grep -F "gitops_beta_dolt_service_install_ok=fishystuff-beta-dolt.service" "${root}/start.stdout" >/dev/null
grep -F "gitops_beta_api_service_install_ok=fishystuff-beta-api.service" "${root}/start.stdout" >/dev/null
grep -F "remote_deploy_performed=false" "${root}/start.stdout" >/dev/null
grep -F "infrastructure_mutation_performed=false" "${root}/start.stdout" >/dev/null
first_install="$(sed -n '1p' "${root}/install.log")"
second_install="$(sed -n '2p' "${root}/install.log")"
if [[ "$first_install" != *"/etc/systemd/system/fishystuff-beta-dolt.service"* ]]; then
  printf '[gitops-beta-start-services-test] expected Dolt install first, got: %s\n' "$first_install" >&2
  exit 1
fi
if [[ "$second_install" != *"/etc/systemd/system/fishystuff-beta-api.service"* ]]; then
  printf '[gitops-beta-start-services-test] expected API install second, got: %s\n' "$second_install" >&2
  exit 1
fi
sed -n '1p' "${root}/systemctl.log" | grep -Fx "daemon-reload" >/dev/null
sed -n '2p' "${root}/systemctl.log" | grep -Fx "restart fishystuff-beta-dolt.service" >/dev/null
sed -n '3p' "${root}/systemctl.log" | grep -Fx "is-active --quiet fishystuff-beta-dolt.service" >/dev/null
sed -n '4p' "${root}/systemctl.log" | grep -Fx "daemon-reload" >/dev/null
sed -n '5p' "${root}/systemctl.log" | grep -Fx "restart fishystuff-beta-api.service" >/dev/null
sed -n '6p' "${root}/systemctl.log" | grep -Fx "is-active --quiet fishystuff-beta-api.service" >/dev/null
pass "valid beta service start sequence"

: >"${root}/install.log"
: >"${root}/systemctl.log"
env \
  FISHYSTUFF_GITOPS_ENABLE_BETA_SERVICE_START=1 \
  FISHYSTUFF_GITOPS_ENABLE_BETA_DOLT_INSTALL=1 \
  FISHYSTUFF_GITOPS_ENABLE_BETA_DOLT_RESTART=1 \
  FISHYSTUFF_GITOPS_ENABLE_BETA_API_INSTALL=1 \
  FISHYSTUFF_GITOPS_ENABLE_BETA_API_RESTART=1 \
  FISHYSTUFF_GITOPS_BETA_DOLT_UNIT_SHA256="$dolt_unit_sha256" \
  FISHYSTUFF_GITOPS_BETA_API_UNIT_SHA256="$api_unit_sha256" \
  FISHYSTUFF_GITOPS_BETA_SERVICE_START_PLAN_ALLOW_ENV_FILE_FIXTURE=1 \
  FISHYSTUFF_FAKE_INSTALL_ROOT="${root}/fs-auto" \
  FISHYSTUFF_FAKE_INSTALL_LOG="${root}/install.log" \
  FISHYSTUFF_FAKE_SYSTEMCTL_LOG="${root}/systemctl.log" \
  bash scripts/recipes/gitops-beta-start-services.sh \
    auto \
    auto \
    "$api_env" \
    "$dolt_env" \
    "$fake_install" \
    "$fake_systemctl" \
    "$summary" \
  >"${root}/auto-start.stdout"

grep -F "gitops_beta_service_start_ok=true" "${root}/auto-start.stdout" >/dev/null
grep -F "gitops_beta_service_start_api_bundle=${api_bundle}" "${root}/auto-start.stdout" >/dev/null
grep -F "gitops_beta_service_start_dolt_bundle=${dolt_bundle}" "${root}/auto-start.stdout" >/dev/null
sed -n '1p' "${root}/systemctl.log" | grep -Fx "daemon-reload" >/dev/null
sed -n '2p' "${root}/systemctl.log" | grep -Fx "restart fishystuff-beta-dolt.service" >/dev/null
sed -n '5p' "${root}/systemctl.log" | grep -Fx "restart fishystuff-beta-api.service" >/dev/null
pass "start services resolves auto bundles from handoff summary"

if grep -E 'fishystuff-api\.service|fishystuff-dolt\.service|/run/fishystuff/api/env|https://api\.fishystuff\.fish|https://cdn\.fishystuff\.fish' "${root}/start.stdout" >/dev/null; then
  printf '[gitops-beta-start-services-test] beta service start leaked production/shared service material\n' >&2
  exit 1
fi
pass "no production service material in beta start output"

printf '[gitops-beta-start-services-test] %s checks passed\n' "$pass_count"
