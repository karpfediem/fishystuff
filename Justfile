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
  bash scripts/recipes/gitops-beta-current-handoff-plan-test.sh
  bash scripts/recipes/gitops-beta-observe-admission-test.sh
  bash scripts/recipes/gitops-beta-admission-packet-test.sh
  bash scripts/recipes/gitops-beta-activation-draft-packet-test.sh
  bash scripts/recipes/gitops-beta-first-service-set-plan-test.sh
  bash scripts/recipes/gitops-beta-first-service-set-packet-test.sh
  bash scripts/recipes/gitops-beta-activation-draft-test.sh
  bash scripts/recipes/gitops-beta-operator-proof-packet-test.sh
  bash scripts/recipes/gitops-beta-host-handoff-plan-test.sh
  bash scripts/recipes/gitops-beta-verify-activation-served-test.sh
  bash scripts/recipes/gitops-beta-operator-proof-test.sh
  bash scripts/recipes/gitops-beta-served-proof-packet-test.sh
  bash scripts/recipes/gitops-beta-served-proof-test.sh
  bash scripts/recipes/gitops-beta-proof-index-test.sh
  bash scripts/recipes/gitops-beta-apply-activation-draft-test.sh
  bash scripts/recipes/gitops-beta-edge-install-packet-test.sh
  bash scripts/recipes/gitops-beta-install-edge-test.sh
  bash scripts/recipes/gitops-beta-install-service-test.sh
  bash scripts/recipes/gitops-beta-runtime-env-test.sh
  bash scripts/recipes/gitops-beta-runtime-env-packet-test.sh
  bash scripts/recipes/gitops-beta-runtime-env-host-preflight-test.sh
  bash scripts/recipes/gitops-beta-service-start-plan-test.sh
  bash scripts/recipes/gitops-beta-service-start-packet-test.sh
  bash scripts/recipes/gitops-beta-host-bootstrap-plan-test.sh
  bash scripts/recipes/gitops-beta-host-bootstrap-apply-test.sh
  bash scripts/recipes/gitops-beta-start-services-test.sh
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

# Apply a checked beta activation draft through local mgmt only after explicit beta opt-ins and reviewed beta operator proof hash.
gitops-beta-apply-activation-draft draft_file="data/gitops/beta-activation.draft.desired.json" summary_file="data/gitops/beta-current.handoff-summary.json" admission_file="" mgmt_bin="auto" deploy_bin="auto" converged_timeout="45" proof_file="" proof_max_age_seconds="86400":
  bash scripts/recipes/gitops-beta-apply-activation-draft.sh "{{draft_file}}" "{{summary_file}}" "{{admission_file}}" "{{mgmt_bin}}" "{{deploy_bin}}" "{{converged_timeout}}" "{{proof_file}}" "{{proof_max_age_seconds}}"

# Verify local served GitOps state still matches the checked production activation draft.
gitops-verify-activation-served draft_file="data/gitops/production-activation.draft.desired.json" summary_file="data/gitops/production-current.handoff-summary.json" admission_file="" deploy_bin="auto" state_dir="/var/lib/fishystuff/gitops" run_dir="/run/fishystuff/gitops":
  bash scripts/recipes/gitops-verify-activation-served.sh "{{draft_file}}" "{{summary_file}}" "{{admission_file}}" "{{deploy_bin}}" "{{state_dir}}" "{{run_dir}}"

# Verify local served beta GitOps state still matches the checked beta activation draft.
gitops-beta-verify-activation-served draft_file="data/gitops/beta-activation.draft.desired.json" summary_file="data/gitops/beta-current.handoff-summary.json" admission_file="" deploy_bin="auto" state_dir="/var/lib/fishystuff/gitops-beta" run_dir="/run/fishystuff/gitops-beta":
  bash scripts/recipes/gitops-beta-verify-activation-served.sh "{{draft_file}}" "{{summary_file}}" "{{admission_file}}" "{{deploy_bin}}" "{{state_dir}}" "{{run_dir}}"

# Write a timestamped local beta GitOps operator proof artifact. No remote mutation.
gitops-beta-operator-proof output_dir="data/gitops" draft_file="data/gitops/beta-activation.draft.desired.json" summary_file="data/gitops/beta-current.handoff-summary.json" admission_file="" edge_bundle="auto" deploy_bin="auto" run_helper_tests="true" served_state_dir="" rollback_set_path="" state_dir="/var/lib/fishystuff/gitops-beta" run_dir="/run/fishystuff/gitops-beta" systemd_unit_path="/etc/systemd/system/fishystuff-beta-edge.service" tls_fullchain_path="/var/lib/fishystuff/gitops-beta/tls/live/fullchain.pem" tls_privkey_path="/var/lib/fishystuff/gitops-beta/tls/live/privkey.pem":
  bash scripts/recipes/gitops-beta-operator-proof.sh "{{output_dir}}" "{{draft_file}}" "{{summary_file}}" "{{admission_file}}" "{{edge_bundle}}" "{{deploy_bin}}" "{{run_helper_tests}}" "{{served_state_dir}}" "{{rollback_set_path}}" "{{state_dir}}" "{{run_dir}}" "{{systemd_unit_path}}" "{{tls_fullchain_path}}" "{{tls_privkey_path}}"

