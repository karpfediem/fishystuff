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
  bash scripts/recipes/gitops-beta-current-desired-test.sh
  bash scripts/recipes/gitops-beta-current-handoff-test.sh
  bash scripts/recipes/gitops-beta-activation-draft-test.sh
  bash scripts/recipes/gitops-beta-host-handoff-plan-test.sh
  bash scripts/recipes/gitops-beta-verify-activation-served-test.sh
  bash scripts/recipes/gitops-production-edge-handoff-bundle-test.sh
  bash scripts/recipes/gitops-beta-edge-handoff-bundle-test.sh
  bash scripts/recipes/gitops-production-host-handoff-plan-test.sh
  bash scripts/recipes/gitops-production-host-inventory-test.sh
  bash scripts/recipes/gitops-production-operator-proof-test.sh
  bash scripts/recipes/gitops-check-production-operator-proof-test.sh
  bash scripts/recipes/gitops-production-served-proof-test.sh
  bash scripts/recipes/gitops-production-proof-index-test.sh
  bash scripts/recipes/gitops-production-preflight-test.sh

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

# Generate a local validate-mode beta desired-state snapshot from exact local outputs.
gitops-beta-current-desired output="data/gitops/beta-current.desired.json" dolt_ref="beta":
  bash scripts/recipes/gitops-beta-current-desired.sh "{{output}}" "{{dolt_ref}}"

# Generate and type-check the local validate-mode beta desired-state snapshot.
gitops-beta-current-validate output="data/gitops/beta-current.desired.json" dolt_ref="beta" mgmt_bin="auto":
  bash scripts/recipes/gitops-beta-current-desired.sh "{{output}}" "{{dolt_ref}}"
  bash scripts/recipes/gitops-unify.sh "{{mgmt_bin}}" "{{output}}"

# Generate and validate a local beta-current handoff snapshot. No remote mutation.
gitops-beta-current-handoff output="data/gitops/beta-current.desired.json" dolt_ref="beta" mgmt_bin="auto" deploy_bin="auto" summary_output="":
  bash scripts/recipes/gitops-beta-current-handoff.sh "{{output}}" "{{dolt_ref}}" "{{mgmt_bin}}" "{{deploy_bin}}" "{{summary_output}}"

# Generate and validate a production-current handoff snapshot with retained rollback input.
gitops-production-current-handoff output="data/gitops/production-current.desired.json" dolt_ref="main" mgmt_bin="auto" deploy_bin="auto" summary_output="":
  bash scripts/recipes/gitops-production-current-handoff.sh "{{output}}" "{{dolt_ref}}" "{{mgmt_bin}}" "{{deploy_bin}}" "{{summary_output}}"

# Verify a production-current handoff summary still matches its exact desired-state file and CDN retention manifest.
gitops-check-handoff-summary summary_file="data/gitops/production-current.handoff-summary.json" state_file="":
  bash scripts/recipes/gitops-check-handoff-summary.sh "{{summary_file}}" "{{state_file}}"

# Generate a checked local-apply production activation draft from a verified handoff and explicit admission evidence.
gitops-production-activation-draft output="data/gitops/production-activation.draft.desired.json" summary_file="data/gitops/production-current.handoff-summary.json" admission_file="" mgmt_bin="auto" deploy_bin="auto":
  bash scripts/recipes/gitops-production-activation-draft.sh "{{output}}" "{{summary_file}}" "{{admission_file}}" "{{mgmt_bin}}" "{{deploy_bin}}"

# Generate a checked local-apply beta activation draft from a verified beta handoff and explicit admission evidence.
gitops-beta-activation-draft output="data/gitops/beta-activation.draft.desired.json" summary_file="data/gitops/beta-current.handoff-summary.json" admission_file="" mgmt_bin="auto" deploy_bin="auto":
  bash scripts/recipes/gitops-beta-activation-draft.sh "{{output}}" "{{summary_file}}" "{{admission_file}}" "{{mgmt_bin}}" "{{deploy_bin}}"

# Verify a production activation draft still matches the verified handoff and admission evidence.
gitops-check-activation-draft draft_file="data/gitops/production-activation.draft.desired.json" summary_file="data/gitops/production-current.handoff-summary.json" admission_file="" deploy_bin="auto":
  bash scripts/recipes/gitops-check-activation-draft.sh "{{draft_file}}" "{{summary_file}}" "{{admission_file}}" "{{deploy_bin}}"

