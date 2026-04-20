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
  module_path="/tmp/fishystuff-mgmt-modules/"
  mkdir -p "$module_path"
  cd mgmt/resident-bootstrap
  "$mgmt_bin" run lang --module-path "$module_path" --download --only-unify main.mcl

# Copy a locally built mgmt closure to a remote host and install the resident service there.
mgmt-resident-kickstart-remote target="mgmt-root" host="mgmt-root" timeout="120" mgmt_flake="/home/carp/code/playground/mgmt-missing-features" mgmt_package="minimal":
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
  mgmt_package='{{mgmt_package}}'
  mgmt_package="${mgmt_package#mgmt_package=}"
  mgmt_installable="$mgmt_flake"
  if [[ -n "$mgmt_package" && "$mgmt_package" != "default" ]]; then
    mgmt_installable="$mgmt_flake#$mgmt_package"
  fi
  mgmt_store="$(nix build "$mgmt_installable" --no-link --print-out-paths)"
  secretspec run --profile beta-deploy -- \
    bash -lc '
      set -euo pipefail
      tmp_key="$(mktemp /tmp/fishystuff-mgmt-ssh.XXXXXX)"
      trap '\''rm -f "$tmp_key"'\'' EXIT
      umask 077
      printf "%s\n" "$HETZNER_SSH_PRIVATE_KEY" > "$tmp_key"
      chmod 600 "$tmp_key"
      ssh_opts=(-i "$tmp_key" -o IdentitiesOnly=yes -o StrictHostKeyChecking=accept-new)
      detect_remote_nix() {
        ssh "${ssh_opts[@]}" "$1" '\''\
          nix_path=""
          nix_daemon_path=""
          if test -x /nix/var/nix/profiles/default/bin/nix; then
            nix_path=/nix/var/nix/profiles/default/bin/nix
          elif command -v nix >/dev/null 2>&1; then
            nix_path="$(command -v nix)"
          fi
          if test -x /nix/var/nix/profiles/default/bin/nix-daemon; then
            nix_daemon_path=/nix/var/nix/profiles/default/bin/nix-daemon
          elif command -v nix-daemon >/dev/null 2>&1; then
            nix_daemon_path="$(command -v nix-daemon)"
          fi
          printf "%s\t%s\n" "$nix_path" "$nix_daemon_path"
        '\'' 2>/dev/null || true
      }
      build_nix_copy_target() {
        local target="ssh-ng://$1?ssh-key=$tmp_key"
        if [[ -n "$2" ]]; then
          target="${target}&remote-program=$2"
        fi
        printf "%s" "$target"
      }
      remote_nix_probe="$(detect_remote_nix "$1")"
      remote_nix_path=""
      remote_nix_daemon_path=""
      if [[ -n "$remote_nix_probe" ]]; then
        IFS=$'\''\t'\'' read -r remote_nix_path remote_nix_daemon_path <<<"$remote_nix_probe"
      fi
      nix_copy_target="$(build_nix_copy_target "$1" "$remote_nix_daemon_path")"
      remote_mgmt_bin="$4/bin/mgmt"
      if [[ -n "$remote_nix_path" ]]; then
        nix copy --no-check-sigs --to "$nix_copy_target" "$4"
      else
        (
          cd "$5"
          devenv shell -- bash -lc '\''MGMT_NOCGO=true MGMT_NOGOLANGRACE=true GOTAGS="noaugeas novirt nodocker" make -B build/mgmt-linux-amd64'\''
        )
        cat "$5/build/mgmt-linux-amd64" | ssh "${ssh_opts[@]}" "$1" "sudo install -d -m 0755 /usr/local/bin && sudo tee /usr/local/bin/fishystuff-mgmt-bootstrap >/dev/null && sudo chmod 0755 /usr/local/bin/fishystuff-mgmt-bootstrap"
        remote_mgmt_bin="/usr/local/bin/fishystuff-mgmt-bootstrap"
      fi
      SSH_OPTS="-i $tmp_key -o IdentitiesOnly=yes" \
        bash mgmt/scripts/kickstart-fishystuff-resident-remote.sh \
          mgmt/resident-bootstrap \
          "$1" \
          "$2" \
          "$3" \
          "$remote_mgmt_bin"
      if [[ "$remote_mgmt_bin" != "$4/bin/mgmt" ]]; then
        remote_nix_probe="$(detect_remote_nix "$1")"
        remote_nix_daemon_path=""
        if [[ -n "$remote_nix_probe" ]]; then
          IFS=$'\''\t'\'' read -r _remote_nix_path remote_nix_daemon_path <<<"$remote_nix_probe"
        fi
        if [[ -z "$remote_nix_daemon_path" ]]; then
          echo "could not detect remote nix-daemon path on $1 after bootstrap" >&2
          exit 1
        fi
        nix_copy_target="$(build_nix_copy_target "$1" "$remote_nix_daemon_path")"
        nix copy --no-check-sigs --to "$nix_copy_target" "$4"
        ssh "${ssh_opts[@]}" "$1" "sudo ln -sfn '\''$4/bin/mgmt'\'' /usr/local/bin/mgmt && sudo systemctl daemon-reload && sudo systemctl restart fishystuff-mgmt.service && sudo systemctl is-enabled fishystuff-mgmt.service >/dev/null && sudo systemctl is-active fishystuff-mgmt.service >/dev/null"
      fi
    ' \
    -- "$target" "$host" "$timeout" "$mgmt_store" "$mgmt_flake"

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
# remote host, and deploy the resident beta graph for the current API/DB host
# shape. The resident graph owns GC-root selection via nix:gcroot.
mgmt-resident-push-api-db target="mgmt-root" host="beta-nbg1-api-db" timeout="120" remote_mgmt_bin="/usr/local/bin/mgmt" api_gcroot="/nix/var/nix/gcroots/mgmt/fishystuff/api-current" dolt_gcroot="/nix/var/nix/gcroots/mgmt/fishystuff/dolt-current" mgmt_modules_dir="/home/carp/code/mgmt/modules" remote_nix_max_jobs="0":
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
  mgmt_modules_dir='{{mgmt_modules_dir}}'
  mgmt_modules_dir="${mgmt_modules_dir#mgmt_modules_dir=}"
  remote_nix_max_jobs='{{remote_nix_max_jobs}}'
  remote_nix_max_jobs="${remote_nix_max_jobs#remote_nix_max_jobs=}"
  api_bundle="$(nix build .#api-service-bundle --no-link --print-out-paths)"
  dolt_bundle="$(nix build .#dolt-service-bundle --no-link --print-out-paths)"
  deploy_dir="$(mktemp -d /tmp/fishystuff-resident-beta.XXXXXX)"
  trap 'rm -rf "$deploy_dir"' EXIT
  cp -a mgmt/resident-beta/. "$deploy_dir/"
  mkdir -p "$deploy_dir/files"
  mkdir -p "$deploy_dir/modules/lib"
  cp -a mgmt/modules/lib/systemd-daemon-reload "$deploy_dir/modules/lib/"
  mkdir -p "$deploy_dir/modules/github.com/purpleidea/mgmt/modules"
  cp -a "$mgmt_modules_dir/misc" "$deploy_dir/modules/github.com/purpleidea/mgmt/modules/"
  jq -n \
    --arg cluster "beta" \
    --arg hostname "$host" \
    --arg site_base_url "https://beta.fishystuff.fish" \
    --arg api_base_url "https://api.beta.fishystuff.fish" \
    --arg cdn_base_url "https://cdn.beta.fishystuff.fish" \
    --arg telemetry_base_url "https://telemetry.beta.fishystuff.fish" \
    --arg deployment_environment "beta" \
    --arg startup_mode "enabled" \
    --arg dolt_data_dir "/var/lib/fishystuff/dolt" \
    --arg dolt_cfg_dir "/var/lib/fishystuff/dolt/.doltcfg" \
    --arg dolt_database_name "fishystuff" \
    --arg dolt_remote_url "fishystuff/fishystuff" \
    --arg dolt_remote_branch "main" \
    --arg dolt_clone_depth "1" \
    --arg dolt_volume_device "" \
    --arg dolt_volume_fs_type "ext4" \
    --arg dolt_port "3306" \
    --arg site_root_dir "/srv/fishystuff/site" \
    --arg cdn_root_dir "/srv/fishystuff/cdn" \
    --arg api_bundle "$api_bundle" \
    --arg api_gcroot "$api_gcroot" \
    --arg dolt_bundle "$dolt_bundle" \
    --arg dolt_gcroot "$dolt_gcroot" \
    --arg edge_bundle "" \
    --arg edge_gcroot "/nix/var/nix/gcroots/mgmt/fishystuff/edge-current" \
    --arg loki_bundle "" \
    --arg loki_gcroot "/nix/var/nix/gcroots/mgmt/fishystuff/loki-current" \
    --arg otel_collector_bundle "" \
    --arg otel_collector_gcroot "/nix/var/nix/gcroots/mgmt/fishystuff/otel-collector-current" \
    --arg vector_bundle "" \
    --arg vector_gcroot "/nix/var/nix/gcroots/mgmt/fishystuff/vector-current" \
    --arg prometheus_bundle "" \
    --arg prometheus_gcroot "/nix/var/nix/gcroots/mgmt/fishystuff/prometheus-current" \
    --arg jaeger_bundle "" \
    --arg jaeger_gcroot "/nix/var/nix/gcroots/mgmt/fishystuff/jaeger-current" \
    --arg grafana_bundle "" \
    --arg grafana_gcroot "/nix/var/nix/gcroots/mgmt/fishystuff/grafana-current" \
    '{
      cluster: $cluster,
      hostname: $hostname,
      public_urls: {
        site_base_url: $site_base_url,
        api_base_url: $api_base_url,
        cdn_base_url: $cdn_base_url,
        telemetry_base_url: $telemetry_base_url
      },
      deployment_environment: $deployment_environment,
      startup_mode: $startup_mode,
      dolt: {
        data_dir: $dolt_data_dir,
        cfg_dir: $dolt_cfg_dir,
        database_name: $dolt_database_name,
        remote_url: $dolt_remote_url,
        remote_branch: $dolt_remote_branch,
        clone_depth: $dolt_clone_depth,
        volume_device: $dolt_volume_device,
        volume_fs_type: $dolt_volume_fs_type,
        port: $dolt_port
      },
      content_roots: {
        site_root_dir: $site_root_dir,
        cdn_root_dir: $cdn_root_dir
      },
      services: {
        api: {bundle_path: $api_bundle, gcroot_path: $api_gcroot},
        dolt: {bundle_path: $dolt_bundle, gcroot_path: $dolt_gcroot},
        edge: {bundle_path: $edge_bundle, gcroot_path: $edge_gcroot},
        loki: {bundle_path: $loki_bundle, gcroot_path: $loki_gcroot},
        otel_collector: {bundle_path: $otel_collector_bundle, gcroot_path: $otel_collector_gcroot},
        vector: {bundle_path: $vector_bundle, gcroot_path: $vector_gcroot},
        prometheus: {bundle_path: $prometheus_bundle, gcroot_path: $prometheus_gcroot},
        jaeger: {bundle_path: $jaeger_bundle, gcroot_path: $jaeger_gcroot},
        grafana: {bundle_path: $grafana_bundle, gcroot_path: $grafana_gcroot}
      }
    }' > "$deploy_dir/files/resident-manifest.json"
  secretspec run --profile beta-deploy -- \
    bash -lc '
      set -euo pipefail
      ssh_target="${1:?}"
      deploy_dir="${2:?}"
      deploy_timeout="${3:?}"
      remote_mgmt_bin="${4:-/usr/local/bin/mgmt}"
      if [[ "$remote_mgmt_bin" != /* ]]; then
        remote_mgmt_bin=/usr/local/bin/mgmt
      fi
      remote_nix_max_jobs="${5:?}"
      shift 5
      tmp_key="$(mktemp /tmp/fishystuff-mgmt-ssh.XXXXXX)"
      trap '\''rm -f "$tmp_key"'\'' EXIT
      umask 077
      printf "%s\n" "$HETZNER_SSH_PRIVATE_KEY" > "$tmp_key"
      chmod 600 "$tmp_key"
      remote_nix_daemon_path="$(ssh -i "$tmp_key" -o IdentitiesOnly=yes -o StrictHostKeyChecking=accept-new "$ssh_target" '\''if test -x /nix/var/nix/profiles/default/bin/nix-daemon; then printf "%s" /nix/var/nix/profiles/default/bin/nix-daemon; elif command -v nix-daemon >/dev/null 2>&1; then command -v nix-daemon; fi'\'')"
      if [[ -z "$remote_nix_daemon_path" ]]; then
        echo "could not detect remote nix-daemon path on $ssh_target" >&2
        exit 1
      fi
      SSH_OPTS="-i $tmp_key -o IdentitiesOnly=yes -o StrictHostKeyChecking=accept-new" \
      NIX_SSH_KEY_PATH="$tmp_key" \
      NIX_REMOTE_PROGRAM_PATH="$remote_nix_daemon_path" \
      FISHYSTUFF_REMOTE_NIX_MAX_JOBS="$remote_nix_max_jobs" \
      bash mgmt/scripts/push-fishystuff-bundles-remote.sh \
          "$ssh_target" \
          "$@"
      SSH_OPTS="-i $tmp_key -o IdentitiesOnly=yes -o StrictHostKeyChecking=accept-new" \
      bash mgmt/scripts/deploy-fishystuff-resident-remote.sh \
          "$deploy_dir" \
          "$ssh_target" \
          "$deploy_timeout" \
          "$remote_mgmt_bin"
    ' \
    -- "$target" "$deploy_dir" "$timeout" "$remote_mgmt_bin" "$remote_nix_max_jobs" "$api_bundle" "$dolt_bundle"

# Build the current pure service bundles for the single-host beta stack, push
# them to a remote host, and deploy the resident graph with API, Dolt, edge,
# and observability daemons. The resident graph owns GC-root selection via
# nix:gcroot.
mgmt-resident-push-full-stack target="mgmt-root" host="beta-nbg1-api-db" timeout="180" remote_mgmt_bin="/usr/local/bin/mgmt" api_gcroot="/nix/var/nix/gcroots/mgmt/fishystuff/api-current" dolt_gcroot="/nix/var/nix/gcroots/mgmt/fishystuff/dolt-current" edge_gcroot="/nix/var/nix/gcroots/mgmt/fishystuff/edge-current" loki_gcroot="/nix/var/nix/gcroots/mgmt/fishystuff/loki-current" otel_collector_gcroot="/nix/var/nix/gcroots/mgmt/fishystuff/otel-collector-current" vector_gcroot="/nix/var/nix/gcroots/mgmt/fishystuff/vector-current" prometheus_gcroot="/nix/var/nix/gcroots/mgmt/fishystuff/prometheus-current" jaeger_gcroot="/nix/var/nix/gcroots/mgmt/fishystuff/jaeger-current" grafana_gcroot="/nix/var/nix/gcroots/mgmt/fishystuff/grafana-current" mgmt_modules_dir="/home/carp/code/mgmt/modules" remote_nix_max_jobs="0":
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
  edge_gcroot='{{edge_gcroot}}'
  edge_gcroot="${edge_gcroot#edge_gcroot=}"
  loki_gcroot='{{loki_gcroot}}'
  loki_gcroot="${loki_gcroot#loki_gcroot=}"
  otel_collector_gcroot='{{otel_collector_gcroot}}'
  otel_collector_gcroot="${otel_collector_gcroot#otel_collector_gcroot=}"
  vector_gcroot='{{vector_gcroot}}'
  vector_gcroot="${vector_gcroot#vector_gcroot=}"
  prometheus_gcroot='{{prometheus_gcroot}}'
  prometheus_gcroot="${prometheus_gcroot#prometheus_gcroot=}"
  jaeger_gcroot='{{jaeger_gcroot}}'
  jaeger_gcroot="${jaeger_gcroot#jaeger_gcroot=}"
  grafana_gcroot='{{grafana_gcroot}}'
  grafana_gcroot="${grafana_gcroot#grafana_gcroot=}"
  mgmt_modules_dir='{{mgmt_modules_dir}}'
  mgmt_modules_dir="${mgmt_modules_dir#mgmt_modules_dir=}"
  remote_nix_max_jobs='{{remote_nix_max_jobs}}'
  remote_nix_max_jobs="${remote_nix_max_jobs#remote_nix_max_jobs=}"
  api_bundle="$(nix build .#api-service-bundle --no-link --print-out-paths)"
  dolt_bundle="$(nix build .#dolt-service-bundle --no-link --print-out-paths)"
  edge_bundle="$(nix build .#edge-service-bundle --no-link --print-out-paths)"
  loki_bundle="$(nix build .#loki-service-bundle --no-link --print-out-paths)"
  otel_collector_bundle="$(nix build .#otel-collector-service-bundle --no-link --print-out-paths)"
  vector_bundle="$(nix build .#vector-service-bundle --no-link --print-out-paths)"
  prometheus_bundle="$(nix build .#prometheus-service-bundle --no-link --print-out-paths)"
  jaeger_bundle="$(nix build .#jaeger-service-bundle --no-link --print-out-paths)"
  grafana_bundle="$(nix build .#grafana-service-bundle --no-link --print-out-paths)"
  deploy_dir="$(mktemp -d /tmp/fishystuff-resident-full-stack.XXXXXX)"
  trap 'rm -rf "$deploy_dir"' EXIT
  cp -a mgmt/resident-beta/. "$deploy_dir/"
  mkdir -p "$deploy_dir/files"
  mkdir -p "$deploy_dir/modules/lib"
  cp -a mgmt/modules/lib/systemd-daemon-reload "$deploy_dir/modules/lib/"
  mkdir -p "$deploy_dir/modules/github.com/purpleidea/mgmt/modules"
  cp -a "$mgmt_modules_dir/misc" "$deploy_dir/modules/github.com/purpleidea/mgmt/modules/"
  jq -n \
    --arg cluster "beta" \
    --arg hostname "$host" \
    --arg site_base_url "https://beta.fishystuff.fish" \
    --arg api_base_url "https://api.beta.fishystuff.fish" \
    --arg cdn_base_url "https://cdn.beta.fishystuff.fish" \
    --arg telemetry_base_url "https://telemetry.beta.fishystuff.fish" \
    --arg deployment_environment "beta" \
    --arg startup_mode "enabled" \
    --arg dolt_data_dir "/var/lib/fishystuff/dolt" \
    --arg dolt_cfg_dir "/var/lib/fishystuff/dolt/.doltcfg" \
    --arg dolt_database_name "fishystuff" \
    --arg dolt_remote_url "fishystuff/fishystuff" \
    --arg dolt_remote_branch "main" \
    --arg dolt_clone_depth "1" \
    --arg dolt_volume_device "" \
    --arg dolt_volume_fs_type "ext4" \
    --arg dolt_port "3306" \
    --arg site_root_dir "/srv/fishystuff/site" \
    --arg cdn_root_dir "/srv/fishystuff/cdn" \
    --arg api_bundle "$api_bundle" \
    --arg api_gcroot "$api_gcroot" \
    --arg dolt_bundle "$dolt_bundle" \
    --arg dolt_gcroot "$dolt_gcroot" \
    --arg edge_bundle "$edge_bundle" \
    --arg edge_gcroot "$edge_gcroot" \
    --arg loki_bundle "$loki_bundle" \
    --arg loki_gcroot "$loki_gcroot" \
    --arg otel_collector_bundle "$otel_collector_bundle" \
    --arg otel_collector_gcroot "$otel_collector_gcroot" \
    --arg vector_bundle "$vector_bundle" \
    --arg vector_gcroot "$vector_gcroot" \
    --arg prometheus_bundle "$prometheus_bundle" \
    --arg prometheus_gcroot "$prometheus_gcroot" \
    --arg jaeger_bundle "$jaeger_bundle" \
    --arg jaeger_gcroot "$jaeger_gcroot" \
    --arg grafana_bundle "$grafana_bundle" \
    --arg grafana_gcroot "$grafana_gcroot" \
    '{
      cluster: $cluster,
      hostname: $hostname,
      public_urls: {
        site_base_url: $site_base_url,
        api_base_url: $api_base_url,
        cdn_base_url: $cdn_base_url,
        telemetry_base_url: $telemetry_base_url
      },
      deployment_environment: $deployment_environment,
      startup_mode: $startup_mode,
      dolt: {
        data_dir: $dolt_data_dir,
        cfg_dir: $dolt_cfg_dir,
        database_name: $dolt_database_name,
        remote_url: $dolt_remote_url,
        remote_branch: $dolt_remote_branch,
        clone_depth: $dolt_clone_depth,
        volume_device: $dolt_volume_device,
        volume_fs_type: $dolt_volume_fs_type,
        port: $dolt_port
      },
      content_roots: {
        site_root_dir: $site_root_dir,
        cdn_root_dir: $cdn_root_dir
      },
      services: {
        api: {bundle_path: $api_bundle, gcroot_path: $api_gcroot},
        dolt: {bundle_path: $dolt_bundle, gcroot_path: $dolt_gcroot},
        edge: {bundle_path: $edge_bundle, gcroot_path: $edge_gcroot},
        loki: {bundle_path: $loki_bundle, gcroot_path: $loki_gcroot},
        otel_collector: {bundle_path: $otel_collector_bundle, gcroot_path: $otel_collector_gcroot},
        vector: {bundle_path: $vector_bundle, gcroot_path: $vector_gcroot},
        prometheus: {bundle_path: $prometheus_bundle, gcroot_path: $prometheus_gcroot},
        jaeger: {bundle_path: $jaeger_bundle, gcroot_path: $jaeger_gcroot},
        grafana: {bundle_path: $grafana_bundle, gcroot_path: $grafana_gcroot}
      }
    }' > "$deploy_dir/files/resident-manifest.json"
  secretspec run --profile beta-deploy -- \
    bash -lc '
      set -euo pipefail
      ssh_target="${1:?}"
      deploy_dir="${2:?}"
      deploy_timeout="${3:?}"
      remote_mgmt_bin="${4:-/usr/local/bin/mgmt}"
      if [[ "$remote_mgmt_bin" != /* ]]; then
        remote_mgmt_bin=/usr/local/bin/mgmt
      fi
      remote_nix_max_jobs="${5:?}"
      shift 5
      tmp_key="$(mktemp /tmp/fishystuff-mgmt-ssh.XXXXXX)"
      trap '\''rm -f "$tmp_key"'\'' EXIT
      umask 077
      printf "%s\n" "$HETZNER_SSH_PRIVATE_KEY" > "$tmp_key"
      chmod 600 "$tmp_key"
      remote_nix_daemon_path="$(ssh -i "$tmp_key" -o IdentitiesOnly=yes -o StrictHostKeyChecking=accept-new "$ssh_target" '\''if test -x /nix/var/nix/profiles/default/bin/nix-daemon; then printf "%s" /nix/var/nix/profiles/default/bin/nix-daemon; elif command -v nix-daemon >/dev/null 2>&1; then command -v nix-daemon; fi'\'')"
      if [[ -z "$remote_nix_daemon_path" ]]; then
        echo "could not detect remote nix-daemon path on $ssh_target" >&2
        exit 1
      fi
      SSH_OPTS="-i $tmp_key -o IdentitiesOnly=yes -o StrictHostKeyChecking=accept-new" \
      NIX_SSH_KEY_PATH="$tmp_key" \
      NIX_REMOTE_PROGRAM_PATH="$remote_nix_daemon_path" \
      FISHYSTUFF_REMOTE_NIX_MAX_JOBS="$remote_nix_max_jobs" \
      bash mgmt/scripts/push-fishystuff-bundles-remote.sh \
          "$ssh_target" \
          "$@"
      SSH_OPTS="-i $tmp_key -o IdentitiesOnly=yes" \
      bash mgmt/scripts/deploy-fishystuff-resident-remote.sh \
          "$deploy_dir" \
          "$ssh_target" \
          "$deploy_timeout" \
          "$remote_mgmt_bin"
    ' \
    -- "$target" "$deploy_dir" "$timeout" "$remote_mgmt_bin" "$remote_nix_max_jobs" "$api_bundle" "$dolt_bundle" "$edge_bundle" "$loki_bundle" "$otel_collector_bundle" "$vector_bundle" "$prometheus_bundle" "$jaeger_bundle" "$grafana_bundle"

# Build a temporary resident graph that installs a bundle-backed systemd unit
# from a local Nix bundle root, validate it, and deploy it to a resident mgmt
# instance over SSH.
mgmt-resident-dolt-bundle-probe target="mgmt-root" timeout="120" bundle_path="" gcroot_path="/nix/var/nix/gcroots/mgmt/fishystuff/dolt-current" remote_mgmt_bin="/usr/local/bin/mgmt" mgmt_bin="" mgmt_flake="/home/carp/code/playground/mgmt-missing-features" mgmt_package="minimal" mgmt_modules_dir="/home/carp/code/mgmt/modules":
  #!/usr/bin/env bash
  set -euo pipefail
  target='{{target}}'
  target="${target#target=}"
  bundle_path='{{bundle_path}}'
  bundle_path="${bundle_path#bundle_path=}"
  gcroot_path='{{gcroot_path}}'
  gcroot_path="${gcroot_path#gcroot_path=}"
  timeout='{{timeout}}'
  timeout="${timeout#timeout=}"
  remote_mgmt_bin='{{remote_mgmt_bin}}'
  remote_mgmt_bin="${remote_mgmt_bin#remote_mgmt_bin=}"
  mgmt_bin='{{mgmt_bin}}'
  mgmt_bin="${mgmt_bin#mgmt_bin=}"
  mgmt_flake='{{mgmt_flake}}'
  mgmt_flake="${mgmt_flake#mgmt_flake=}"
  mgmt_package='{{mgmt_package}}'
  mgmt_package="${mgmt_package#mgmt_package=}"
  mgmt_modules_dir='{{mgmt_modules_dir}}'
  mgmt_modules_dir="${mgmt_modules_dir#mgmt_modules_dir=}"
  if [[ -z "$bundle_path" ]]; then
    bundle_path="$(nix build .#dolt-service-bundle --no-link --print-out-paths)"
  fi
  if [[ -z "$mgmt_bin" ]]; then
    mgmt_installable="$mgmt_flake"
    if [[ -n "$mgmt_package" && "$mgmt_package" != "default" ]]; then
      mgmt_installable="$mgmt_flake#$mgmt_package"
    fi
    mgmt_store="$(nix build "$mgmt_installable" --no-link --print-out-paths)"
    mgmt_bin="$mgmt_store/bin/mgmt"
  fi
  probe_dir="$(mktemp -d /tmp/fishystuff-resident-bundle-probe.XXXXXX)"
  trap 'rm -rf "$probe_dir"' EXIT
  mkdir -p "$probe_dir/modules/lib" "$probe_dir/modules/github.com/purpleidea/mgmt/modules"
  cp -a mgmt/resident-beta/modules/lib/fishystuff-systemd "$probe_dir/modules/lib/"
  cp -a mgmt/resident-beta/modules/lib/fishystuff-bundle-nix "$probe_dir/modules/lib/"
  cp -a mgmt/resident-beta/modules/lib/fishystuff-bundle-systemd "$probe_dir/modules/lib/"
  cp -a mgmt/modules/lib/systemd-daemon-reload "$probe_dir/modules/lib/"
  cp -a "$mgmt_modules_dir/misc" "$probe_dir/modules/github.com/purpleidea/mgmt/modules/"
  printf '%s\n' \
    'import "modules/lib/fishystuff-bundle-systemd/" as fishystuff_bundle_systemd' \
    '' \
    'include fishystuff_bundle_systemd.unit(struct {' \
    "	bundle_path => \"${bundle_path}\"," \
    "	gcroot_path => \"${gcroot_path}\"," \
    '	startup_mode => "enabled",' \
    '})' \
    > "$probe_dir/main.mcl"
  printf 'main: main.mcl\n' > "$probe_dir/metadata.yaml"
  "$mgmt_bin" run lang --only-unify "$probe_dir/main.mcl"
  secretspec run --profile beta-deploy -- \
    env \
    FS_SSH_TARGET="$target" \
    FS_BUNDLE_PATH="$bundle_path" \
    bash -lc '
      set -euo pipefail
      ssh_target="${FS_SSH_TARGET:?}"
      bundle_path="${FS_BUNDLE_PATH:?}"
      tmp_key="$(mktemp /tmp/fishystuff-mgmt-ssh.XXXXXX)"
      trap '\''rm -f "$tmp_key"'\'' EXIT
      umask 077
      printf "%s\n" "$HETZNER_SSH_PRIVATE_KEY" > "$tmp_key"
      chmod 600 "$tmp_key"
      remote_nix_daemon_path="$(ssh -i "$tmp_key" -o IdentitiesOnly=yes -o StrictHostKeyChecking=accept-new "$ssh_target" '\''if test -x /nix/var/nix/profiles/default/bin/nix-daemon; then printf "%s" /nix/var/nix/profiles/default/bin/nix-daemon; elif command -v nix-daemon >/dev/null 2>&1; then command -v nix-daemon; fi'\'')"
      if [[ -z "$remote_nix_daemon_path" ]]; then
        echo "could not detect remote nix-daemon path on $ssh_target" >&2
        exit 1
      fi
      SSH_OPTS="-i $tmp_key -o IdentitiesOnly=yes -o StrictHostKeyChecking=accept-new" \
      NIX_SSH_KEY_PATH="$tmp_key" \
      NIX_REMOTE_PROGRAM_PATH="$remote_nix_daemon_path" \
      bash mgmt/scripts/push-fishystuff-bundles-remote.sh \
          "$ssh_target" \
          "$bundle_path"
    '
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
      remote_nix_daemon_path="$(ssh -i "$tmp_key" -o IdentitiesOnly=yes -o StrictHostKeyChecking=accept-new "$2" '\''if test -x /nix/var/nix/profiles/default/bin/nix-daemon; then printf "%s" /nix/var/nix/profiles/default/bin/nix-daemon; elif command -v nix-daemon >/dev/null 2>&1; then command -v nix-daemon; fi'\'')"
      if [[ -z "$remote_nix_daemon_path" ]]; then
        echo "could not detect remote nix-daemon path on $2" >&2
        exit 1
      fi
      SSH_OPTS="-i $tmp_key -o IdentitiesOnly=yes -o StrictHostKeyChecking=accept-new" \
      NIX_SSH_KEY_PATH="$tmp_key" \
      NIX_REMOTE_PROGRAM_PATH="$remote_nix_daemon_path" \
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
