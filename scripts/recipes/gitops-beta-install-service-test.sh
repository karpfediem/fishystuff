#!/usr/bin/env bash
set -euo pipefail

SCRIPT_PATH="$(readlink -f "${BASH_SOURCE[0]}")"
SCRIPT_DIR="$(cd "$(dirname "$SCRIPT_PATH")" && pwd)"
source "${SCRIPT_DIR}/lib/common.sh"

cd "$RECIPE_REPO_ROOT"

pass_count=0

pass() {
  printf '[gitops-beta-install-service-test] pass: %s\n' "$1"
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
    printf '[gitops-beta-install-service-test] expected failure: %s\n' "$name" >&2
    exit 1
  fi
  if ! grep -F "$expected" "$stderr" >/dev/null; then
    printf '[gitops-beta-install-service-test] expected %s stderr to contain %q\n' "$name" "$expected" >&2
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
  /etc/systemd/system/fishystuff-beta-api.service | \
  /etc/systemd/system/fishystuff-beta-dolt.service)
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
  "restart fishystuff-beta-api.service" | \
  "is-active --quiet fishystuff-beta-api.service" | \
  "restart fishystuff-beta-dolt.service" | \
  "is-active --quiet fishystuff-beta-dolt.service")
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
  local exe_real=""
  local config_real=""
  local systemd_unit_real=""

  case "$service" in
    api)
      service_id="fishystuff-beta-api"
      unit_name="fishystuff-beta-api.service"
      config_destination="config.toml"
      runtime_env_target="/var/lib/fishystuff/gitops-beta/api/beta.env"
      ;;
    dolt)
      service_id="fishystuff-beta-dolt"
      unit_name="fishystuff-beta-dolt.service"
      config_destination="sql-server.yaml"
      runtime_env_target="/var/lib/fishystuff/gitops-beta/dolt/beta.env"
      ;;
    *)
      printf '[gitops-beta-install-service-test] unsupported fixture service: %s\n' "$service" >&2
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
EnvironmentFile=${runtime_env_target}
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
fake_install="${root}/install"
fake_systemctl="${root}/systemctl"
fake_install_root="${root}/fake-install-root"
fake_install_log="${root}/fake-install.log"
fake_systemctl_log="${root}/fake-systemctl.log"
make_beta_service_bundle "$api_bundle" api
make_beta_service_bundle "$dolt_bundle" dolt
write_fake_install "$fake_install"
write_fake_systemctl "$fake_systemctl"
touch "$fake_install_log" "$fake_systemctl_log"
read -r api_unit_sha256 _ < <(sha256sum "${api_bundle}/artifacts/systemd/unit")
read -r dolt_unit_sha256 _ < <(sha256sum "${dolt_bundle}/artifacts/systemd/unit")
base_install_env=(
  env
  FISHYSTUFF_FAKE_INSTALL_ROOT="$fake_install_root"
  FISHYSTUFF_FAKE_INSTALL_LOG="$fake_install_log"
  FISHYSTUFF_FAKE_SYSTEMCTL_LOG="$fake_systemctl_log"
)

bash scripts/recipes/gitops-check-beta-service-bundle.sh api "$api_bundle" >"${root}/check-api.stdout"
grep -F "gitops_beta_service_bundle_service=api" "${root}/check-api.stdout" >/dev/null
grep -F "gitops_beta_service_bundle_unit_name=fishystuff-beta-api.service" "${root}/check-api.stdout" >/dev/null
grep -F "gitops_beta_api_service_bundle_unit_sha256=${api_unit_sha256}" "${root}/check-api.stdout" >/dev/null
pass "valid beta API service bundle check"