# Print beta operator-proof readiness and next command. No mutation.
gitops-beta-operator-proof-packet proof_file="" proof_dir="data/gitops" max_age_seconds="86400" draft_file="data/gitops/beta-activation.draft.desired.json" summary_file="data/gitops/beta-current.handoff-summary.json" admission_file="data/gitops/beta-admission.evidence.json" edge_bundle="auto" deploy_bin="auto" api_upstream="http://127.0.0.1:18192" observation_dir="data/gitops/beta-admission-observations":
  @bash scripts/recipes/gitops-beta-operator-proof-packet.sh "{{proof_file}}" "{{proof_dir}}" "{{max_age_seconds}}" "{{draft_file}}" "{{summary_file}}" "{{admission_file}}" "{{edge_bundle}}" "{{deploy_bin}}" "{{api_upstream}}" "{{observation_dir}}"

# Check a stored beta GitOps operator proof artifact is fresh and still matches current inputs.
gitops-check-beta-operator-proof proof_file="" max_age_seconds="86400" proof_dir="data/gitops":
  bash scripts/recipes/gitops-check-beta-operator-proof.sh "{{proof_file}}" "{{max_age_seconds}}" "{{proof_dir}}"

# Write a timestamped proof that beta served state matches the checked beta activation and operator proof.
gitops-beta-served-proof output_dir="data/gitops" draft_file="data/gitops/beta-activation.draft.desired.json" summary_file="data/gitops/beta-current.handoff-summary.json" admission_file="" proof_file="" deploy_bin="auto" state_dir="/var/lib/fishystuff/gitops-beta" run_dir="/run/fishystuff/gitops-beta" proof_max_age_seconds="86400":
  bash scripts/recipes/gitops-beta-served-proof.sh "{{output_dir}}" "{{draft_file}}" "{{summary_file}}" "{{admission_file}}" "{{proof_file}}" "{{deploy_bin}}" "{{state_dir}}" "{{run_dir}}" "{{proof_max_age_seconds}}"

# Print beta served-proof readiness and next command. No mutation.
gitops-beta-served-proof-packet proof_dir="data/gitops" max_age_seconds="86400" draft_file="data/gitops/beta-activation.draft.desired.json" summary_file="data/gitops/beta-current.handoff-summary.json" admission_file="data/gitops/beta-admission.evidence.json" proof_file="" deploy_bin="auto" state_dir="/var/lib/fishystuff/gitops-beta" run_dir="/run/fishystuff/gitops-beta" edge_bundle="auto" api_upstream="http://127.0.0.1:18192" observation_dir="data/gitops/beta-admission-observations":
  @bash scripts/recipes/gitops-beta-served-proof-packet.sh "{{proof_dir}}" "{{max_age_seconds}}" "{{draft_file}}" "{{summary_file}}" "{{admission_file}}" "{{proof_file}}" "{{deploy_bin}}" "{{state_dir}}" "{{run_dir}}" "{{edge_bundle}}" "{{api_upstream}}" "{{observation_dir}}"

# Print the latest local beta GitOps proof chain. No remote mutation.
gitops-beta-proof-index proof_dir="data/gitops" max_age_seconds="86400" require_complete="false":
  bash scripts/recipes/gitops-beta-proof-index.sh "{{proof_dir}}" "{{max_age_seconds}}" "{{require_complete}}"

# Print beta edge install readiness and reviewed hashes. No mutation.
gitops-beta-edge-install-packet edge_bundle="auto" proof_dir="data/gitops" max_age_seconds="86400" draft_file="data/gitops/beta-activation.draft.desired.json" summary_file="data/gitops/beta-current.handoff-summary.json" admission_file="data/gitops/beta-admission.evidence.json" proof_file="" deploy_bin="auto" state_dir="/var/lib/fishystuff/gitops-beta" run_dir="/run/fishystuff/gitops-beta" api_upstream="http://127.0.0.1:18192" observation_dir="data/gitops/beta-admission-observations":
  @bash scripts/recipes/gitops-beta-edge-install-packet.sh "{{edge_bundle}}" "{{proof_dir}}" "{{max_age_seconds}}" "{{draft_file}}" "{{summary_file}}" "{{admission_file}}" "{{proof_file}}" "{{deploy_bin}}" "{{state_dir}}" "{{run_dir}}" "{{api_upstream}}" "{{observation_dir}}"

# Install/restart the beta edge unit only after explicit opt-ins and checked beta proof hashes.
gitops-beta-install-edge edge_bundle="auto" proof_dir="data/gitops" max_age_seconds="86400" install_bin="install" systemctl_bin="systemctl":
  bash scripts/recipes/gitops-beta-install-edge.sh "{{edge_bundle}}" "{{proof_dir}}" "{{max_age_seconds}}" "{{install_bin}}" "{{systemctl_bin}}"

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

