# GitOps Tests

Fast local checks:

```bash
just gitops-helper-test
cargo test -p fishystuff_deploy
```

These run host-local Rust tests for deployment helpers, including a real temporary Dolt repo/file-remote workflow. They do not boot a NixOS VM.

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
nix build .#checks.x86_64-linux.gitops-served-symlink-transition-vm
nix build .#checks.x86_64-linux.gitops-served-caddy-handoff-vm
nix build .#checks.x86_64-linux.gitops-served-rollback-transition-vm
nix build .#checks.x86_64-linux.gitops-failed-candidate-vm
nix build .#checks.x86_64-linux.gitops-failed-served-candidate-refusal
nix build .#checks.x86_64-linux.gitops-local-apply-without-optin-refusal
nix build .#checks.x86_64-linux.gitops-missing-active-artifact-refusal
nix build .#checks.x86_64-linux.gitops-missing-retained-artifact-refusal
nix build .#checks.x86_64-linux.gitops-desired-state-admission-probe
nix build .#checks.x86_64-linux.gitops-desired-state-beta-validate
nix build .#checks.x86_64-linux.gitops-desired-state-vm-serve-fixture
nix build .#checks.x86_64-linux.gitops-desired-state-serve-without-retained-refusal
nix build .#checks.x86_64-linux.gitops-desired-state-active-retained-refusal
nix build .#checks.x86_64-linux.gitops-missing-retained-release-refusal
nix build .#checks.x86_64-linux.gitops-no-retained-release-refusal
nix build .#checks.x86_64-linux.gitops-active-retained-release-refusal
nix build .#checks.x86_64-linux.gitops-raw-cdn-serve-refusal
nix build .#checks.x86_64-linux.gitops-missing-cdn-runtime-file-refusal
nix build .#checks.x86_64-linux.gitops-missing-cdn-serving-manifest-entry-refusal
nix build .#checks.x86_64-linux.gitops-missing-cdn-retained-root-refusal
nix build .#checks.x86_64-linux.gitops-wrong-cdn-retained-root-refusal
```

Recipe wrappers:

```bash
just gitops-helper-test
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
just gitops-vm-test served-symlink-transition
just gitops-vm-test served-caddy-handoff
just gitops-vm-test served-rollback-transition
just gitops-vm-test failed-candidate
just gitops-vm-test failed-served-candidate-refusal
just gitops-vm-test local-apply-without-optin-refusal
just gitops-vm-test missing-active-artifact-refusal
just gitops-vm-test missing-retained-artifact-refusal
just gitops-vm-test missing-retained-release-refusal
just gitops-vm-test no-retained-release-refusal
just gitops-vm-test active-retained-release-refusal
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

`gitops-served-retained-dolt-fetch-pin-vm` boots a local NixOS VM, creates a local file-backed Dolt remote, and serves a candidate while retaining multiple rollback releases. It checks that the active candidate and every retained rollback release have pinned Dolt refs in the same VM-local cache before active/status/route state is published, while the rollback readiness document records the first retained release as the primary rollback target.

`gitops-closure-roots-vm` boots a local NixOS VM, generates desired state with tiny real Nix store artifacts, and checks that `nix:closure` verifies them and `nix:gcroot` roots them under `/var/lib/fishystuff/gitops-test/gcroots`.

`gitops-unused-release-closure-noop-vm` boots a local NixOS VM in `vm-test-closures` mode with one selected release backed by real tiny store artifacts and one unselected release backed by bogus store paths. It checks that only the selected release is realized/rooted and that the unused release does not create gcroots or candidate files.

`gitops-multi-environment-candidates-vm` boots a local NixOS VM with two enabled arbitrary single-host environments. It checks that each environment publishes its own candidate, admission, and status documents while no active route or served symlinks are created.

`gitops-multi-environment-served-vm` boots a local NixOS VM with two served arbitrary single-host environments on the same host. It checks that each environment gets separate active symlinks under `/var/lib/fishystuff/gitops-test/served/<environment>/{site,cdn}` and separate route documents.

`gitops-served-closure-roots-vm` boots a local NixOS VM with `serve: true` in `vm-test-closures` mode. It checks all active and retained rollback release artifacts are rooted under `/var/lib/fishystuff/gitops-test/gcroots`, then checks the selected active symlinks and route document.

`gitops-json-status-escaping-vm` boots a local NixOS VM with quote/backslash characters in the exact release identity inputs and checks that candidate, admission, and status JSON files remain parseable and preserve the decoded strings.

`gitops-served-candidate-vm` boots a local NixOS VM with `serve: true` in `vm-test` mode. It checks fixture admission, candidate state, served status, exact release identity, retained rollback release IDs, the VM-local active selection file under `/var/lib/fishystuff/gitops-test/active/local-test.json`, served symlinks under `/var/lib/fishystuff/gitops-test/served/local-test/{site,cdn}`, and the route selection document under `/run/fishystuff/gitops-test/routes/local-test.json`. Its admission fixture reads the selected site root, CDN runtime manifest, runtime JS/WASM files, and CDN serving manifest from the exact release store paths. Its CDN fixture uses the real `cdn-serving-root` derivation to prove current runtime files and retained source-map/runtime files can coexist in one Caddy-facing root. It still asserts that no real FishyStuff services or deployment directories are touched.

