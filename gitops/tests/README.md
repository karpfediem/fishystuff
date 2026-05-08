# GitOps Tests

Fast local checks:

```bash
just gitops-helper-test
just gitops-check-served environment=local-test state_dir=/var/lib/fishystuff/gitops
just gitops-served-summary environment=local-test state_dir=/var/lib/fishystuff/gitops
just gitops-inspect-served environment=local-test state_dir=/var/lib/fishystuff/gitops run_dir=/run/fishystuff/gitops
just gitops-check-desired-serving state_file=data/gitops/production-current.desired.json environment=production
just gitops-retained-releases-json environment=production state_dir=/var/lib/fishystuff/gitops > /tmp/fishystuff-retained-releases.json
just gitops-production-current-desired output=/tmp/fishystuff-production-current.desired.json
FISHYSTUFF_GITOPS_RETAINED_RELEASES_FILE=/tmp/fishystuff-retained-releases.json just gitops-production-current-handoff output=/tmp/fishystuff-production-current.desired.json
just gitops-check-handoff-summary summary_file=/tmp/fishystuff-production-current.handoff-summary.json state_file=/tmp/fishystuff-production-current.desired.json
just gitops-write-activation-admission-evidence output=/tmp/fishystuff-production-admission.json summary_file=/tmp/fishystuff-production-current.handoff-summary.json api_upstream=https://api.fishystuff.fish api_meta_source=/tmp/fishystuff-api-meta.json db_probe_file=/tmp/fishystuff-db-probe.json site_cdn_probe_file=/tmp/fishystuff-site-cdn-probe.json
just gitops-production-activation-draft output=/tmp/fishystuff-production-activation.draft.desired.json summary_file=/tmp/fishystuff-production-current.handoff-summary.json admission_file=/tmp/fishystuff-production-admission.json
just gitops-check-activation-draft draft_file=/tmp/fishystuff-production-activation.draft.desired.json summary_file=/tmp/fishystuff-production-current.handoff-summary.json admission_file=/tmp/fishystuff-production-admission.json
just gitops-review-activation-draft draft_file=/tmp/fishystuff-production-activation.draft.desired.json summary_file=/tmp/fishystuff-production-current.handoff-summary.json admission_file=/tmp/fishystuff-production-admission.json
FISHYSTUFF_GITOPS_ENABLE_PRODUCTION_APPLY=1 FISHYSTUFF_GITOPS_ENABLE_LOCAL_APPLY=1 FISHYSTUFF_GITOPS_APPLY_DRAFT_SHA256=<reviewed draft hash> just gitops-apply-activation-draft draft_file=/tmp/fishystuff-production-activation.draft.desired.json summary_file=/tmp/fishystuff-production-current.handoff-summary.json admission_file=/tmp/fishystuff-production-admission.json
just gitops-verify-activation-served draft_file=/tmp/fishystuff-production-activation.draft.desired.json summary_file=/tmp/fishystuff-production-current.handoff-summary.json admission_file=/tmp/fishystuff-production-admission.json
just gitops-production-current-from-served state_dir=/var/lib/fishystuff/gitops
cargo test -p fishystuff_deploy
```