# Print the exact checked production activation tuple before any later apply path consumes it.
gitops-review-activation-draft draft_file="data/gitops/production-activation.draft.desired.json" summary_file="data/gitops/production-current.handoff-summary.json" admission_file="" deploy_bin="auto":
  bash scripts/recipes/gitops-review-activation-draft.sh "{{draft_file}}" "{{summary_file}}" "{{admission_file}}" "{{deploy_bin}}"

# Apply a checked production activation draft through local mgmt only after explicit opt-ins and reviewed operator proof hash.
gitops-apply-activation-draft draft_file="data/gitops/production-activation.draft.desired.json" summary_file="data/gitops/production-current.handoff-summary.json" admission_file="" mgmt_bin="auto" deploy_bin="auto" converged_timeout="45" proof_file="" proof_max_age_seconds="86400":
  bash scripts/recipes/gitops-apply-activation-draft.sh "{{draft_file}}" "{{summary_file}}" "{{admission_file}}" "{{mgmt_bin}}" "{{deploy_bin}}" "{{converged_timeout}}" "{{proof_file}}" "{{proof_max_age_seconds}}"

# Verify local served GitOps state still matches the checked production activation draft.
gitops-verify-activation-served draft_file="data/gitops/production-activation.draft.desired.json" summary_file="data/gitops/production-current.handoff-summary.json" admission_file="" deploy_bin="auto" state_dir="/var/lib/fishystuff/gitops" run_dir="/run/fishystuff/gitops":
  bash scripts/recipes/gitops-verify-activation-served.sh "{{draft_file}}" "{{summary_file}}" "{{admission_file}}" "{{deploy_bin}}" "{{state_dir}}" "{{run_dir}}"

# Verify local served beta GitOps state still matches the checked beta activation draft.
gitops-beta-verify-activation-served draft_file="data/gitops/beta-activation.draft.desired.json" summary_file="data/gitops/beta-current.handoff-summary.json" admission_file="" deploy_bin="auto" state_dir="/var/lib/fishystuff/gitops-beta" run_dir="/run/fishystuff/gitops-beta":
  bash scripts/recipes/gitops-beta-verify-activation-served.sh "{{draft_file}}" "{{summary_file}}" "{{admission_file}}" "{{deploy_bin}}" "{{state_dir}}" "{{run_dir}}"

# Write a timestamped proof that served state matches the checked activation and operator proof.
gitops-production-served-proof output_dir="data/gitops" draft_file="data/gitops/production-activation.draft.desired.json" summary_file="data/gitops/production-current.handoff-summary.json" admission_file="" proof_file="" deploy_bin="auto" state_dir="/var/lib/fishystuff/gitops" run_dir="/run/fishystuff/gitops" proof_max_age_seconds="86400":
  bash scripts/recipes/gitops-production-served-proof.sh "{{output_dir}}" "{{draft_file}}" "{{summary_file}}" "{{admission_file}}" "{{proof_file}}" "{{deploy_bin}}" "{{state_dir}}" "{{run_dir}}" "{{proof_max_age_seconds}}"

# Print the latest local production GitOps proof chain. No remote mutation.
gitops-production-proof-index proof_dir="data/gitops" max_age_seconds="86400" require_complete="false":
  bash scripts/recipes/gitops-production-proof-index.sh "{{proof_dir}}" "{{max_age_seconds}}" "{{require_complete}}"

# Write activation admission evidence from observed API meta, DB-backed, and site/CDN probe outputs.
gitops-write-activation-admission-evidence output="data/gitops/production-admission.evidence.json" summary_file="data/gitops/production-current.handoff-summary.json" api_upstream="" api_meta_source="" db_probe_file="" site_cdn_probe_file="":
  bash scripts/recipes/gitops-write-activation-admission-evidence.sh "{{output}}" "{{summary_file}}" "{{api_upstream}}" "{{api_meta_source}}" "{{db_probe_file}}" "{{site_cdn_probe_file}}"

# Write beta activation admission evidence from observed API meta, DB-backed, and site/CDN probe outputs.
gitops-beta-write-activation-admission-evidence output="data/gitops/beta-admission.evidence.json" summary_file="data/gitops/beta-current.handoff-summary.json" api_upstream="" api_meta_source="" db_probe_file="" site_cdn_probe_file="":
  bash scripts/recipes/gitops-beta-write-activation-admission-evidence.sh "{{output}}" "{{summary_file}}" "{{api_upstream}}" "{{api_meta_source}}" "{{db_probe_file}}" "{{site_cdn_probe_file}}"