# Observe beta loopback admission probes and write checked activation evidence. No remote mutation.
gitops-beta-observe-admission output="data/gitops/beta-admission.evidence.json" summary_file="data/gitops/beta-current.handoff-summary.json" api_upstream="http://127.0.0.1:18192" observation_dir="data/gitops/beta-admission-observations" curl_bin="curl" expected_cdn_base_url="https://cdn.beta.fishystuff.fish/":
  bash scripts/recipes/gitops-beta-observe-admission.sh "{{output}}" "{{summary_file}}" "{{api_upstream}}" "{{observation_dir}}" "{{curl_bin}}" "{{expected_cdn_base_url}}"

# Print beta admission evidence readiness and next command. No mutation.
gitops-beta-admission-packet admission_file="data/gitops/beta-admission.evidence.json" summary_file="data/gitops/beta-current.handoff-summary.json" api_upstream="http://127.0.0.1:18192" observation_dir="data/gitops/beta-admission-observations" draft_file="data/gitops/beta-activation.draft.desired.json":
  @bash scripts/recipes/gitops-beta-admission-packet.sh "{{admission_file}}" "{{summary_file}}" "{{api_upstream}}" "{{observation_dir}}" "{{draft_file}}"

# Print beta activation-draft readiness and next command. No mutation.
gitops-beta-activation-draft-packet draft_file="data/gitops/beta-activation.draft.desired.json" summary_file="data/gitops/beta-current.handoff-summary.json" admission_file="data/gitops/beta-admission.evidence.json" proof_dir="data/gitops" edge_bundle="auto" deploy_bin="auto" api_upstream="http://127.0.0.1:18192" observation_dir="data/gitops/beta-admission-observations":
  @bash scripts/recipes/gitops-beta-activation-draft-packet.sh "{{draft_file}}" "{{summary_file}}" "{{admission_file}}" "{{proof_dir}}" "{{edge_bundle}}" "{{deploy_bin}}" "{{api_upstream}}" "{{observation_dir}}"

# Print read-only readiness for generating beta current desired state and handoff summary.
gitops-beta-current-handoff-plan output="data/gitops/beta-current.desired.json" dolt_ref="beta" mgmt_bin="auto" deploy_bin="auto" summary_output="":
  bash scripts/recipes/gitops-beta-current-handoff-plan.sh "{{output}}" "{{dolt_ref}}" "{{mgmt_bin}}" "{{deploy_bin}}" "{{summary_output}}"

# Print the read-only first beta service-set runbook and current artifact readiness.
gitops-beta-first-service-set-plan summary_file="data/gitops/beta-current.handoff-summary.json" admission_file="data/gitops/beta-admission.evidence.json" draft_file="data/gitops/beta-activation.draft.desired.json" proof_dir="data/gitops" api_bundle="auto" dolt_bundle="auto" edge_bundle="auto" api_env_file="/var/lib/fishystuff/gitops-beta/api/runtime.env" dolt_env_file="/var/lib/fishystuff/gitops-beta/dolt/beta.env" api_upstream="http://127.0.0.1:18192" observation_dir="data/gitops/beta-admission-observations":
  bash scripts/recipes/gitops-beta-first-service-set-plan.sh "{{summary_file}}" "{{admission_file}}" "{{draft_file}}" "{{proof_dir}}" "{{api_bundle}}" "{{dolt_bundle}}" "{{edge_bundle}}" "{{api_env_file}}" "{{dolt_env_file}}" "{{api_upstream}}" "{{observation_dir}}"

# Print only the compact current operator packet from the first beta service-set plan. No mutation.
gitops-beta-first-service-set-packet summary_file="data/gitops/beta-current.handoff-summary.json" admission_file="data/gitops/beta-admission.evidence.json" draft_file="data/gitops/beta-activation.draft.desired.json" proof_dir="data/gitops" api_bundle="auto" dolt_bundle="auto" edge_bundle="auto" api_env_file="/var/lib/fishystuff/gitops-beta/api/runtime.env" dolt_env_file="/var/lib/fishystuff/gitops-beta/dolt/beta.env" api_upstream="http://127.0.0.1:18192" observation_dir="data/gitops/beta-admission-observations":
  @bash scripts/recipes/gitops-beta-first-service-set-packet.sh "{{summary_file}}" "{{admission_file}}" "{{draft_file}}" "{{proof_dir}}" "{{api_bundle}}" "{{dolt_bundle}}" "{{edge_bundle}}" "{{api_env_file}}" "{{dolt_env_file}}" "{{api_upstream}}" "{{observation_dir}}"

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

# Run fast local regression checks for beta admission observation.
gitops-beta-observe-admission-test:
  bash scripts/recipes/gitops-beta-observe-admission-test.sh

# Run fast local regression checks for beta admission packet readiness.
gitops-beta-admission-packet-test:
  bash scripts/recipes/gitops-beta-admission-packet-test.sh

# Run fast local regression checks for beta activation-draft packet readiness.
gitops-beta-activation-draft-packet-test:
  bash scripts/recipes/gitops-beta-activation-draft-packet-test.sh

# Run fast local regression checks for beta current handoff input planning.
gitops-beta-current-handoff-plan-test:
  bash scripts/recipes/gitops-beta-current-handoff-plan-test.sh