These run host-local Rust tests for deployment helpers, including a real temporary Dolt repo/file-remote workflow. They do not boot a NixOS VM. `gitops-helper-test` runs the Rust helper tests and the production-current handoff recipe regression. `gitops-check-served`, `gitops-served-summary`, and `gitops-inspect-served` are read-only checks for already-produced local GitOps status, active, rollback-set, rollback readiness, route, admission, and root-readiness documents. `gitops-check-desired-serving` is a read-only JSON preflight for active/retained release tuples before serving. `gitops-retained-releases-json` is also read-only; it derives production-current retained-release JSON from the rollback-set index's exact member documents and refuses inconsistent release identities. `gitops-production-current-handoff` composes the local production-current generator, desired-serving preflight, local closure-path existence checks, active CDN retained-root verification, mgmt unify step, and handoff-summary verification for the exact generated file. `gitops-check-handoff-summary` verifies a handoff summary still matches the exact desired-state SHA-256, local closure paths, and active CDN retention manifest it records. `gitops-write-activation-admission-evidence` writes admission evidence from observed API-meta, DB-backed, and site/CDN probe JSON. `gitops-production-activation-draft` requires explicit admission evidence before writing and unifying a local-only `local-apply` serving draft. `gitops-check-activation-draft` re-checks that draft against the verified handoff and admission evidence. `gitops-review-activation-draft` prints the exact checked tuple for operator inspection without applying it. `gitops-apply-activation-draft` is the guarded local mgmt consumer; the regression uses a fake mgmt binary and verifies the opt-ins and draft hash gate. `gitops-verify-activation-served` ties local served documents back to the checked activation draft after reconciliation. `gitops-production-current-from-served` derives retained JSON from served rollback-set state first, then runs the same handoff. The handoff regression covers that composition with explicit local fixture inputs, tiny local Nix store CDN serving-root fixtures, and a fake mgmt binary.

Flake checks:

```bash
nix build .#checks.x86_64-linux.fishystuff-deploy-tests
nix build .#checks.x86_64-linux.gitops-empty-unify
nix build .#checks.x86_64-linux.gitops-single-host-candidate-vm
nix build .#checks.x86_64-linux.gitops-dolt-fetch-pin-vm
nix build .#checks.x86_64-linux.gitops-dolt-admission-pin-vm
nix build .#checks.x86_64-linux.gitops-served-retained-dolt-fetch-pin-vm
nix build .#checks.x86_64-linux.gitops-multi-environment-candidates-vm
nix build .#checks.x86_64-linux.gitops-multi-environment-served-vm
nix build .#checks.x86_64-linux.gitops-closure-roots-vm
nix build .#checks.x86_64-linux.gitops-unused-release-closure-noop-vm
nix build .#checks.x86_64-linux.gitops-served-closure-roots-vm
nix build .#checks.x86_64-linux.gitops-json-status-escaping-vm
nix build .#checks.x86_64-linux.gitops-served-candidate-vm
nix build .#checks.x86_64-linux.gitops-generated-served-candidate-vm
nix build .#checks.x86_64-linux.gitops-production-vm-serve-fixture-vm
nix build .#checks.x86_64-linux.gitops-production-rollback-transition-vm
nix build .#checks.x86_64-linux.gitops-production-api-meta-vm
nix build .#checks.x86_64-linux.gitops-production-edge-handoff-vm
nix build .#checks.x86_64-linux.gitops-served-symlink-transition-vm
nix build .#checks.x86_64-linux.gitops-served-caddy-handoff-vm
nix build .#checks.x86_64-linux.gitops-served-caddy-rollback-transition-vm
nix build .#checks.x86_64-linux.gitops-served-rollback-transition-vm
nix build .#checks.x86_64-linux.gitops-failed-candidate-vm
nix build .#checks.x86_64-linux.gitops-failed-served-candidate-refusal
nix build .#checks.x86_64-linux.gitops-local-apply-without-optin-refusal
nix build .#checks.x86_64-linux.gitops-local-apply-candidate-vm
nix build .#checks.x86_64-linux.gitops-local-apply-fetch-pin-vm
nix build .#checks.x86_64-linux.gitops-local-apply-http-admission-vm
nix build .#checks.x86_64-linux.gitops-missing-active-artifact-refusal
nix build .#checks.x86_64-linux.gitops-missing-retained-artifact-refusal
nix build .#checks.x86_64-linux.gitops-desired-state-admission-probe
nix build .#checks.x86_64-linux.gitops-desired-state-http-admission-probe
nix build .#checks.x86_64-linux.gitops-desired-state-beta-validate
nix build .#checks.x86_64-linux.gitops-desired-state-production-validate
nix build .#checks.x86_64-linux.gitops-desired-state-production-api-meta
nix build .#checks.x86_64-linux.gitops-desired-state-production-vm-serve-fixture
nix build .#checks.x86_64-linux.gitops-desired-state-production-rollback-transition
nix build .#checks.x86_64-linux.gitops-desired-state-production-serve-shape-refusal
nix build .#checks.x86_64-linux.gitops-desired-state-rollback-transition
nix build .#checks.x86_64-linux.gitops-desired-state-rollback-transition-retention-refusal
nix build .#checks.x86_64-linux.gitops-desired-state-vm-serve-fixture
nix build .#checks.x86_64-linux.gitops-desired-state-serve-without-retained-refusal
nix build .#checks.x86_64-linux.gitops-desired-state-active-retained-refusal
nix build .#checks.x86_64-linux.gitops-desired-state-transition-shape-refusal
nix build .#checks.x86_64-linux.gitops-missing-retained-release-refusal
nix build .#checks.x86_64-linux.gitops-no-retained-release-refusal
nix build .#checks.x86_64-linux.gitops-active-retained-release-refusal
nix build .#checks.x86_64-linux.gitops-rollback-transition-retention-refusal
nix build .#checks.x86_64-linux.gitops-raw-cdn-serve-refusal
nix build .#checks.x86_64-linux.gitops-missing-cdn-runtime-file-refusal
nix build .#checks.x86_64-linux.gitops-missing-cdn-serving-manifest-entry-refusal
nix build .#checks.x86_64-linux.gitops-missing-cdn-retained-root-refusal
nix build .#checks.x86_64-linux.gitops-wrong-cdn-retained-root-refusal
```

