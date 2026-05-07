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

# Type-check the clean-slate local GitOps mgmt graph against a desired-state fixture.
gitops-unify mgmt_bin="auto" state_file="gitops/fixtures/empty.desired.json":
  bash scripts/recipes/gitops-unify.sh "{{mgmt_bin}}" "{{state_file}}"

# Run fast host-local GitOps deployment helper tests. No NixOS VM.
gitops-helper-test:
  cargo test -p fishystuff_deploy
  bash scripts/recipes/gitops-production-current-handoff-test.sh

# Validate local GitOps served status, active selection, rollback set, and rollback readiness documents.
gitops-check-served deploy_bin="auto" environment="local-test" state_dir="/var/lib/fishystuff/gitops" host="" release_id="":
  bash scripts/recipes/gitops-check-served.sh "{{deploy_bin}}" "{{environment}}" "{{state_dir}}" "{{host}}" "{{release_id}}"

# Print a local GitOps served release and rollback target summary.
gitops-served-summary deploy_bin="auto" environment="local-test" state_dir="/var/lib/fishystuff/gitops" host="" release_id="":
  bash scripts/recipes/gitops-check-served.sh "{{deploy_bin}}" "{{environment}}" "{{state_dir}}" "{{host}}" "{{release_id}}" "summary-served"

# Inspect local GitOps served state plus admission, route, and root-readiness documents.
gitops-inspect-served deploy_bin="auto" environment="local-test" state_dir="/var/lib/fishystuff/gitops" run_dir="/run/fishystuff/gitops" host="" release_id="":
  bash scripts/recipes/gitops-inspect-served.sh "{{deploy_bin}}" "{{environment}}" "{{state_dir}}" "{{run_dir}}" "{{host}}" "{{release_id}}"

# Check that a desired-state JSON environment has a serving-capable active/retained release tuple.
gitops-check-desired-serving deploy_bin="auto" state_file="data/gitops/production-current.desired.json" environment="production":
  bash scripts/recipes/gitops-check-desired-serving.sh "{{deploy_bin}}" "{{state_file}}" "{{environment}}"

# Derive retained-release JSON from a local GitOps rollback-set index and its member documents.
gitops-retained-releases-json deploy_bin="auto" environment="production" state_dir="/var/lib/fishystuff/gitops" rollback_set_path="":
  bash scripts/recipes/gitops-retained-releases-json.sh "{{deploy_bin}}" "{{environment}}" "{{state_dir}}" "{{rollback_set_path}}"

# Generate a local validate-mode production desired-state snapshot from exact local outputs.
gitops-production-current-desired output="data/gitops/production-current.desired.json" dolt_ref="main":
  bash scripts/recipes/gitops-production-current-desired.sh "{{output}}" "{{dolt_ref}}"

# Generate and validate a production-current handoff snapshot with retained rollback input.
gitops-production-current-handoff output="data/gitops/production-current.desired.json" dolt_ref="main" mgmt_bin="auto" deploy_bin="auto" summary_output="":
  bash scripts/recipes/gitops-production-current-handoff.sh "{{output}}" "{{dolt_ref}}" "{{mgmt_bin}}" "{{deploy_bin}}" "{{summary_output}}"

# Verify a production-current handoff summary still matches its exact desired-state file and CDN retention manifest.
gitops-check-handoff-summary summary_file="data/gitops/production-current.handoff-summary.json" state_file="":
  bash scripts/recipes/gitops-check-handoff-summary.sh "{{summary_file}}" "{{state_file}}"

# Derive retained releases from served GitOps state, then generate and validate production-current handoff artifacts.
gitops-production-current-from-served output="data/gitops/production-current.desired.json" state_dir="/var/lib/fishystuff/gitops" environment="production" retained_output="" dolt_ref="main" mgmt_bin="auto" deploy_bin="auto" summary_output="":
  bash scripts/recipes/gitops-production-current-from-served.sh "{{output}}" "{{state_dir}}" "{{environment}}" "{{retained_output}}" "{{dolt_ref}}" "{{mgmt_bin}}" "{{deploy_bin}}" "{{summary_output}}"

# Run fast local regression checks for the production-current handoff recipe.
gitops-production-current-handoff-test:
  bash scripts/recipes/gitops-production-current-handoff-test.sh

# Run a local-only GitOps flake check or NixOS VM test.
gitops-vm-test test_name="single-host-candidate":
  bash scripts/recipes/gitops-vm-test.sh "{{test_name}}"

# Copy one or more local closures to a remote host.
push-closure host *closures:
  bash scripts/recipes/push-closure.sh "{{host}}" {{closures}}

# Deploy the selected services for a named deployment.
# The no-service default reuses active CDN content; pass cdn explicitly to update it.
# API without Dolt is refused unless explicitly acknowledged with a reason.
deploy deployment *services:
  bash scripts/recipes/deploy.sh "{{deployment}}" {{services}}

# Validate local deploy target boundaries without contacting remote hosts.
deploy-safety-check deployment:
  bash scripts/recipes/deploy-safety-check.sh "{{deployment}}"

# Report the local deploy authority boundary without contacting remote hosts.
deploy-authority-check deployment *services:
  bash scripts/recipes/deploy-authority-check.sh "{{deployment}}" {{services}}

# Run local deploy safety guard regression tests.
deploy-safety-test:
  bash scripts/recipes/deploy-safety-test.sh

# Verify deploy keys are accepted only by their own environment hosts.
deploy-key-boundary-check beta_target="root@beta.fishystuff.fish" production_target="root@fishystuff.fish" beta_telemetry_target="root@telemetry.beta.fishystuff.fish":
  bash scripts/recipes/deploy-key-boundary-check.sh "{{beta_target}}" "{{production_target}}" "{{beta_telemetry_target}}"

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