# Run fast local regression checks for the first beta service-set plan.
gitops-beta-first-service-set-plan-test:
  bash scripts/recipes/gitops-beta-first-service-set-plan-test.sh

# Run fast local regression checks for the first beta service-set operator packet.
gitops-beta-first-service-set-packet-test:
  bash scripts/recipes/gitops-beta-first-service-set-packet-test.sh

# Run fast local regression checks for beta activation/admission draft generation.
gitops-beta-activation-draft-test:
  bash scripts/recipes/gitops-beta-activation-draft-test.sh

# Run fast local regression checks for beta operator-proof packet readiness.
gitops-beta-operator-proof-packet-test:
  bash scripts/recipes/gitops-beta-operator-proof-packet-test.sh

# Run fast local regression checks for beta served-proof packet readiness.
gitops-beta-served-proof-packet-test:
  bash scripts/recipes/gitops-beta-served-proof-packet-test.sh

# Run fast local regression checks for beta edge-install packet readiness.
gitops-beta-edge-install-packet-test:
  bash scripts/recipes/gitops-beta-edge-install-packet-test.sh

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

# Validate a beta GitOps API or Dolt service bundle. No remote mutation.
gitops-beta-check-service-bundle service="api" bundle="auto":
  bash scripts/recipes/gitops-check-beta-service-bundle.sh "{{service}}" "{{bundle}}"

# Print the local beta deploy credential readiness packet. No remote mutation.
gitops-beta-deploy-credentials-packet:
  @bash scripts/recipes/gitops-beta-deploy-credentials-packet.sh

# Print the reviewed beta resident host provision shape. No hcloud, SSH, or DNS mutation.
gitops-beta-host-provision-plan host_name="site-nbg1-beta" server_type="cx33" image="debian-13" location="nbg1" datacenter="nbg1-dc3":
  @bash scripts/recipes/gitops-beta-host-provision-plan.sh "{{host_name}}" "{{server_type}}" "{{image}}" "{{location}}" "{{datacenter}}"

# Run fast local regression checks for the beta host provision packet.
gitops-beta-host-provision-plan-test:
  bash scripts/recipes/gitops-beta-host-provision-plan-test.sh

# Bind an operator-confirmed beta host IPv4 to follow-up packet commands. No SSH or DNS mutation.
gitops-beta-host-selection-packet public_ipv4="" host_name="site-nbg1-beta" ssh_user="root":
  @bash scripts/recipes/gitops-beta-host-selection-packet.sh "{{public_ipv4}}" "{{host_name}}" "{{ssh_user}}"

# Run fast local regression checks for the beta host selection packet.
gitops-beta-host-selection-packet-test:
  bash scripts/recipes/gitops-beta-host-selection-packet-test.sh

# Probe a fresh beta host over SSH through beta-deploy credentials. Read-only.
gitops-beta-remote-host-preflight target="" expected_hostname="site-nbg1-beta":
  @bash scripts/recipes/gitops-beta-remote-host-preflight.sh "{{target}}" "{{expected_hostname}}"

# Bootstrap beta-local users/directories on a fresh beta host after explicit opt-ins. Remote host mutation.
gitops-beta-remote-host-bootstrap target="" expected_hostname="site-nbg1-beta":
  bash scripts/recipes/gitops-beta-remote-host-bootstrap.sh "{{target}}" "{{expected_hostname}}"

# Install multi-user Nix on a fresh beta host after explicit opt-ins. Remote host mutation.
gitops-beta-remote-install-nix target="" expected_hostname="site-nbg1-beta":
  bash scripts/recipes/gitops-beta-remote-install-nix.sh "{{target}}" "{{expected_hostname}}"

# Copy the exact checked beta handoff closures to the fresh beta host. Remote store mutation.
gitops-beta-copy-handoff-closures target="" summary_file="data/gitops/beta-current.handoff-summary.json" push_bin="scripts/recipes/push-closure.sh":
  bash scripts/recipes/gitops-beta-copy-handoff-closures.sh "{{target}}" "{{summary_file}}" "{{push_bin}}"

# Copy checked beta API/Dolt runtime env files to the fresh beta host. Remote host mutation.
gitops-beta-copy-runtime-env target="" api_source="" dolt_source="" ssh_bin="ssh" scp_bin="scp" summary_file="data/gitops/beta-current.handoff-summary.json":
  bash scripts/recipes/gitops-beta-copy-runtime-env.sh "{{target}}" "{{api_source}}" "{{dolt_source}}" "{{ssh_bin}}" "{{scp_bin}}" "{{summary_file}}"

# Fetch beta Dolt data and pin the checked GitOps release ref on the fresh beta host. Remote host mutation.
gitops-beta-remote-materialize-dolt-ref target="" expected_hostname="site-nbg1-beta" summary_file="data/gitops/beta-current.handoff-summary.json" ssh_bin="ssh":
  bash scripts/recipes/gitops-beta-remote-materialize-dolt-ref.sh "{{target}}" "{{expected_hostname}}" "{{summary_file}}" "{{ssh_bin}}"