Recipe wrappers:

```bash
just gitops-helper-test
just gitops-check-served environment=local-test state_dir=/var/lib/fishystuff/gitops
just gitops-served-summary environment=local-test state_dir=/var/lib/fishystuff/gitops
just gitops-inspect-served environment=local-test state_dir=/var/lib/fishystuff/gitops run_dir=/run/fishystuff/gitops
just gitops-production-current-desired output=/tmp/fishystuff-production-current.desired.json
just gitops-vm-test empty-unify
just gitops-vm-test single-host-candidate
just gitops-vm-test dolt-fetch-pin
just gitops-vm-test dolt-admission-pin
just gitops-vm-test served-retained-dolt-fetch-pin
just gitops-vm-test multi-environment-candidates
just gitops-vm-test multi-environment-served
just gitops-vm-test closure-roots
just gitops-vm-test unused-release-closure-noop
just gitops-vm-test served-closure-roots
just gitops-vm-test json-status-escaping
just gitops-vm-test served-candidate
just gitops-vm-test generated-served-candidate
just gitops-vm-test production-vm-serve-fixture
just gitops-vm-test production-rollback-transition
just gitops-vm-test production-api-meta
just gitops-vm-test production-edge-handoff
just gitops-vm-test served-symlink-transition
just gitops-vm-test served-caddy-handoff
just gitops-vm-test served-caddy-rollback-transition
just gitops-vm-test served-rollback-transition
just gitops-vm-test failed-candidate
just gitops-vm-test failed-served-candidate-refusal
just gitops-vm-test local-apply-without-optin-refusal
just gitops-vm-test local-apply-candidate
just gitops-vm-test local-apply-fetch-pin
just gitops-vm-test local-apply-http-admission
just gitops-vm-test missing-active-artifact-refusal
just gitops-vm-test missing-retained-artifact-refusal
just gitops-vm-test missing-retained-release-refusal
just gitops-vm-test no-retained-release-refusal
just gitops-vm-test active-retained-release-refusal
just gitops-vm-test rollback-transition-retention-refusal
just gitops-vm-test raw-cdn-serve-refusal
just gitops-vm-test missing-cdn-runtime-file-refusal
just gitops-vm-test missing-cdn-serving-manifest-entry-refusal
just gitops-vm-test missing-cdn-retained-root-refusal
just gitops-vm-test wrong-cdn-retained-root-refusal
```

