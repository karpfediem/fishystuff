# Start the full local dev server stack
[default]
up:
  devenv up --no-tui

# Start the local dev stack plus rebuild/restart watchers
watch:
  devenv up --profile watch --no-tui

# Open a local service UI in the default browser
open target:
  bash -eu -c 'target="$1"; case "$target" in site) url="http://127.0.0.1:1990/" ;; map) url="http://127.0.0.1:1990/map/" ;; api) url="http://127.0.0.1:8080/api/v1/meta" ;; cdn) url="http://127.0.0.1:4040/" ;; jaeger) url="http://127.0.0.1:16686/" ;; grafana|logs|loki) url="http://127.0.0.1:3000/explore" ;; dashboard|grafana-dashboard) url="http://127.0.0.1:3000/d/fishystuff-operator-overview/fishystuff-operator-overview" ;; dashboard-local|grafana-dashboard-local) url="http://127.0.0.1:3000/d/fishystuff-local-observability/fishystuff-local-observability" ;; loki-status) url="http://127.0.0.1:3100/services" ;; prometheus) url="http://127.0.0.1:9090/" ;; vector) url="http://127.0.0.1:8686/playground" ;; *) echo "unknown open target: $target" >&2; echo "available targets: site map api cdn jaeger grafana dashboard dashboard-local logs loki loki-status prometheus vector" >&2; exit 2 ;; esac; exec xdg-open "$url"' -- "{{target}}"

# Initialize a clone of our dolt database on http://dolthub.com/repositories/fishystuff/fishystuff
clone-db:
    dolt clone fishystuff/fishystuff .

# Starts a local MySQL server using Dolt
serve-db:
    dolt sql-server

# Replaces the current Fishing_Table with the one obtained from a (new) Fishing_Table.xlsx file in the current directory
update_fishing_table:
    xlsx2csv Fishing_Table.xlsx table.csv
    awk 'BEGIN{FS=OFS=","} NR==1{print "index", $0} NR>1{print NR-1, $0}' table.csv > indexed.csv
    dolt table import --replace-table "indexed" "indexed.csv"
    dolt sql -c < sql/update_zone_index.sql
    rm table.csv indexed.csv


# Build and deploy the discord bot
deploy-bot:
  skopeo --insecure-policy --debug copy docker-archive:"$(nix build .#bot-container --no-link --print-out-paths)" docker://registry.fly.io/criobot:latest --dest-creds x:"$(fly -a criobot tokens create deploy --expiry 10m)" --format v2s2
  flyctl deploy --remote-only -c bot/fly.toml

# Build and deploy the Axum API
deploy-api:
  skopeo --insecure-policy --debug copy docker-archive:"$(nix build .#api-container --no-link --print-out-paths)" docker://registry.fly.io/api-fishystuff-fish:latest --dest-creds x:"$(fly -a api-fishystuff-fish tokens create deploy --expiry 10m)" --format v2s2
  flyctl deploy --remote-only --smoke-checks=false --wait-timeout 10m -c api/fly.toml

# Run the Discord bot with the SecretSpec bot profile
bot-run:
  secretspec run --profile bot -- cargo run --manifest-path bot/Cargo.toml

# Stage CDN-served runtime assets under data/cdn/public
cdn-stage:
  ./tools/scripts/stage_cdn_assets.sh

# Rebuild source-backed CDN item icons only
cdn-stage-icons:
  node tools/scripts/build_item_icons_from_source.mjs --output-dir data/cdn/public/images/items

# Push the staged CDN tree to Bunny Storage via HTTP API.
# Override BUNNY_STORAGE_PARALLEL (or legacy BUNNY_FTP_PARALLEL) in the shell if needed.
cdn-push:
  secretspec run --profile cdn -- ./tools/scripts/push_bunnycdn.sh

# Compute the exact CDN filenames required by the current deployment inputs.
cdn-required-files out="data/cdn/required-files.json":
  ./tools/scripts/compute_required_cdn_filenames.sh --out "{{out}}"

# Refresh the staged tree and then push it to Bunny Storage
cdn-sync:
  just cdn-stage
  just cdn-push