bash scripts/recipes/gitops-check-beta-service-bundle.sh dolt "$dolt_bundle" >"${root}/check-dolt.stdout"
grep -F "gitops_beta_service_bundle_service=dolt" "${root}/check-dolt.stdout" >/dev/null
grep -F "gitops_beta_service_bundle_unit_name=fishystuff-beta-dolt.service" "${root}/check-dolt.stdout" >/dev/null
grep -F "gitops_beta_dolt_service_bundle_unit_sha256=${dolt_unit_sha256}" "${root}/check-dolt.stdout" >/dev/null
pass "valid beta Dolt service bundle check"

bad_api_bundle="${root}/bad-api-bundle"
make_beta_service_bundle "$bad_api_bundle" api
printf '\nWants=fishystuff-api.service\n' >>"${bad_api_bundle}/artifacts/systemd/unit"
expect_fail_contains \
  "reject production API service unit" \
  "production service name" \
  bash scripts/recipes/gitops-check-beta-service-bundle.sh api "$bad_api_bundle"

bad_dolt_bundle="${root}/bad-dolt-bundle"
make_beta_service_bundle "$bad_dolt_bundle" dolt
perl -0pi -e 's#/var/lib/fishystuff/beta-dolt#/var/lib/fishystuff/dolt#g' "${bad_dolt_bundle}/artifacts/config/base"
expect_fail_contains \
  "reject production Dolt data path" \
  "production Dolt data directory" \
  bash scripts/recipes/gitops-check-beta-service-bundle.sh dolt "$bad_dolt_bundle"

expect_fail_contains \
  "refuse API install without opt-in" \
  "gitops-beta-install-service requires FISHYSTUFF_GITOPS_ENABLE_BETA_API_INSTALL=1" \
  "${base_install_env[@]}" \
    bash scripts/recipes/gitops-beta-install-service.sh \
      api \
      "$api_bundle" \
      "$fake_install" \
      "$fake_systemctl"

expect_fail_contains \
  "refuse API restart without opt-in" \
  "gitops-beta-install-service requires FISHYSTUFF_GITOPS_ENABLE_BETA_API_RESTART=1" \
  "${base_install_env[@]}" \
    FISHYSTUFF_GITOPS_ENABLE_BETA_API_INSTALL=1 \
    bash scripts/recipes/gitops-beta-install-service.sh \
      api \
      "$api_bundle" \
      "$fake_install" \
      "$fake_systemctl"

expect_fail_contains \
  "refuse API install without unit hash" \
  "gitops-beta-install-service requires FISHYSTUFF_GITOPS_BETA_API_UNIT_SHA256" \
  "${base_install_env[@]}" \
    FISHYSTUFF_GITOPS_ENABLE_BETA_API_INSTALL=1 \
    FISHYSTUFF_GITOPS_ENABLE_BETA_API_RESTART=1 \
    bash scripts/recipes/gitops-beta-install-service.sh \
      api \
      "$api_bundle" \
      "$fake_install" \
      "$fake_systemctl"

expect_fail_contains \
  "refuse stale API unit hash" \
  "FISHYSTUFF_GITOPS_BETA_API_UNIT_SHA256 does not match beta api systemd unit" \
  "${base_install_env[@]}" \
    FISHYSTUFF_GITOPS_ENABLE_BETA_API_INSTALL=1 \
    FISHYSTUFF_GITOPS_ENABLE_BETA_API_RESTART=1 \
    FISHYSTUFF_GITOPS_BETA_API_UNIT_SHA256=0000000000000000000000000000000000000000000000000000000000000000 \
    bash scripts/recipes/gitops-beta-install-service.sh \
      api \
      "$api_bundle" \
      "$fake_install" \
      "$fake_systemctl"

expect_fail_contains \
  "refuse Dolt install without Dolt hash" \
  "gitops-beta-install-service requires FISHYSTUFF_GITOPS_BETA_DOLT_UNIT_SHA256" \
  "${base_install_env[@]}" \
    FISHYSTUFF_GITOPS_ENABLE_BETA_DOLT_INSTALL=1 \
    FISHYSTUFF_GITOPS_ENABLE_BETA_DOLT_RESTART=1 \
    FISHYSTUFF_GITOPS_BETA_API_UNIT_SHA256="$api_unit_sha256" \
    bash scripts/recipes/gitops-beta-install-service.sh \
      dolt \
      "$dolt_bundle" \
      "$fake_install" \
      "$fake_systemctl"

