# Start the full local dev server stack
[default]
up:
  devenv up --no-tui

# Start the local dev stack plus rebuild/restart watchers
watch:
  devenv up --profile watch --no-tui

# Open a deployment service URL or tunnel a private service UI first.
open deployment *services:
  bash scripts/recipes/open.sh "{{deployment}}" {{services}}

# Report deployment service state, rooted store paths, and public/open URLs.
status deployment *services:
  bash scripts/recipes/status.sh "{{deployment}}" {{services}}

# Smoke-test the selected deployment through its public URLs.
smoke deployment:
  bash scripts/recipes/smoke.sh "{{deployment}}"

# Smoke-test a remote deployment against an explicit origin IP without DNS cutover.
origin-smoke deployment origin_ipv4="":
  bash scripts/recipes/origin-smoke.sh "{{deployment}}" "{{origin_ipv4}}"

# Initialize a clone of our dolt database on http://dolthub.com/repositories/fishystuff/fishystuff
clone-db:
    dolt clone fishystuff/fishystuff .

# Starts a local MySQL server using Dolt
serve-db:
    dolt sql-server

# Replaces the current Fishing_Table with the one obtained from a (new) Fishing_Table.xlsx file in the current directory
update_fishing_table:
    bash scripts/recipes/update-fishing-table.sh

# Run the Discord bot with the SecretSpec bot profile
bot-run:
  secretspec run --profile bot -- cargo run --manifest-path bot/Cargo.toml

# Stage CDN-served runtime assets under data/cdn/public
cdn-stage:
  ./tools/scripts/stage_cdn_assets.sh

# Rebuild source-backed CDN calculator icons and pet textures only
cdn-stage-icons:
  node tools/scripts/build_item_icons_from_source.mjs --output-dir data/cdn/public/images/items
  node tools/scripts/build_pet_icons_from_source.mjs --output-dir data/cdn/public/images/pets

# Compute the exact CDN filenames required by the current deployment inputs.
cdn-required-files out="data/cdn/required-files.json":
  ./tools/scripts/compute_required_cdn_filenames.sh --out "{{out}}"

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

# Copy one or more local closures to a remote host.
push-closure host *closures:
  bash scripts/recipes/push-closure.sh "{{host}}" {{closures}}

# Deploy the selected services for a named deployment.
# The no-service default reuses active CDN content; pass cdn explicitly to update it.
# API without Dolt is refused unless explicitly acknowledged with a reason.
deploy deployment *services:
  bash scripts/recipes/deploy.sh "{{deployment}}" {{services}}

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