`gitops-empty-unify` type-checks `gitops/main.mcl` with `fixtures/empty.desired.json` and asserts that no local test state paths are created.

`gitops-single-host-candidate-vm` boots a local NixOS VM, runs mgmt against `fixtures/vm-single-host.example.desired.json`, and checks only VM-local state under:

- `/var/lib/fishystuff/gitops-test`
- `/run/fishystuff/gitops-test`

`gitops-dolt-fetch-pin-vm` boots a local NixOS VM, creates a local file-backed Dolt remote, and runs a `fetch_pin` desired state through the `fishystuff_deploy dolt fetch-pin` helper. It verifies mgmt pins an exact release ref in a persistent VM-local Dolt cache, then pushes a second commit to the same local remote and verifies the cache is fetched forward instead of recloned.

`gitops-dolt-admission-pin-vm` extends that path with an explicit `admission_probe.kind = "dolt_sql_scalar"` desired-state object. It runs `fishystuff_deploy dolt probe-sql-scalar` after `fetch_pin`, verifies the materialization status still names the exact requested commit/ref/cache, and runs a single-scalar Dolt SQL query against the pinned release ref before admission can publish `passed_fixture`.

`gitops-served-retained-dolt-fetch-pin-vm` boots a local NixOS VM, creates a local file-backed Dolt remote, and serves a candidate while retaining multiple rollback releases. It checks that the active candidate and every retained rollback release have pinned Dolt refs in the same VM-local cache before active/status/route state is published, while the rollback readiness document records the first retained release as the primary rollback target and rollback-set member documents record every retained release's exact Dolt status path.

`gitops-closure-roots-vm` boots a local NixOS VM, generates desired state with tiny real Nix store artifacts, and checks that `nix:closure` verifies them and file-managed symlinks root them under `/nix/var/nix/gcroots/fishystuff/gitops-test`.

`gitops-unused-release-closure-noop-vm` boots a local NixOS VM in `vm-test-closures` mode with one selected release backed by real tiny store artifacts and one unselected release backed by bogus store paths. It checks that only the selected release is realized/rooted and that the unused release does not create gcroots or candidate files.

`gitops-multi-environment-candidates-vm` boots a local NixOS VM with two enabled arbitrary single-host environments. It checks that each environment publishes its own candidate, admission, and status documents while no active route or served symlinks are created.

`gitops-multi-environment-served-vm` boots a local NixOS VM with two served arbitrary single-host environments on the same host. It checks that each environment gets separate active symlinks under `/var/lib/fishystuff/gitops-test/served/<environment>/{site,cdn}` and separate route documents.

`gitops-served-closure-roots-vm` boots a local NixOS VM with `serve: true` in `vm-test-closures` mode. It checks all active and retained rollback release artifacts are rooted under `/nix/var/nix/gcroots/fishystuff/gitops-test` and reported by Nix as GC roots, verifies the per-release `roots-ready` status files under `/run/fishystuff/gitops-test/roots`, then checks the selected active symlinks and route document.

`gitops-json-status-escaping-vm` boots a local NixOS VM with quote/backslash characters in the exact release identity inputs and checks that candidate, admission, and status JSON files remain parseable and preserve the decoded strings.

`gitops-served-candidate-vm` boots a local NixOS VM with `serve: true` in `vm-test` mode. It checks fixture admission, candidate state, served status, exact release identity, retained rollback release IDs, the VM-local active selection file under `/var/lib/fishystuff/gitops-test/active/local-test.json`, served symlinks under `/var/lib/fishystuff/gitops-test/served/local-test/{site,cdn}`, and the route selection document under `/run/fishystuff/gitops-test/routes/local-test.json`. Its admission fixture reads the selected site root, CDN runtime manifest, runtime JS/WASM files, and CDN serving manifest from the exact release store paths. Its CDN fixture uses the real `cdn-serving-root` derivation to prove current runtime files and retained source-map/runtime files can coexist in one Caddy-facing root. It still asserts that no real FishyStuff services or deployment directories are touched.