# Install and start the checked beta Dolt/API service units on the fresh beta host. Remote host mutation.
gitops-beta-remote-start-services target="" expected_hostname="site-nbg1-beta" summary_file="data/gitops/beta-current.handoff-summary.json" ssh_bin="ssh":
  bash scripts/recipes/gitops-beta-remote-start-services.sh "{{target}}" "{{expected_hostname}}" "{{summary_file}}" "{{ssh_bin}}"

# Copy and start the checked beta edge service on the beta host with explicit placeholder or existing TLS mode. Remote host mutation.
gitops-beta-remote-start-edge target="" expected_hostname="site-nbg1-beta" edge_bundle="auto" summary_file="data/gitops/beta-current.handoff-summary.json" push_bin="scripts/recipes/push-closure.sh" ssh_bin="ssh" scp_bin="scp":
  bash scripts/recipes/gitops-beta-remote-start-edge.sh "{{target}}" "{{expected_hostname}}" "{{edge_bundle}}" "{{summary_file}}" "{{push_bin}}" "{{ssh_bin}}" "{{scp_bin}}"

# Install and start the reviewed beta TLS resident mgmt unit on the beta host. Remote host mutation.
gitops-beta-remote-install-tls-resident target="" expected_hostname="site-nbg1-beta" desired_state="data/gitops/beta-tls.staging.desired.json" unit_file="data/gitops/fishystuff-beta-tls-reconciler.service" cloudflare_token_source="env:CLOUDFLARE_API_TOKEN" ssh_bin="ssh" scp_bin="scp":
  bash scripts/recipes/gitops-beta-remote-install-tls-resident.sh "{{target}}" "{{expected_hostname}}" "{{desired_state}}" "{{unit_file}}" "{{cloudflare_token_source}}" "{{ssh_bin}}" "{{scp_bin}}"

# Install operator-supplied beta edge TLS material on the fresh beta host. Remote host mutation.
gitops-beta-remote-install-edge-tls target="" expected_hostname="site-nbg1-beta" fullchain="" privkey="" ssh_bin="ssh" scp_bin="scp":
  bash scripts/recipes/gitops-beta-remote-install-edge-tls.sh "{{target}}" "{{expected_hostname}}" "{{fullchain}}" "{{privkey}}" "{{ssh_bin}}" "{{scp_bin}}"

# Generate beta TLS desired state for mgmt ACME reconciliation. Local file write only.
gitops-beta-tls-desired output="data/gitops/beta-tls.staging.desired.json" ca="staging" contact_email="":
  bash scripts/recipes/gitops-beta-tls-desired.sh "{{output}}" "{{ca}}" "{{contact_email}}"

# Print the read-only beta TLS mgmt ACME reconciliation packet.
gitops-beta-tls-reconcile-packet state_file="data/gitops/beta-tls.staging.desired.json" ca="staging" contact_email="":
  @bash scripts/recipes/gitops-beta-tls-reconcile-packet.sh "{{state_file}}" "{{ca}}" "{{contact_email}}"

# Reconcile beta TLS through the clean mgmt ACME graph after explicit opt-ins. Local beta host mutation.
gitops-beta-reconcile-tls state_file="data/gitops/beta-tls.staging.desired.json" ca="staging" mgmt_bin="auto" converged_timeout="300":
  bash scripts/recipes/gitops-beta-reconcile-tls.sh "{{state_file}}" "{{ca}}" "{{mgmt_bin}}" "{{converged_timeout}}"

# Generate the beta TLS resident mgmt systemd unit. Local file write only.
gitops-beta-tls-resident-unit output="data/gitops/fishystuff-beta-tls-reconciler.service" state_file="/var/lib/fishystuff/gitops-beta/desired/beta-tls.staging.desired.json" mgmt_bin="auto" gitops_dir="auto" cloudflare_token_credential="/var/lib/fishystuff/gitops-beta/secrets/cloudflare-api-token" converged_timeout="-1":
  bash scripts/recipes/gitops-beta-tls-resident-unit.sh "{{output}}" "{{state_file}}" "{{mgmt_bin}}" "{{gitops_dir}}" "{{cloudflare_token_credential}}" "{{converged_timeout}}"

# Print the read-only beta TLS resident install packet.
gitops-beta-tls-resident-install-packet desired_state="data/gitops/beta-tls.staging.desired.json" unit_file="data/gitops/fishystuff-beta-tls-reconciler.service" cloudflare_token_source="env:CLOUDFLARE_API_TOKEN":
  @bash scripts/recipes/gitops-beta-tls-resident-install-packet.sh "{{desired_state}}" "{{unit_file}}" "{{cloudflare_token_source}}"

# Print the read-only beta TLS resident status packet. No mutation.
gitops-beta-tls-resident-status-packet desired_state="/var/lib/fishystuff/gitops-beta/desired/beta-tls.staging.desired.json" unit_file="/etc/systemd/system/fishystuff-beta-tls-reconciler.service" cloudflare_token="/var/lib/fishystuff/gitops-beta/secrets/cloudflare-api-token" tls_fullchain="/var/lib/fishystuff/gitops-beta/tls/live/fullchain.pem" tls_privkey="/var/lib/fishystuff/gitops-beta/tls/live/privkey.pem" systemctl_bin="systemctl" openssl_bin="openssl":
  @bash scripts/recipes/gitops-beta-tls-resident-status-packet.sh "{{desired_state}}" "{{unit_file}}" "{{cloudflare_token}}" "{{tls_fullchain}}" "{{tls_privkey}}" "{{systemctl_bin}}" "{{openssl_bin}}"

