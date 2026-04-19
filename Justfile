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