`gitops-generated-served-candidate-vm` boots a local NixOS VM with the generated `.#gitops-desired-state-vm-serve-fixture` JSON. It checks the generated release ID, exact API/Dolt/site/CDN fixture paths, the retained `previous-release` object, the CDN serving manifest with retained runtime assets, VM-local served state, and that `vm-test` mode does not create real gcroots or FishyStuff service state.

`gitops-served-symlink-transition-vm` boots a local NixOS VM and runs two served desired states in sequence. It checks that `/var/lib/fishystuff/gitops-test/served/local-test/{site,cdn}` and the route selection document move from the previous release to the candidate release through mgmt reconciliation only.

`gitops-served-caddy-handoff-vm` boots a local NixOS VM, runs Caddy against `/var/lib/fishystuff/gitops-test/served/local-test/{site,cdn}`, and runs two served desired states in sequence. It checks that Caddy serves the previous release, retained previous CDN assets, the candidate release, and retained candidate CDN assets through stable symlink roots without restarting Caddy.

`gitops-served-rollback-transition-vm` boots a local NixOS VM and runs a served candidate desired state followed by a rollback desired state. It checks that `/var/lib/fishystuff/gitops-test/served/local-test/{site,cdn}`, the route selection document, the primary rollback readiness document, and the rollback-set index/member documents move back to the previous release and that the rollback CDN serving root retains the candidate CDN root for stale clients.

`gitops-failed-candidate-vm` boots a local NixOS VM with a deterministic failed admission fixture. It checks that a failed non-serving candidate still publishes candidate, admission, and status facts, records `failure_reason: admission_failed`, and does not create an active selection or served symlinks.

`gitops-failed-served-candidate-refusal` boots a local NixOS VM and checks that a failed admission fixture cannot be served even when desired state asks for `serve: true`.

`gitops-local-apply-without-optin-refusal` boots a local NixOS VM and checks that `local-apply` mode is refused without `FISHYSTUFF_GITOPS_ENABLE_LOCAL_APPLY=1`, before any GitOps, service, or served state paths are written.

`gitops-missing-active-artifact-refusal` boots a local NixOS VM and checks that hand-written serving desired state cannot omit an active release artifact path.

`gitops-missing-retained-artifact-refusal` boots a local NixOS VM and checks that retained rollback releases cannot omit rollback-critical artifact paths.

`gitops-missing-retained-release-refusal` boots a local NixOS VM and checks that a retained rollback release ID must resolve to a release object before the graph can publish candidate, admission, status, or active state.

`gitops-no-retained-release-refusal` boots a local NixOS VM and checks that `serve: true` must include at least one retained rollback release before the graph can publish candidate, admission, status, or active state.

`gitops-active-retained-release-refusal` checks that a hand-written desired-state file that lists the active release as retained cannot publish candidate, status, active, route, or rollback state.

`gitops-desired-state-beta-validate` type-checks the validation-only generated desired-state package from `.#gitops-desired-state-beta-validate`. The generated JSON is built from exact local Nix closure outputs, keeps `cdn_runtime` disabled, keeps `serve: false`, and derives a non-fixture release key from those inputs so `gitops/main.mcl` must select the release named by the enabled environment's `active_release`.

`gitops-desired-state-vm-serve-fixture` type-checks a generated local `vm-test` serving desired-state package. It uses tiny local store artifacts and verifies the generator emits `serve: true` only with API, Dolt service, site, and finalized CDN runtime closures present.

`gitops-desired-state-serve-without-retained-refusal` checks that the generated desired-state helper refuses to emit serving JSON unless at least one retained rollback release is provided.

`gitops-desired-state-active-retained-refusal` checks that the generated desired-state helper refuses to emit a rollback set that includes the active release.

`gitops-raw-cdn-serve-refusal` boots a local NixOS VM and checks the negative path: a serving desired state with `cdn_runtime` pointed at a raw runtime directory must fail before activation because it lacks `cdn-serving-manifest.json`.

`gitops-missing-cdn-runtime-file-refusal` boots a local NixOS VM and checks a later negative path: a serving desired state with `runtime-manifest.json` and `cdn-serving-manifest.json` still fails before activation if the runtime manifest names a JS/WASM file that is not present in the finalized CDN root.

`gitops-missing-cdn-serving-manifest-entry-refusal` boots a local NixOS VM and checks the manifest-accounting path: a serving desired state with runtime files present still fails before activation if `cdn-serving-manifest.json` does not list the selected runtime JS/WASM asset.

`gitops-missing-cdn-retained-root-refusal` boots a local NixOS VM and checks the rollback-retention path: a serving desired state with retained rollback releases still fails before activation if the selected CDN serving manifest records no retained CDN roots.

`gitops-wrong-cdn-retained-root-refusal` boots a local NixOS VM and checks the exact-retention path: a serving desired state still fails before activation when the selected CDN serving manifest retains a different CDN root than the one required by the retained release.

The VM test does not use real secrets, deploy scripts, remote SSH, Hetzner, Cloudflare, beta, or production hosts.

GitOps VM tests set explicit VM memory because the pinned mgmt build can use more than the default 1 GiB NixOS test VM while converging this graph. Heavier Dolt/rollback tests use 8 GiB.