# Build the map runtime, refresh staged map assets, and push only the CDN map root.
cdn-sync-map:
  ./tools/scripts/build_map.sh
  ./tools/scripts/stage_cdn_assets.sh --map-only
  BUNNY_SYNC_ROOTS=map secretspec run --profile cdn -- ./tools/scripts/push_bunnycdn.sh

# Validate that the local SecretSpec provider has the required values for a profile
secrets-check profile="api":
  p='{{profile}}'; p="${p#profile=}"; secretspec check --profile "$p"

# Type-check the local mgmt Hetzner beta topology module
mgmt-beta-unify mgmt_bin="../result/bin/mgmt":
  #!/usr/bin/env bash
  set -euo pipefail
  mgmt_bin='{{mgmt_bin}}'
  mgmt_bin="${mgmt_bin#mgmt_bin=}"
  cd mgmt
  "$mgmt_bin" run lang --only-unify main.mcl

# Run the local mgmt Hetzner beta topology bootstrap as a one-shot converging apply.
# Default state is absent for safety; use state=running to request server creation.
mgmt-beta-bootstrap state="absent" converged_timeout="30" mgmt_bin="../result/bin/mgmt" client_urls="http://127.0.0.1:3379" server_urls="http://127.0.0.1:3380" prometheus="false" prometheus_listen="127.0.0.1:9233" pprof_path="":
  #!/usr/bin/env bash
  set -euo pipefail
  state='{{state}}'
  state="${state#state=}"
  converged_timeout='{{converged_timeout}}'
  converged_timeout="${converged_timeout#converged_timeout=}"
  mgmt_bin='{{mgmt_bin}}'
  mgmt_bin="${mgmt_bin#mgmt_bin=}"
  client_urls='{{client_urls}}'
  client_urls="${client_urls#client_urls=}"
  server_urls='{{server_urls}}'
  server_urls="${server_urls#server_urls=}"
  prometheus='{{prometheus}}'
  prometheus="${prometheus#prometheus=}"
  prometheus_listen='{{prometheus_listen}}'
  prometheus_listen="${prometheus_listen#prometheus_listen=}"
  pprof_path='{{pprof_path}}'
  pprof_path="${pprof_path#pprof_path=}"
  FISHYSTUFF_HETZNER_STATE="$state" \
    secretspec run --profile beta-deploy -- \
    bash -lc '
      set -euo pipefail
      cd mgmt
      cmd=(
        "$1" run
        --client-urls="$2"
        --server-urls="$3"
        --advertise-client-urls="$2"
        --advertise-server-urls="$3"
      )
      case "$5" in
        true|1|yes)
          cmd+=(--prometheus --prometheus-listen "$6")
          ;;
      esac
      cmd+=(lang --tmp-prefix --no-watch --converged-timeout "$4" main.mcl)
      if [[ -n "$7" ]]; then
        export MGMT_PPROF_PATH="$7"
      fi
      "${cmd[@]}"
    ' \
    -- "$mgmt_bin" "$client_urls" "$server_urls" "$converged_timeout" "$prometheus" "$prometheus_listen" "$pprof_path"

# Type-check the resident bootstrap graph used to install a host-local mgmt service.
mgmt-resident-bootstrap-unify mgmt_bin="../result/bin/mgmt":
  #!/usr/bin/env bash
  set -euo pipefail
  mgmt_bin='{{mgmt_bin}}'
  mgmt_bin="${mgmt_bin#mgmt_bin=}"
  cd mgmt/resident-bootstrap
  "$mgmt_bin" run lang --only-unify main.mcl

