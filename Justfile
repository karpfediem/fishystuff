# Start the full local dev server stack
[default]
up:
  devenv up

# Start the local dev stack plus rebuild/restart watchers via the `watch` profile
up-watch:
  devenv up --profile watch

# Stop detached local dev processes started via `devenv up -d`
down:
  devenv processes down

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

# Serve the staged CDN tree locally with cache headers
cdn-serve:
  ./tools/scripts/run_cdn_server.sh

# Push the staged CDN tree to Bunny Storage via HTTP API.
# Override BUNNY_STORAGE_PARALLEL (or legacy BUNNY_FTP_PARALLEL) in the shell if needed.
cdn-push:
  secretspec run --profile cdn -- ./tools/scripts/push_bunnycdn.sh

# Refresh the staged tree and then push it to Bunny Storage
cdn-sync:
  just cdn-stage
  just cdn-push

# Build the map runtime, refresh staged map assets, and push only the CDN map root.
cdn-sync-map:
  ./tools/scripts/build_map.sh
  ./tools/scripts/stage_cdn_assets.sh
  BUNNY_SYNC_ROOTS=map secretspec run --profile cdn -- ./tools/scripts/push_bunnycdn.sh

# Validate that the local SecretSpec provider has the required values for a profile
secrets-check profile="api":
  p='{{profile}}'; p="${p#profile=}"; secretspec check --profile "$p"

# Build the current map runtime and staged CDN payload once
dev-build-map:
  ./tools/scripts/build_map.sh
  ./tools/scripts/stage_cdn_assets.sh

# Build the current site output once
dev-build-site:
  cd site && just build-release

# Build the current local dev outputs once
dev-build:
  just dev-build-map
  just dev-build-site

# Watch map/CDN/site build inputs in parallel. Use with a running `just up`.
dev-watch-builds:
  ./tools/scripts/dev_watch_builds.sh

# Watch the API in a dedicated terminal. Use with `just dev-up-no-api`.
dev-watch-api:
  watchexec -r \
    -w api \
    -w lib \
    -w Cargo.toml \
    -w Cargo.lock \
    -w secretspec.toml \
    -w tools/scripts/run_api.sh \
    --exts rs,toml \
    -- ./tools/scripts/run_api.sh

# Watch the wasm map runtime and refresh staged CDN assets
dev-watch-map:
  watchexec -r --postpone \
    -w map/fishystuff_ui_bevy \
    -w lib/fishystuff_api \
    -w lib/fishystuff_client \
    -w lib/fishystuff_core \
    -w Cargo.toml \
    -w Cargo.lock \
    -w tools/scripts/build_map.sh \
    --exts rs,toml,css \
    -- just dev-build-map

# Watch CDN-owned browser host assets and restage them
dev-watch-cdn:
  watchexec -r --postpone \
    -w site/assets/map \
    -w tools/scripts/stage_cdn_assets.sh \
    -w tools/scripts/build_item_icons_from_source.mjs \
    --exts js,mjs,css \
    -- just cdn-stage

# Watch site inputs and rebuild `.out`
dev-watch-site:
  watchexec -r --postpone \
    -w site/content \
    -w site/layouts \
    -w site/assets \
    -w site/scripts \
    -w site/tailwind.input.css \
    -w site/zine.ziggy \
    --ignore site/assets/js/datastar.js \
    --ignore site/assets/img/icons.svg \
    --ignore site/assets/img/guides/*-320.webp \
    --ignore site/assets/img/guides/*-640.webp \
    --ignore site/assets/img/favicon-16x16.png \
    --ignore site/assets/img/favicon-32x32.png \
    --ignore site/assets/img/logo-32.png \
    --ignore site/assets/img/logo-64.png \
    --ignore site/assets/css/fonts/**/*.site.woff2 \
    --ignore site/assets/css/site.css \
    -- just dev-build-site

# Start the local dev servers except the API, for use with `just dev-watch-api`
dev-up-no-api:
  devenv up db caddy