`gitops-generated-served-candidate-vm` boots a local NixOS VM with the generated `.#gitops-desired-state-vm-serve-fixture` JSON. It checks the generated release ID, exact API/Dolt/site/CDN fixture paths, the retained `previous-release` object, the CDN serving manifest with retained runtime assets, VM-local served state, and that `vm-test` mode does not create real gcroots or FishyStuff service state.

`gitops-production-vm-serve-fixture-vm` boots a local NixOS VM with the generated `.#gitops-desired-state-production-vm-serve-fixture` JSON. It uses production API/Dolt service bundles and production site content with fixture CDN serving roots, checks served state under the VM-local `production` environment, and asserts `/var/lib/fishystuff/gitops`, `/srv/fishystuff`, and real FishyStuff services remain untouched.

`gitops-production-rollback-transition-vm` boots a local NixOS VM with the generated production rollback desired state. It checks the VM-local `production` served state after rollback, verifies the candidate release remains retained as the primary rollback target, and runs the read-only served-state helper against `environment=production`.

`gitops-production-api-meta-vm` boots a local NixOS VM with the generated production `local-apply` API-meta desired-state shape, then writes an exact runtime desired state from a local file-backed Dolt remote. It pins active and retained rollback commits into `/var/lib/fishystuff/gitops/dolt-cache`, starts an isolated loopback candidate API service against the pinned active release ref, checks `/api/v1/meta` for the exact release identity and Dolt commit, and verifies the served state remains local to the VM.

`gitops-production-edge-handoff-vm` boots a local NixOS VM with the actual `edge-service-bundle-production-gitops-handoff` Caddyfile. It serves GitOps-managed production symlinks under `/var/lib/fishystuff/gitops/served/production/{site,cdn}`, proxies `api.fishystuff.fish` to a loopback API-meta fixture, verifies cache headers for CDN runtime assets, and confirms the bundle does not use `/srv/fishystuff` or beta hostnames.

`gitops-served-symlink-transition-vm` boots a local NixOS VM and runs two served desired states in sequence. It checks that `/var/lib/fishystuff/gitops-test/served/local-test/{site,cdn}` and the route selection document move from the previous release to the candidate release through mgmt reconciliation only.

`gitops-served-caddy-handoff-vm` boots a local NixOS VM, runs Caddy against `/var/lib/fishystuff/gitops-test/served/local-test/{site,cdn}`, and runs two served desired states in sequence. It checks that Caddy serves the previous release, retained previous CDN assets, the candidate release, and retained candidate CDN assets through stable symlink roots without restarting Caddy.

`gitops-served-caddy-rollback-transition-vm` boots a local NixOS VM, runs Caddy against `/var/lib/fishystuff/gitops-test/served/local-test/{site,cdn}`, serves a candidate, and then rolls back to the previous release. It checks that Caddy serves the candidate first, then the previous release after rollback, while the rolled-away candidate CDN runtime asset remains available through the retained CDN serving root.

`gitops-served-rollback-transition-vm` boots a local NixOS VM and runs a served candidate desired state followed by a rollback desired state with `transition.kind = "rollback"`. It checks that `/var/lib/fishystuff/gitops-test/served/local-test/{site,cdn}`, the active/status transition fields, the route selection document, the primary rollback readiness document, and the rollback-set index/member documents move back to the previous release and that the rollback CDN serving root retains the candidate CDN root for stale clients.

`gitops-failed-candidate-vm` boots a local NixOS VM with a deterministic failed admission fixture. It checks that a failed non-serving candidate still publishes candidate, admission, and status facts, records `failure_reason: admission_failed`, and does not create an active selection or served symlinks.

`gitops-failed-served-candidate-refusal` boots a local NixOS VM and checks that a failed admission fixture cannot be served even when desired state asks for `serve: true`.