# Copy a locally built mgmt closure to a remote host and install the resident service there.
mgmt-resident-kickstart-remote target="mgmt-root" host="mgmt-root" timeout="120" mgmt_flake="/home/carp/code/mgmt":
  #!/usr/bin/env bash
  set -euo pipefail
  target='{{target}}'
  target="${target#target=}"
  host='{{host}}'
  host="${host#host=}"
  timeout='{{timeout}}'
  timeout="${timeout#timeout=}"
  mgmt_flake='{{mgmt_flake}}'
  mgmt_flake="${mgmt_flake#mgmt_flake=}"
  mgmt_store="$(nix build "$mgmt_flake" --no-link --print-out-paths)"
  secretspec run --profile beta-deploy -- \
    bash -lc '
      set -euo pipefail
      tmp_key="$(mktemp /tmp/fishystuff-mgmt-ssh.XXXXXX)"
      trap '\''rm -f "$tmp_key"'\'' EXIT
      umask 077
      printf "%s\n" "$HETZNER_SSH_PRIVATE_KEY" > "$tmp_key"
      chmod 600 "$tmp_key"
      nix copy --to "ssh-ng://$1?ssh-key=$tmp_key" "$4"
      SSH_OPTS="-i $tmp_key -o IdentitiesOnly=yes" \
        bash mgmt/scripts/kickstart-fishystuff-resident-remote.sh \
          mgmt/resident-bootstrap \
          "$1" \
          "$2" \
          "$3" \
          "$4/bin/mgmt"
    ' \
    -- "$target" "$host" "$timeout" "$mgmt_store"

# Push a self-contained graph directory into the resident mgmt instance on a remote host.
mgmt-resident-deploy-remote target="mgmt-root" dir="mgmt/resident-deploy-probe" timeout="120" remote_mgmt_bin="/usr/local/bin/mgmt":
  #!/usr/bin/env bash
  set -euo pipefail
  target='{{target}}'
  target="${target#target=}"
  dir='{{dir}}'
  dir="${dir#dir=}"
  timeout='{{timeout}}'
  timeout="${timeout#timeout=}"
  remote_mgmt_bin='{{remote_mgmt_bin}}'
  remote_mgmt_bin="${remote_mgmt_bin#remote_mgmt_bin=}"
  secretspec run --profile beta-deploy -- \
    bash -lc '
      set -euo pipefail
      tmp_key="$(mktemp /tmp/fishystuff-mgmt-ssh.XXXXXX)"
      trap '\''rm -f "$tmp_key"'\'' EXIT
      umask 077
      printf "%s\n" "$HETZNER_SSH_PRIVATE_KEY" > "$tmp_key"
      chmod 600 "$tmp_key"
      SSH_OPTS="-i $tmp_key -o IdentitiesOnly=yes" \
      bash mgmt/scripts/deploy-fishystuff-resident-remote.sh \
          "$1" \
          "$2" \
          "$3" \
          "$4"
    ' \
    -- "$dir" "$target" "$timeout" "$remote_mgmt_bin"

# Build the API and Dolt service bundles locally, push both closures to a
# remote host, root them at stable GC-root paths, and deploy the resident beta
# graph for the current API/DB host shape.
mgmt-resident-push-api-db target="mgmt-root" host="beta-nbg1-api-db" timeout="120" remote_mgmt_bin="/usr/local/bin/mgmt" api_gcroot="/nix/var/nix/gcroots/mgmt/fishystuff/api-current" dolt_gcroot="/nix/var/nix/gcroots/mgmt/fishystuff/dolt-current":
  #!/usr/bin/env bash
  set -euo pipefail
  target='{{target}}'
  target="${target#target=}"
  host='{{host}}'
  host="${host#host=}"
  timeout='{{timeout}}'
  timeout="${timeout#timeout=}"
  remote_mgmt_bin='{{remote_mgmt_bin}}'
  remote_mgmt_bin="${remote_mgmt_bin#remote_mgmt_bin=}"
  api_gcroot='{{api_gcroot}}'
  api_gcroot="${api_gcroot#api_gcroot=}"
  dolt_gcroot='{{dolt_gcroot}}'
  dolt_gcroot="${dolt_gcroot#dolt_gcroot=}"
  api_bundle="$(nix build .#api-service-bundle --no-link --print-out-paths)"
  dolt_bundle="$(nix build .#dolt-service-bundle --no-link --print-out-paths)"
  secretspec run --profile beta-deploy -- \
    bash -lc '
      set -euo pipefail
      tmp_key="$(mktemp /tmp/fishystuff-mgmt-ssh.XXXXXX)"
      trap '\''rm -f "$tmp_key"'\'' EXIT
      umask 077
      printf "%s\n" "$HETZNER_SSH_PRIVATE_KEY" > "$tmp_key"
      chmod 600 "$tmp_key"
      export FISHYSTUFF_BETA_HOSTNAME="$2"
      export FISHYSTUFF_API_BUNDLE_PATH="$3"
      export FISHYSTUFF_DOLT_BUNDLE_PATH="$5"
      SSH_OPTS="-i $tmp_key -o IdentitiesOnly=yes" \
      NIX_SSH_KEY_PATH="$tmp_key" \
      bash mgmt/scripts/push-fishystuff-bundles-remote.sh \
          "$1" \
          "$4" \
          "$3" \
          "$6" \
          "$5"
      SSH_OPTS="-i $tmp_key -o IdentitiesOnly=yes" \
      bash mgmt/scripts/deploy-fishystuff-resident-remote.sh \
          mgmt/resident-beta \
          "$1" \
          "$7" \
          "$8"
    ' \
    -- "$target" "$host" "$api_gcroot" "$api_bundle" "$dolt_gcroot" "$dolt_bundle" "$timeout" "$remote_mgmt_bin"

