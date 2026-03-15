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

# Push the staged CDN tree to Bunny Storage via FTP
# Override BUNNY_FTP_PARALLEL / BUNNY_FTP_CONNECTION_LIMIT in the shell if needed.
cdn-push:
  secretspec run --profile cdn -- ./tools/scripts/push_bunnycdn.sh

# Refresh the staged tree and then push it to Bunny Storage
cdn-sync:
  just cdn-stage
  just cdn-push

# Validate that the local SecretSpec provider has the required values for a profile
secrets-check profile="api":
  p='{{profile}}'; p="${p#profile=}"; secretspec check --profile "$p"

# Start the full local dev stack through devenv process orchestration
dev-up:
  devenv up