# Derive retained releases from served GitOps state, then generate and validate production-current handoff artifacts.
gitops-production-current-from-served output="data/gitops/production-current.desired.json" state_dir="/var/lib/fishystuff/gitops" environment="production" retained_output="" dolt_ref="main" mgmt_bin="auto" deploy_bin="auto" summary_output="":
  bash scripts/recipes/gitops-production-current-from-served.sh "{{output}}" "{{state_dir}}" "{{environment}}" "{{retained_output}}" "{{dolt_ref}}" "{{mgmt_bin}}" "{{deploy_bin}}" "{{summary_output}}"

# Run fast local regression checks for the production-current handoff recipe.
gitops-production-current-handoff-test:
  bash scripts/recipes/gitops-production-current-handoff-test.sh

# Run fast local regression checks for beta current desired-state generation.
gitops-beta-current-desired-test:
  bash scripts/recipes/gitops-beta-current-desired-test.sh

# Run fast local regression checks for beta current handoff generation.
gitops-beta-current-handoff-test:
  bash scripts/recipes/gitops-beta-current-handoff-test.sh

# Run fast local regression checks for beta activation/admission draft generation.
gitops-beta-activation-draft-test:
  bash scripts/recipes/gitops-beta-activation-draft-test.sh

# Build or validate the local production GitOps edge handoff bundle. No remote mutation.
gitops-production-edge-handoff-bundle bundle="auto":
  bash scripts/recipes/gitops-check-edge-handoff-bundle.sh "{{bundle}}" production

# Run fast local regression checks for production GitOps edge handoff bundle validation.
gitops-production-edge-handoff-bundle-test:
  bash scripts/recipes/gitops-production-edge-handoff-bundle-test.sh

# Build or validate the local beta GitOps edge handoff bundle. No remote mutation.
gitops-beta-edge-handoff-bundle bundle="auto":
  bash scripts/recipes/gitops-check-edge-handoff-bundle.sh "{{bundle}}" beta

# Run fast local regression checks for beta GitOps edge handoff bundle validation.
gitops-beta-edge-handoff-bundle-test:
  bash scripts/recipes/gitops-beta-edge-handoff-bundle-test.sh

# Build the distinct beta GitOps API and Dolt service bundles. No remote mutation.
gitops-beta-service-bundles:
  nix build --no-link ".#api-service-bundle-beta-gitops-handoff" ".#dolt-service-bundle-beta-gitops-handoff"

# Run local Nix checks for the distinct beta GitOps API, Dolt, and edge service bundles.
gitops-beta-service-bundles-test system="x86_64-linux":
  nix build --no-link ".#checks.{{system}}.api-service-bundle-beta-gitops-handoff" ".#checks.{{system}}.dolt-service-bundle-beta-gitops-handoff" ".#checks.{{system}}.edge-service-bundle-beta-gitops-handoff"

# Print the dry-run host-local production GitOps handoff plan. No remote mutation.
gitops-production-host-handoff-plan draft_file="data/gitops/production-activation.draft.desired.json" summary_file="data/gitops/production-current.handoff-summary.json" admission_file="" edge_bundle="auto" deploy_bin="auto":
  bash scripts/recipes/gitops-production-host-handoff-plan.sh "{{draft_file}}" "{{summary_file}}" "{{admission_file}}" "{{edge_bundle}}" "{{deploy_bin}}"

# Print the dry-run host-local beta GitOps handoff plan. No remote mutation.
gitops-beta-host-handoff-plan draft_file="data/gitops/beta-activation.draft.desired.json" summary_file="data/gitops/beta-current.handoff-summary.json" admission_file="" edge_bundle="auto" deploy_bin="auto":
  bash scripts/recipes/gitops-beta-host-handoff-plan.sh "{{draft_file}}" "{{summary_file}}" "{{admission_file}}" "{{edge_bundle}}" "{{deploy_bin}}"