# Build a temporary resident graph that installs a bundle-backed systemd unit
# from a local Nix bundle root, validate it, and deploy it to a resident mgmt
# instance over SSH.
mgmt-resident-dolt-bundle-probe target="mgmt-root" bundle_path="/nix/var/nix/gcroots/mgmt/fishystuff/dolt-current" timeout="120" remote_mgmt_bin="/usr/local/bin/mgmt" mgmt_bin="/home/carp/code/playground/mgmt-missing-features/mgmt":
  #!/usr/bin/env bash
  set -euo pipefail
  target='{{target}}'
  target="${target#target=}"
  bundle_path='{{bundle_path}}'
  bundle_path="${bundle_path#bundle_path=}"
  timeout='{{timeout}}'
  timeout="${timeout#timeout=}"
  remote_mgmt_bin='{{remote_mgmt_bin}}'
  remote_mgmt_bin="${remote_mgmt_bin#remote_mgmt_bin=}"
  mgmt_bin='{{mgmt_bin}}'
  mgmt_bin="${mgmt_bin#mgmt_bin=}"
  probe_dir="$(mktemp -d /tmp/fishystuff-resident-bundle-probe.XXXXXX)"
  trap 'rm -rf "$probe_dir"' EXIT
  mkdir -p "$probe_dir/modules/lib"
  cp -a mgmt/resident-beta/modules/lib/fishystuff-systemd "$probe_dir/modules/lib/"
  cp -a mgmt/resident-beta/modules/lib/fishystuff-bundle-systemd "$probe_dir/modules/lib/"
  printf '%s\n' \
    'import "modules/lib/fishystuff-bundle-systemd/" as fishystuff_bundle_systemd' \
    '' \
    'include fishystuff_bundle_systemd.unit(struct {' \
    "	bundle_path => \"${bundle_path}\"," \
    '	startup_mode => "enabled",' \
    '})' \
    > "$probe_dir/main.mcl"
  printf 'main: main.mcl\n' > "$probe_dir/metadata.yaml"
  "$mgmt_bin" run lang --only-unify "$probe_dir/main.mcl"
  secretspec run --profile beta-deploy -- \
    bash -lc '
      set -euo pipefail
      tmp_key="$(mktemp /tmp/fishystuff-mgmt-ssh.XXXXXX)"
      trap '\''rm -f "$tmp_key"'\'' EXIT
      umask 077
      printf "%s\n" "$HETZNER_SSH_PRIVATE_KEY" > "$tmp_key"
      chmod 600 "$tmp_key"
      SSH_OPTS="-i $tmp_key -o IdentitiesOnly=yes" \
      bash mgmt/scripts/deploy-fishystuff-resident-remote.sh \
          "$1" \
          "$2" \
          "$3" \
          "$4"
    ' \
    -- "$probe_dir" "$target" "$timeout" "$remote_mgmt_bin"