`gitops-local-apply-without-optin-refusal` boots a local NixOS VM and checks that `local-apply` mode is refused without `FISHYSTUFF_GITOPS_ENABLE_LOCAL_APPLY=1`, before any GitOps, service, or served state paths are written.

`gitops-local-apply-candidate-vm` boots a local NixOS VM and runs a non-serving `local-apply` candidate with `FISHYSTUFF_GITOPS_ENABLE_LOCAL_APPLY=1`. It checks that candidate/status facts are written under `/var/lib/fishystuff/gitops` and `/run/fishystuff/gitops`, that VM-test paths are not used, and that `/srv/fishystuff` and real FishyStuff services remain untouched.

`gitops-local-apply-fetch-pin-vm` boots a local NixOS VM and runs a non-serving `local-apply` candidate with `fetch_pin` Dolt materialization. It creates a local file-backed Dolt remote, pins one exact commit, then pushes and pins a second exact commit using the same local cache under `/var/lib/fishystuff/gitops/dolt-cache`.

`gitops-local-apply-http-admission-vm` boots a local NixOS VM, defines a loopback candidate API service backed by a tiny VM-local Dolt SQL fixture, and runs a serving `local-apply` candidate with `api_upstream`, `api_service`, and `admission_probe.kind = "api_meta"`. It checks that mgmt writes candidate API JSON and env config, starts the isolated real `fishystuff_server` service, probes `/api/v1/meta` for the exact release ID, release identity, Dolt commit, and fixture-backed meta rows, switches desired state to a second release, then rolls back to the retained first release. It verifies the same candidate service is restarted before each new identity can pass admission, publishes served status, active symlinks, route document, rollback transition fields, rollback-set state, and candidate instance/admission documents under `/var/lib/fishystuff/gitops` and `/run/fishystuff/gitops`, and confirms VM-test paths, `/srv/fishystuff`, and real FishyStuff service names remain untouched.

`gitops-missing-active-artifact-refusal` boots a local NixOS VM and checks that hand-written serving desired state cannot omit an active release artifact path.

`gitops-missing-retained-artifact-refusal` boots a local NixOS VM and checks that retained rollback releases cannot omit rollback-critical artifact paths.

`gitops-missing-retained-release-refusal` boots a local NixOS VM and checks that a retained rollback release ID must resolve to a release object before the graph can publish candidate, admission, status, or active state.

`gitops-no-retained-release-refusal` boots a local NixOS VM and checks that `serve: true` must include at least one retained rollback release before the graph can publish candidate, admission, status, or active state.

`gitops-active-retained-release-refusal` checks that a hand-written desired-state file that lists the active release as retained cannot publish candidate, status, active, route, or rollback state.

`gitops-rollback-transition-retention-refusal` checks that a rollback transition cannot publish candidate, status, active, route, rollback, or rollback-set state unless the release being rolled away from remains retained after rollback.

`gitops-desired-state-beta-validate` type-checks the validation-only generated desired-state package from `.#gitops-desired-state-beta-validate`. The generated JSON is built from exact local Nix closure outputs, keeps `cdn_runtime` disabled, keeps `serve: false`, and derives a non-fixture release key from those inputs so `gitops/main.mcl` must select the release named by the enabled environment's `active_release`.

`gitops-desired-state-production-validate` type-checks the production-shaped validation-only generated desired-state package from `.#gitops-desired-state-production-validate`. It uses production API/Dolt service bundles, production site content, `dolt.branch_context = "main"`, keeps `serve: false`, and does not mutate production.

`gitops-desired-state-production-api-meta` type-checks the generated production-shaped `local-apply` API-meta fixture. It requires `api_upstream`, an isolated candidate API service name, and an `api_meta` admission probe targeting that upstream.

`gitops-desired-state-production-vm-serve-fixture` type-checks a production-shaped `vm-test` serving desired-state package. It uses production API/Dolt service bundles and production site content, but fixture CDN serving roots, and does not mutate production.