# Print read-only local production GitOps host inventory. No remote mutation.
gitops-production-host-inventory state_dir="/var/lib/fishystuff/gitops" run_dir="/run/fishystuff/gitops" edge_bundle="auto" systemd_unit_path="/etc/systemd/system/fishystuff-edge.service" tls_fullchain_path="/run/fishystuff/edge/tls/fullchain.pem" tls_privkey_path="/run/fishystuff/edge/tls/privkey.pem" environment="production":
  bash scripts/recipes/gitops-production-host-inventory.sh "{{state_dir}}" "{{run_dir}}" "{{edge_bundle}}" "{{systemd_unit_path}}" "{{tls_fullchain_path}}" "{{tls_privkey_path}}" "{{environment}}"

# Run the local-only production GitOps preflight over exact handoff, admission, edge, and optional served rollback artifacts.
gitops-production-preflight draft_file="data/gitops/production-activation.draft.desired.json" summary_file="data/gitops/production-current.handoff-summary.json" admission_file="" edge_bundle="auto" deploy_bin="auto" run_helper_tests="true" served_state_dir="" rollback_set_path="":
  bash scripts/recipes/gitops-production-preflight.sh "{{draft_file}}" "{{summary_file}}" "{{admission_file}}" "{{edge_bundle}}" "{{deploy_bin}}" "{{run_helper_tests}}" "{{served_state_dir}}" "{{rollback_set_path}}"

# Write a timestamped local production GitOps operator proof artifact. No remote mutation.
gitops-production-operator-proof output_dir="data/gitops" draft_file="data/gitops/production-activation.draft.desired.json" summary_file="data/gitops/production-current.handoff-summary.json" admission_file="" edge_bundle="auto" deploy_bin="auto" run_helper_tests="true" served_state_dir="" rollback_set_path="" state_dir="/var/lib/fishystuff/gitops" run_dir="/run/fishystuff/gitops" systemd_unit_path="/etc/systemd/system/fishystuff-edge.service" tls_fullchain_path="/run/fishystuff/edge/tls/fullchain.pem" tls_privkey_path="/run/fishystuff/edge/tls/privkey.pem" environment="production":
  bash scripts/recipes/gitops-production-operator-proof.sh "{{output_dir}}" "{{draft_file}}" "{{summary_file}}" "{{admission_file}}" "{{edge_bundle}}" "{{deploy_bin}}" "{{run_helper_tests}}" "{{served_state_dir}}" "{{rollback_set_path}}" "{{state_dir}}" "{{run_dir}}" "{{systemd_unit_path}}" "{{tls_fullchain_path}}" "{{tls_privkey_path}}" "{{environment}}"

# Check a stored production GitOps operator proof artifact is fresh and still matches current inputs.
gitops-check-production-operator-proof proof_file="" max_age_seconds="86400" proof_dir="data/gitops":
  bash scripts/recipes/gitops-check-production-operator-proof.sh "{{proof_file}}" "{{max_age_seconds}}" "{{proof_dir}}"

# Run fast local regression checks for the dry-run production host handoff plan.
gitops-production-host-handoff-plan-test:
  bash scripts/recipes/gitops-production-host-handoff-plan-test.sh

# Run fast local regression checks for the dry-run beta host handoff plan.
gitops-beta-host-handoff-plan-test:
  bash scripts/recipes/gitops-beta-host-handoff-plan-test.sh

# Run fast local regression checks for production GitOps host inventory.
gitops-production-host-inventory-test:
  bash scripts/recipes/gitops-production-host-inventory-test.sh

# Run fast local regression checks for production GitOps operator proofs.
gitops-production-operator-proof-test:
  bash scripts/recipes/gitops-production-operator-proof-test.sh

# Run fast local regression checks for production GitOps operator proof checks.
gitops-check-production-operator-proof-test:
  bash scripts/recipes/gitops-check-production-operator-proof-test.sh

# Run fast local regression checks for production GitOps served proofs.
gitops-production-served-proof-test:
  bash scripts/recipes/gitops-production-served-proof-test.sh

# Run fast local regression checks for beta GitOps served verification.
gitops-beta-verify-activation-served-test:
  bash scripts/recipes/gitops-beta-verify-activation-served-test.sh

# Run fast local regression checks for production GitOps proof indexing.
gitops-production-proof-index-test:
  bash scripts/recipes/gitops-production-proof-index-test.sh

# Run fast local regression checks for the production GitOps preflight wrapper.
gitops-production-preflight-test:
  bash scripts/recipes/gitops-production-preflight-test.sh

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
