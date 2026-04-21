# Start the full local dev server stack
[default]
up:
  devenv up --no-tui

# Start the local dev stack plus rebuild/restart watchers
watch:
  devenv up --profile watch --no-tui

# Open a local service UI in the default browser, or tunnel beta Grafana first.
open target ssh_target="root@beta.fishystuff.fish" local_port="3300":
  bash scripts/recipes/open.sh "{{target}}" "{{ssh_target}}" "{{local_port}}"

# Initialize a clone of our dolt database on http://dolthub.com/repositories/fishystuff/fishystuff
clone-db:
    dolt clone fishystuff/fishystuff .

# Starts a local MySQL server using Dolt
serve-db:
    dolt sql-server

# Replaces the current Fishing_Table with the one obtained from a (new) Fishing_Table.xlsx file in the current directory
update_fishing_table:
    bash scripts/recipes/update-fishing-table.sh


# Build and deploy the discord bot
deploy-bot:
  bash scripts/recipes/deploy-bot.sh

# Build and deploy the Axum API
deploy-api:
  bash scripts/recipes/deploy-api.sh

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
  bash scripts/recipes/mgmt-beta-unify.sh "{{mgmt_bin}}"

# Run the local mgmt Hetzner beta topology bootstrap as a one-shot converging apply.
# Default state is absent for safety; use state=running to request server creation.
mgmt-beta-bootstrap state="absent" converged_timeout="30" mgmt_bin="../result/bin/mgmt" client_urls="http://127.0.0.1:3379" server_urls="http://127.0.0.1:3380" prometheus="false" prometheus_listen="127.0.0.1:9233" pprof_path="":
  bash scripts/recipes/mgmt-beta-bootstrap.sh "{{state}}" "{{converged_timeout}}" "{{mgmt_bin}}" "{{client_urls}}" "{{server_urls}}" "{{prometheus}}" "{{prometheus_listen}}" "{{pprof_path}}"

# Type-check the resident bootstrap graph used to install a host-local mgmt service.
mgmt-resident-bootstrap-unify mgmt_bin="../result/bin/mgmt":
  bash scripts/recipes/mgmt-resident-bootstrap-unify.sh "{{mgmt_bin}}"

# Copy a locally built mgmt closure to a remote host and install the resident service there.
mgmt-resident-kickstart-remote target="" host="" timeout="120" mgmt_flake="/home/carp/code/playground/mgmt-missing-features" mgmt_package="minimal":
  bash scripts/recipes/mgmt-resident-kickstart-remote.sh "{{target}}" "{{host}}" "{{timeout}}" "{{mgmt_flake}}" "{{mgmt_package}}"

# Push a self-contained graph directory into the resident mgmt instance on a remote host.
mgmt-resident-deploy-remote target="" dir="mgmt/resident-deploy-probe" timeout="120" remote_mgmt_bin="/usr/local/bin/mgmt":
  bash scripts/recipes/mgmt-resident-deploy-remote.sh "{{target}}" "{{dir}}" "{{timeout}}" "{{remote_mgmt_bin}}"

# Build the API and Dolt service bundles locally, push both closures to a
# remote host, and deploy the resident beta graph for the current API/DB host
# shape. The resident graph owns GC-root selection via nix:gcroot.
mgmt-resident-push-api-db *args:
  bash scripts/recipes/mgmt-resident-push-api-db.sh "{{args}}"

# Build the current pure service bundles for the single-host beta stack, push
# them to a remote host, and deploy the resident graph with API, Dolt, edge,
# and observability daemons. The resident graph owns GC-root selection via
# nix:gcroot. Set `services_csv=` to a comma-separated subset when you only
# want to rebuild and push specific optional services.
mgmt-resident-push-full-stack *args:
  bash scripts/recipes/mgmt-resident-push-full-stack.sh "{{args}}"

# Build a temporary resident graph that installs a bundle-backed systemd unit
# from a local Nix bundle root, validate it, and deploy it to a resident mgmt
# instance over SSH.
mgmt-resident-dolt-bundle-probe target="" timeout="120" bundle_path="" gcroot_path="/nix/var/nix/gcroots/mgmt/fishystuff/dolt-current" remote_mgmt_bin="/usr/local/bin/mgmt" mgmt_bin="" mgmt_flake="/home/carp/code/playground/mgmt-missing-features" mgmt_package="minimal" mgmt_modules_dir="/home/carp/code/mgmt/modules":
  bash scripts/recipes/mgmt-resident-dolt-bundle-probe.sh "{{target}}" "{{timeout}}" "{{bundle_path}}" "{{gcroot_path}}" "{{remote_mgmt_bin}}" "{{mgmt_bin}}" "{{mgmt_flake}}" "{{mgmt_package}}" "{{mgmt_modules_dir}}"

# Build the Dolt service bundle, copy it to a remote host, root it, install the
# rendered unit, and verify that the SQL server answers a local health check.
mgmt-dolt-target-smoke target="" gcroot="/nix/var/nix/gcroots/mgmt/fishystuff/dolt-current" sql_host="127.0.0.1" sql_port="3306" query_timeout="20":
  bash scripts/recipes/mgmt-dolt-target-smoke.sh "{{target}}" "{{gcroot}}" "{{sql_host}}" "{{sql_port}}" "{{query_timeout}}"

# Build the current map runtime and map-serving CDN payload once
build-map:
  ./tools/scripts/build_map.sh
  ./tools/scripts/stage_cdn_assets.sh --map-only

# Build the current site output once
build-site:
  cd site && just build-release

# Build the current local dev outputs once
build:
  bash scripts/recipes/build.sh