: >"$fake_install_log"
: >"$fake_systemctl_log"
FISHYSTUFF_GITOPS_ENABLE_BETA_API_INSTALL=1 \
FISHYSTUFF_GITOPS_ENABLE_BETA_API_RESTART=1 \
FISHYSTUFF_GITOPS_BETA_API_UNIT_SHA256="$api_unit_sha256" \
  "${base_install_env[@]}" \
    bash scripts/recipes/gitops-beta-install-service.sh \
      api \
      "$api_bundle" \
      "$fake_install" \
      "$fake_systemctl" >"${root}/install-api.stdout"
grep -F "gitops_beta_service_install_ok=fishystuff-beta-api.service" "${root}/install-api.stdout" >/dev/null
grep -F "gitops_beta_api_service_install_ok=fishystuff-beta-api.service" "${root}/install-api.stdout" >/dev/null
grep -F "gitops_beta_service_install_unit_sha256=${api_unit_sha256}" "${root}/install-api.stdout" >/dev/null
cmp "${api_bundle}/artifacts/systemd/unit" "${fake_install_root}/etc/systemd/system/fishystuff-beta-api.service" >/dev/null
grep -Fx "restart fishystuff-beta-api.service" "$fake_systemctl_log" >/dev/null
grep -Fx "is-active --quiet fishystuff-beta-api.service" "$fake_systemctl_log" >/dev/null
pass "valid beta API install gate"

: >"$fake_install_log"
: >"$fake_systemctl_log"
FISHYSTUFF_GITOPS_ENABLE_BETA_DOLT_INSTALL=1 \
FISHYSTUFF_GITOPS_ENABLE_BETA_DOLT_RESTART=1 \
FISHYSTUFF_GITOPS_BETA_DOLT_UNIT_SHA256="$dolt_unit_sha256" \
  "${base_install_env[@]}" \
    bash scripts/recipes/gitops-beta-install-service.sh \
      dolt \
      "$dolt_bundle" \
      "$fake_install" \
      "$fake_systemctl" >"${root}/install-dolt.stdout"
grep -F "gitops_beta_service_install_ok=fishystuff-beta-dolt.service" "${root}/install-dolt.stdout" >/dev/null
grep -F "gitops_beta_dolt_service_install_ok=fishystuff-beta-dolt.service" "${root}/install-dolt.stdout" >/dev/null
grep -F "gitops_beta_service_install_unit_sha256=${dolt_unit_sha256}" "${root}/install-dolt.stdout" >/dev/null
cmp "${dolt_bundle}/artifacts/systemd/unit" "${fake_install_root}/etc/systemd/system/fishystuff-beta-dolt.service" >/dev/null
grep -Fx "restart fishystuff-beta-dolt.service" "$fake_systemctl_log" >/dev/null
grep -Fx "is-active --quiet fishystuff-beta-dolt.service" "$fake_systemctl_log" >/dev/null
pass "valid beta Dolt install gate"

if grep -F "fishystuff-api.service" "$fake_systemctl_log" "${root}/install-api.stdout" "${root}/install-dolt.stdout" >/dev/null; then
  printf '[gitops-beta-install-service-test] beta service install unexpectedly mentioned production API unit\n' >&2
  exit 1
fi
if grep -F "fishystuff-dolt.service" "$fake_systemctl_log" "${root}/install-api.stdout" "${root}/install-dolt.stdout" >/dev/null; then
  printf '[gitops-beta-install-service-test] beta service install unexpectedly mentioned production Dolt unit\n' >&2
  exit 1
fi
pass "no production service units in beta installs"

printf '[gitops-beta-install-service-test] %s checks passed\n' "$pass_count"
