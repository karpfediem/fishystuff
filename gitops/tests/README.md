# GitOps Tests

Local checks:

```bash
nix build .#checks.x86_64-linux.gitops-empty-unify
nix build .#checks.x86_64-linux.gitops-single-host-candidate-vm
nix build .#checks.x86_64-linux.gitops-closure-roots-vm
nix build .#checks.x86_64-linux.gitops-json-status-escaping-vm
nix build .#checks.x86_64-linux.gitops-served-candidate-vm
nix build .#checks.x86_64-linux.gitops-generated-served-candidate-vm
nix build .#checks.x86_64-linux.gitops-desired-state-beta-validate
nix build .#checks.x86_64-linux.gitops-desired-state-vm-serve-fixture
nix build .#checks.x86_64-linux.gitops-missing-retained-release-refusal
nix build .#checks.x86_64-linux.gitops-no-retained-release-refusal
nix build .#checks.x86_64-linux.gitops-raw-cdn-serve-refusal
nix build .#checks.x86_64-linux.gitops-missing-cdn-runtime-file-refusal
nix build .#checks.x86_64-linux.gitops-missing-cdn-serving-manifest-entry-refusal
nix build .#checks.x86_64-linux.gitops-missing-cdn-retained-root-refusal
```

Recipe wrappers:

```bash
just gitops-vm-test empty-unify
just gitops-vm-test single-host-candidate
just gitops-vm-test closure-roots
just gitops-vm-test json-status-escaping
just gitops-vm-test served-candidate
just gitops-vm-test generated-served-candidate
just gitops-vm-test missing-retained-release-refusal
just gitops-vm-test no-retained-release-refusal
just gitops-vm-test raw-cdn-serve-refusal
just gitops-vm-test missing-cdn-runtime-file-refusal
just gitops-vm-test missing-cdn-serving-manifest-entry-refusal
just gitops-vm-test missing-cdn-retained-root-refusal
```

`gitops-empty-unify` type-checks `gitops/main.mcl` with `fixtures/empty.desired.json` and asserts that no local test state paths are created.

`gitops-single-host-candidate-vm` boots a local NixOS VM, runs mgmt against `fixtures/vm-single-host.example.desired.json`, and checks only VM-local state under:

- `/var/lib/fishystuff/gitops-test`
- `/run/fishystuff/gitops-test`

`gitops-closure-roots-vm` boots a local NixOS VM, generates desired state with tiny real Nix store artifacts, and checks that `nix:closure` verifies them and `nix:gcroot` roots them under `/var/lib/fishystuff/gitops-test/gcroots`.

`gitops-json-status-escaping-vm` boots a local NixOS VM with quote/backslash characters in the exact release identity inputs and checks that candidate, admission, and status JSON files remain parseable and preserve the decoded strings.

`gitops-served-candidate-vm` boots a local NixOS VM with `serve: true` in `vm-test` mode. It checks fixture admission, candidate state, served status, exact release identity, retained rollback release IDs, and the VM-local active selection file under `/var/lib/fishystuff/gitops-test/active/local-test.json`. Its admission fixture reads the selected site root, CDN runtime manifest, runtime JS/WASM files, and CDN serving manifest from the exact release store paths. Its CDN fixture uses the real `cdn-serving-root` derivation to prove current runtime files and retained source-map/runtime files can coexist in one Caddy-facing root. It still asserts that no real FishyStuff services or deployment directories are touched.

`gitops-generated-served-candidate-vm` boots a local NixOS VM with the generated `.#gitops-desired-state-vm-serve-fixture` JSON. It checks the generated release ID, exact API/Dolt/site/CDN fixture paths, the retained `previous-release` object, the CDN serving manifest with retained runtime assets, VM-local served state, and that `vm-test` mode does not create real gcroots or FishyStuff service state.

`gitops-missing-retained-release-refusal` boots a local NixOS VM and checks that a retained rollback release ID must resolve to a release object before the graph can publish candidate, admission, status, or active state.

`gitops-no-retained-release-refusal` boots a local NixOS VM and checks that `serve: true` must include at least one retained rollback release before the graph can publish candidate, admission, status, or active state.

`gitops-desired-state-beta-validate` type-checks the validation-only generated desired-state package from `.#gitops-desired-state-beta-validate`. The generated JSON is built from exact local Nix closure outputs, keeps `cdn_runtime` disabled, keeps `serve: false`, and derives a non-fixture release key from those inputs so `gitops/main.mcl` must select the release named by the enabled environment's `active_release`.

`gitops-desired-state-vm-serve-fixture` type-checks a generated local `vm-test` serving desired-state package. It uses tiny local store artifacts and verifies the generator emits `serve: true` only with API, Dolt service, site, and finalized CDN runtime closures present.

`gitops-raw-cdn-serve-refusal` boots a local NixOS VM and checks the negative path: a serving desired state with `cdn_runtime` pointed at a raw runtime directory must fail before activation because it lacks `cdn-serving-manifest.json`.

`gitops-missing-cdn-runtime-file-refusal` boots a local NixOS VM and checks a later negative path: a serving desired state with `runtime-manifest.json` and `cdn-serving-manifest.json` still fails before activation if the runtime manifest names a JS/WASM file that is not present in the finalized CDN root.

`gitops-missing-cdn-serving-manifest-entry-refusal` boots a local NixOS VM and checks the manifest-accounting path: a serving desired state with runtime files present still fails before activation if `cdn-serving-manifest.json` does not list the selected runtime JS/WASM asset.

`gitops-missing-cdn-retained-root-refusal` boots a local NixOS VM and checks the rollback-retention path: a serving desired state with retained rollback releases still fails before activation if the selected CDN serving manifest records no retained CDN roots.

The VM test does not use real secrets, deploy scripts, remote SSH, Hetzner, Cloudflare, beta, or production hosts.