# Install and start the reviewed beta TLS resident mgmt unit. Local beta host mutation.
gitops-beta-install-tls-resident desired_state="data/gitops/beta-tls.staging.desired.json" unit_file="data/gitops/fishystuff-beta-tls-reconciler.service" cloudflare_token_source="env:CLOUDFLARE_API_TOKEN" install_bin="install" systemctl_bin="systemctl":
  bash scripts/recipes/gitops-beta-install-tls-resident.sh "{{desired_state}}" "{{unit_file}}" "{{cloudflare_token_source}}" "{{install_bin}}" "{{systemctl_bin}}"

# Run fast local regression checks for the beta remote host preflight/bootstrap helpers.
gitops-beta-remote-host-test:
  bash scripts/recipes/gitops-beta-remote-host-test.sh

# Run fast local regression checks for the beta remote Nix installer helper.
gitops-beta-remote-install-nix-test:
  bash scripts/recipes/gitops-beta-remote-install-nix-test.sh

# Run fast local regression checks for beta handoff closure copying.
gitops-beta-copy-handoff-closures-test:
  bash scripts/recipes/gitops-beta-copy-handoff-closures-test.sh

# Run fast local regression checks for beta remote runtime env copying.
gitops-beta-copy-runtime-env-test:
  bash scripts/recipes/gitops-beta-copy-runtime-env-test.sh

# Run fast local regression checks for remote beta Dolt release ref materialization.
gitops-beta-remote-materialize-dolt-ref-test:
  bash scripts/recipes/gitops-beta-remote-materialize-dolt-ref-test.sh

# Run fast local regression checks for remote beta service starting.
gitops-beta-remote-start-services-test:
  bash scripts/recipes/gitops-beta-remote-start-services-test.sh

# Run fast local regression checks for remote beta edge starting.
gitops-beta-remote-start-edge-test:
  bash scripts/recipes/gitops-beta-remote-start-edge-test.sh

# Run fast local regression checks for remote beta TLS resident installation.
gitops-beta-remote-install-tls-resident-test:
  bash scripts/recipes/gitops-beta-remote-install-tls-resident-test.sh

# Run fast local regression checks for remote beta edge TLS installation.
gitops-beta-remote-install-edge-tls-test:
  bash scripts/recipes/gitops-beta-remote-install-edge-tls-test.sh

# Run fast local regression checks for beta TLS mgmt ACME packet generation.
gitops-beta-tls-reconcile-packet-test:
  bash scripts/recipes/gitops-beta-tls-reconcile-packet-test.sh

# Run fast local regression checks for guarded beta TLS mgmt reconciliation.
gitops-beta-reconcile-tls-test:
  bash scripts/recipes/gitops-beta-reconcile-tls-test.sh

# Run fast local regression checks for the beta TLS resident unit generator.
gitops-beta-tls-resident-unit-test:
  bash scripts/recipes/gitops-beta-tls-resident-unit-test.sh

# Run fast local regression checks for the beta TLS resident install packet.
gitops-beta-tls-resident-install-packet-test:
  bash scripts/recipes/gitops-beta-tls-resident-install-packet-test.sh

# Run fast local regression checks for the beta TLS resident status packet.
gitops-beta-tls-resident-status-packet-test:
  bash scripts/recipes/gitops-beta-tls-resident-status-packet-test.sh

# Run fast local regression checks for the beta TLS resident install gate.
gitops-beta-install-tls-resident-test:
  bash scripts/recipes/gitops-beta-install-tls-resident-test.sh

# Read Hetzner beta server inventory through beta-deploy credentials. Read-only.
gitops-beta-hetzner-inventory-packet old_server_name="site-nbg1-beta" replacement_server_name="site-nbg1-beta-v2":
  @bash scripts/recipes/gitops-beta-hetzner-inventory-packet.sh "{{old_server_name}}" "{{replacement_server_name}}"

# Move beta-only Cloudflare A records to the selected fresh beta IPv4. Cloudflare DNS mutation.
gitops-beta-cloudflare-dns-cutover target_ipv4="" zone_name="fishystuff.fish" curl_bin="curl":
  bash scripts/recipes/gitops-beta-cloudflare-dns-cutover.sh "{{target_ipv4}}" "{{zone_name}}" "{{curl_bin}}"

# Run fast local regression checks for the beta-only Cloudflare DNS cutover helper.
gitops-beta-cloudflare-dns-cutover-test:
  bash scripts/recipes/gitops-beta-cloudflare-dns-cutover-test.sh

# Plan the fresh beta host replacement sequence. No hcloud, SSH, or DNS mutation.
gitops-beta-host-replacement-plan old_server_name="site-nbg1-beta" replacement_server_name="site-nbg1-beta-v2" proof_dir="data/gitops":
  @bash scripts/recipes/gitops-beta-host-replacement-plan.sh "{{old_server_name}}" "{{replacement_server_name}}" "{{proof_dir}}"