# Build the Dolt service bundle, copy it to a remote host, root it, install the
# rendered unit, and verify that the SQL server answers a local health check.
mgmt-dolt-target-smoke target="mgmt-root" gcroot="/nix/var/nix/gcroots/mgmt/fishystuff/dolt-current" sql_host="127.0.0.1" sql_port="3306" query_timeout="20":
  #!/usr/bin/env bash
  set -euo pipefail
  target='{{target}}'
  target="${target#target=}"
  gcroot='{{gcroot}}'
  gcroot="${gcroot#gcroot=}"
  sql_host='{{sql_host}}'
  sql_host="${sql_host#sql_host=}"
  sql_port='{{sql_port}}'
  sql_port="${sql_port#sql_port=}"
  query_timeout='{{query_timeout}}'
  query_timeout="${query_timeout#query_timeout=}"
  bundle="$(nix build .#dolt-service-bundle --no-link --print-out-paths)"
  secretspec run --profile beta-deploy -- \
    bash -lc '
      set -euo pipefail
      tmp_key="$(mktemp /tmp/fishystuff-dolt-smoke-ssh.XXXXXX)"
      trap '\''rm -f "$tmp_key"'\'' EXIT
      umask 077
      printf "%s\n" "$HETZNER_SSH_PRIVATE_KEY" > "$tmp_key"
      chmod 600 "$tmp_key"
      SSH_OPTS="-i $tmp_key -o IdentitiesOnly=yes" \
      NIX_SSH_KEY_PATH="$tmp_key" \
      bash mgmt/scripts/smoke-fishystuff-dolt-target.sh \
        "$1" \
        "$2" \
        "$3" \
        "$4" \
        "$5" \
        "$6"
    ' \
    -- "$bundle" "$target" "$gcroot" "$sql_host" "$sql_port" "$query_timeout"

# Build the current map runtime and map-serving CDN payload once
build-map:
  ./tools/scripts/build_map.sh
  ./tools/scripts/stage_cdn_assets.sh --map-only

# Build the current site output once
build-site:
  cd site && just build-release

# Build the current local dev outputs once
build:
  #!/usr/bin/env bash
  set -euo pipefail

  format_elapsed() {
    local elapsed="${1:-0}"
    if (( elapsed >= 60 )); then
      printf '%dm%02ds' "$((elapsed / 60))" "$((elapsed % 60))"
      return
    fi
    printf '%ds' "$elapsed"
  }

  run_step() {
    local label="$1"
    shift

    local started="$SECONDS"
    echo "[build] starting ${label}"
    if "$@"; then
      local elapsed="$((SECONDS - started))"
      echo "[build] finished ${label} in $(format_elapsed "$elapsed")"
      return 0
    else
      local status="$?"
      local elapsed="$((SECONDS - started))"
      echo "[build] failed ${label} after $(format_elapsed "$elapsed")" >&2
      return "$status"
    fi
  }

  wait_step() {
    local label="$1"
    local pid="$2"
    local started="$3"

    echo "[build] waiting for ${label}"
    if wait "$pid"; then
      local elapsed="$((SECONDS - started))"
      echo "[build] finished ${label} in $(format_elapsed "$elapsed")"
      return 0
    else
      local status="$?"
      local elapsed="$((SECONDS - started))"
      echo "[build] failed ${label} after $(format_elapsed "$elapsed")" >&2
      return "$status"
    fi
  }

  background_pids=()

  cleanup() {
    local pid
    for pid in "${background_pids[@]}"; do
      kill "$pid" 2>/dev/null || true
    done
  }

  trap cleanup EXIT

  site_started="$SECONDS"
  echo "[build] starting build-site in background"
  just build-site &
  site_pid="$!"
  background_pids+=("$site_pid")

  status=0
  run_step "build-map" just build-map || status=1
  if (( status == 0 )); then
    run_step "cdn-stage-icons" just cdn-stage-icons || status=1
  else
    echo "[build] skipping cdn-stage-icons because build-map failed" >&2
  fi

  if ! wait_step "build-site" "$site_pid" "$site_started"; then
    status=1
  fi

  trap - EXIT
  exit "$status"