`gitops-desired-state-production-rollback-transition` type-checks the production-shaped rollback companion. It serves `previous-production-release`, retains the exact candidate release ID derived from the production serve fixture, keeps `dolt.branch_context = "main"` on both releases, and verifies the rollback transition can unify without mutating production.

`gitops-desired-state-production-serve-shape-refusal` checks that production-shaped serving desired state is refused when rollback retention or the CDN runtime closure is missing.

`gitops-desired-state-vm-serve-fixture` type-checks a generated local `vm-test` serving desired-state package. It uses tiny local store artifacts and verifies the generator emits `serve: true` only with API, Dolt service, site, and finalized CDN runtime closures present.

`gitops-desired-state-http-admission-probe` type-checks a generated `local-apply` desired-state package with `api_upstream` plus `admission_probe.kind = "api_meta"`. It verifies HTTP admission does not require Dolt `fetch_pin` materialization, requires probe URLs to target the declared upstream when present, and still unifies with `FISHYSTUFF_GITOPS_ENABLE_LOCAL_APPLY=1`.

`gitops-desired-state-rollback-transition` type-checks a generated local `vm-test` rollback desired-state package. It verifies the generated environment names the previous release as active, retains the rolled-away candidate release, and emits `transition.kind = "rollback"` with the retained `from_release`.

`gitops-desired-state-local-apply-rollback` type-checks a generated `local-apply` rollback desired-state package with `api_service`, `api_upstream`, and API meta admission. It is the fast JSON/schema companion to the heavier real-API VM rollback handoff test.

`gitops-desired-state-rollback-transition-retention-refusal` checks that the generated desired-state helper refuses rollback desired state when `transition.from_release` would not remain retained after rollback.

`gitops-desired-state-serve-without-retained-refusal` checks that the generated desired-state helper refuses to emit serving JSON unless at least one retained rollback release is provided.

`gitops-desired-state-active-retained-refusal` checks that the generated desired-state helper refuses to emit a rollback set that includes the active release.

`gitops-desired-state-transition-shape-refusal` checks that the generated desired-state helper refuses contradictory explicit transition intent, including `candidate` while serving, `activate` while not serving, and `from_release` on non-rollback transitions.

`gitops-raw-cdn-serve-refusal` boots a local NixOS VM and checks the negative path: a serving desired state with `cdn_runtime` pointed at a raw runtime directory must fail before activation because it lacks `cdn-serving-manifest.json`.

`gitops-missing-cdn-runtime-file-refusal` boots a local NixOS VM and checks a later negative path: a serving desired state with `runtime-manifest.json` and `cdn-serving-manifest.json` still fails before activation if the runtime manifest names a JS/WASM file that is not present in the finalized CDN root.

`gitops-missing-cdn-serving-manifest-entry-refusal` boots a local NixOS VM and checks the manifest-accounting path: a serving desired state with runtime files present still fails before activation if `cdn-serving-manifest.json` does not list the selected runtime JS/WASM asset.

`gitops-missing-cdn-retained-root-refusal` boots a local NixOS VM and checks the rollback-retention path: a serving desired state with retained rollback releases still fails before activation if the selected CDN serving manifest records no retained CDN roots.

`gitops-wrong-cdn-retained-root-refusal` boots a local NixOS VM and checks the exact-retention path: a serving desired state still fails before activation when the selected CDN serving manifest retains a different CDN root than the one required by the retained release.

The VM test does not use real secrets, deploy scripts, remote SSH, Hetzner, Cloudflare, beta, or production hosts.

GitOps VM tests set explicit VM memory because the pinned mgmt build can use more than the default 1 GiB NixOS test VM while converging this graph. Current checks use up to 12 GiB.

The flake applies `nix/patches/mgmt-recwatch-bound-watch-path-index.patch` to the GitOps mgmt package used by these checks. Without that local backport, nested state directory creation can trip a mgmt `recwatch` index panic before later `svc` and admission resources reconcile.