# Run fast local regression checks for the beta host replacement plan.
gitops-beta-host-replacement-plan-test:
  bash scripts/recipes/gitops-beta-host-replacement-plan-test.sh

# Create the fresh beta Hetzner resident host after explicit opt-ins. Infrastructure mutation.
gitops-beta-hetzner-create-host server_name="site-nbg1-beta-v2" server_type="cx33" image="debian-13" datacenter="nbg1-dc3":
  bash scripts/recipes/gitops-beta-hetzner-create-host.sh "{{server_name}}" "{{server_type}}" "{{image}}" "{{datacenter}}"

# Run fast local regression checks for the guarded beta Hetzner create helper.
gitops-beta-hetzner-create-host-test:
  bash scripts/recipes/gitops-beta-hetzner-create-host-test.sh

# Retire the old beta Hetzner host after explicit opt-ins. Infrastructure mutation.
gitops-beta-hetzner-retire-host retire_server_name="site-nbg1-beta" retire_server_id="" retire_server_ipv4="" active_server_name="site-nbg1-beta-v2" active_server_ipv4="":
  bash scripts/recipes/gitops-beta-hetzner-retire-host.sh "{{retire_server_name}}" "{{retire_server_id}}" "{{retire_server_ipv4}}" "{{active_server_name}}" "{{active_server_ipv4}}"

# Run fast local regression checks for the guarded beta Hetzner retire helper.
gitops-beta-hetzner-retire-host-test:
  bash scripts/recipes/gitops-beta-hetzner-retire-host-test.sh

# Generate and store a missing beta deploy SSH key after explicit opt-in. No upload or remote mutation.
gitops-beta-deploy-key-ensure key_comment="fishystuff-beta-deploy" key_name="fishystuff-beta-deploy":
  bash scripts/recipes/gitops-beta-deploy-key-ensure.sh "{{key_comment}}" "{{key_name}}"

# Run fast local regression checks for beta deploy credential helpers.
gitops-beta-deploy-credentials-test:
  bash scripts/recipes/gitops-beta-deploy-credentials-test.sh

# Run local Nix checks for the distinct beta GitOps API, Dolt, and edge service bundles.
gitops-beta-service-bundles-test system="x86_64-linux":
  nix build --no-link ".#checks.{{system}}.api-service-bundle-beta-gitops-handoff" ".#checks.{{system}}.dolt-service-bundle-beta-gitops-handoff" ".#checks.{{system}}.edge-service-bundle-beta-gitops-handoff"

# Install/restart a beta API or Dolt unit only after explicit opt-ins and checked unit hash.
gitops-beta-install-service service="api" bundle="auto" install_bin="install" systemctl_bin="systemctl":
  bash scripts/recipes/gitops-beta-install-service.sh "{{service}}" "{{bundle}}" "{{install_bin}}" "{{systemctl_bin}}"

# Write the beta API or Dolt runtime env file after explicit service-specific opt-in.
gitops-beta-write-runtime-env service="api" output="":
  bash scripts/recipes/gitops-beta-write-runtime-env.sh "{{service}}" "{{output}}"

# Write the beta runtime env through a narrow SecretSpec profile after explicit opt-in.
gitops-beta-write-runtime-env-secretspec service="api" output="" profile="beta-runtime":
  bash scripts/recipes/gitops-beta-write-runtime-env-secretspec.sh "{{service}}" "{{output}}" "{{profile}}"

# Validate the beta API or Dolt runtime env file. No mutation.
gitops-beta-check-runtime-env service="api" env_file="":
  bash scripts/recipes/gitops-check-beta-runtime-env.sh "{{service}}" "{{env_file}}"

# Print the beta runtime env readiness packet. No mutation.
gitops-beta-runtime-env-packet api_env_file="/var/lib/fishystuff/gitops-beta/api/runtime.env" dolt_env_file="/var/lib/fishystuff/gitops-beta/dolt/beta.env" api_bundle="auto" dolt_bundle="auto" summary_file="data/gitops/beta-current.handoff-summary.json":
  @bash scripts/recipes/gitops-beta-runtime-env-packet.sh "{{api_env_file}}" "{{dolt_env_file}}" "{{api_bundle}}" "{{dolt_bundle}}" "{{summary_file}}"

# Print the beta runtime env host-context preflight. No mutation.
gitops-beta-runtime-env-host-preflight api_env_file="/var/lib/fishystuff/gitops-beta/api/runtime.env" dolt_env_file="/var/lib/fishystuff/gitops-beta/dolt/beta.env":
  @bash scripts/recipes/gitops-beta-runtime-env-host-preflight.sh "{{api_env_file}}" "{{dolt_env_file}}"

# Run fast local regression checks for beta runtime env writers/checkers.
gitops-beta-runtime-env-test:
  bash scripts/recipes/gitops-beta-runtime-env-test.sh

# Run fast local regression checks for the beta runtime env readiness packet.
gitops-beta-runtime-env-packet-test:
  bash scripts/recipes/gitops-beta-runtime-env-packet-test.sh

# Run fast local regression checks for the beta runtime env host-context preflight.
gitops-beta-runtime-env-host-preflight-test:
  bash scripts/recipes/gitops-beta-runtime-env-host-preflight-test.sh

# Print the beta API/Dolt service start plan from checked env files and service bundles. No mutation.
gitops-beta-service-start-plan api_bundle="auto" dolt_bundle="auto" api_env_file="/var/lib/fishystuff/gitops-beta/api/runtime.env" dolt_env_file="/var/lib/fishystuff/gitops-beta/dolt/beta.env" summary_file="data/gitops/beta-current.handoff-summary.json":
  bash scripts/recipes/gitops-beta-service-start-plan.sh "{{api_bundle}}" "{{dolt_bundle}}" "{{api_env_file}}" "{{dolt_env_file}}" "{{summary_file}}"

# Print the compact beta API/Dolt service start packet from checked env files and service bundles. No mutation.
gitops-beta-service-start-packet api_bundle="auto" dolt_bundle="auto" api_env_file="/var/lib/fishystuff/gitops-beta/api/runtime.env" dolt_env_file="/var/lib/fishystuff/gitops-beta/dolt/beta.env" summary_file="data/gitops/beta-current.handoff-summary.json":
  @bash scripts/recipes/gitops-beta-service-start-packet.sh "{{api_bundle}}" "{{dolt_bundle}}" "{{api_env_file}}" "{{dolt_env_file}}" "{{summary_file}}"

# Install/restart beta Dolt, then beta API, after explicit opt-ins and checked unit hashes.
gitops-beta-start-services api_bundle="auto" dolt_bundle="auto" api_env_file="/var/lib/fishystuff/gitops-beta/api/runtime.env" dolt_env_file="/var/lib/fishystuff/gitops-beta/dolt/beta.env" install_bin="install" systemctl_bin="systemctl" summary_file="data/gitops/beta-current.handoff-summary.json":
  bash scripts/recipes/gitops-beta-start-services.sh "{{api_bundle}}" "{{dolt_bundle}}" "{{api_env_file}}" "{{dolt_env_file}}" "{{install_bin}}" "{{systemctl_bin}}" "{{summary_file}}"

# Run fast local regression checks for the beta service start plan.
gitops-beta-service-start-plan-test:
  bash scripts/recipes/gitops-beta-service-start-plan-test.sh

# Run fast local regression checks for the beta service start packet.
gitops-beta-service-start-packet-test:
  bash scripts/recipes/gitops-beta-service-start-packet-test.sh

# Print the fresh beta host bootstrap/readiness plan. No mutation.
gitops-beta-host-bootstrap-plan api_runtime_env_path="/var/lib/fishystuff/gitops-beta/api/runtime.env" api_release_env_path="/var/lib/fishystuff/gitops-beta/api/beta.env" dolt_runtime_env_path="/var/lib/fishystuff/gitops-beta/dolt/beta.env":
  bash scripts/recipes/gitops-beta-host-bootstrap-plan.sh "{{api_runtime_env_path}}" "{{api_release_env_path}}" "{{dolt_runtime_env_path}}"

# Run fast local regression checks for the beta host bootstrap/readiness plan.
gitops-beta-host-bootstrap-plan-test:
  bash scripts/recipes/gitops-beta-host-bootstrap-plan-test.sh

# Apply the fresh beta host bootstrap contract locally after explicit opt-ins. No remote mutation.
gitops-beta-host-bootstrap-apply install_bin="install" groupadd_bin="groupadd" useradd_bin="useradd" getent_bin="getent":
  bash scripts/recipes/gitops-beta-host-bootstrap-apply.sh "{{install_bin}}" "{{groupadd_bin}}" "{{useradd_bin}}" "{{getent_bin}}"

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

# Run fast local regression checks for beta GitOps operator proofs.
gitops-beta-operator-proof-test:
  bash scripts/recipes/gitops-beta-operator-proof-test.sh

# Run fast local regression checks for beta GitOps served proofs.
gitops-beta-served-proof-test:
  bash scripts/recipes/gitops-beta-served-proof-test.sh

# Run fast local regression checks for beta GitOps proof indexing.
gitops-beta-proof-index-test:
  bash scripts/recipes/gitops-beta-proof-index-test.sh

# Run fast local regression checks for the beta GitOps apply gate.
gitops-beta-apply-activation-draft-test:
  bash scripts/recipes/gitops-beta-apply-activation-draft-test.sh

# Run fast local regression checks for the beta edge install/restart gate.
gitops-beta-install-edge-test:
  bash scripts/recipes/gitops-beta-install-edge-test.sh

# Run fast local regression checks for beta API/Dolt service install gates.
gitops-beta-install-service-test:
  bash scripts/recipes/gitops-beta-install-service-test.sh

# Run fast local regression checks for the guarded beta API/Dolt start sequence.
gitops-beta-start-services-test:
  bash scripts/recipes/gitops-beta-start-services-test.sh

# Run fast local regression checks for the guarded beta host bootstrap apply gate.
gitops-beta-host-bootstrap-apply-test:
  bash scripts/recipes/gitops-beta-host-bootstrap-apply-test.sh

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
