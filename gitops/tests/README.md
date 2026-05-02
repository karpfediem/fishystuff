# GitOps Tests

Local checks:

```bash
nix build .#checks.x86_64-linux.gitops-empty-unify
nix build .#checks.x86_64-linux.gitops-single-host-candidate-vm
nix build .#checks.x86_64-linux.gitops-closure-roots-vm
nix build .#checks.x86_64-linux.gitops-served-candidate-vm
nix build .#checks.x86_64-linux.gitops-generated-served-candidate-vm
nix build .#checks.x86_64-linux.gitops-desired-state-beta-validate
nix build .#checks.x86_64-linux.gitops-desired-state-vm-serve-fixture
```

Recipe wrappers:

```bash
just gitops-vm-test empty-unify
just gitops-vm-test single-host-candidate
just gitops-vm-test closure-roots
just gitops-vm-test served-candidate
just gitops-vm-test generated-served-candidate
```

`gitops-empty-unify` type-checks `gitops/main.mcl` with `fixtures/empty.desired.json` and asserts that no local test state paths are created.

`gitops-single-host-candidate-vm` boots a local NixOS VM, runs mgmt against `fixtures/vm-single-host.example.desired.json`, and checks only VM-local state under:

- `/var/lib/fishystuff/gitops-test`
- `/run/fishystuff/gitops-test`

`gitops-closure-roots-vm` boots a local NixOS VM, generates desired state with tiny real Nix store artifacts, and checks that `nix:closure` verifies them and `nix:gcroot` roots them under `/var/lib/fishystuff/gitops-test/gcroots`.

`gitops-served-candidate-vm` boots a local NixOS VM with `serve: true` in `vm-test` mode. It checks fixture admission, candidate state, served status, exact release identity, retained rollback release IDs, and the VM-local active selection file under `/var/lib/fishystuff/gitops-test/active/local-test.json`. Its admission fixture reads the selected site root and CDN runtime manifest from the exact release store paths. Its CDN fixture uses the real `cdn-serving-root` derivation to prove current runtime files and retained source-map/runtime files can coexist in one Caddy-facing root. It still asserts that no real FishyStuff services or deployment directories are touched.

`gitops-generated-served-candidate-vm` boots a local NixOS VM with the generated `.#gitops-desired-state-vm-serve-fixture` JSON. It checks the generated release ID, exact API/Dolt/site/CDN fixture paths, the CDN serving manifest, VM-local served state, and that `vm-test` mode does not create real gcroots or FishyStuff service state.

`gitops-desired-state-beta-validate` type-checks the validation-only generated desired-state package from `.#gitops-desired-state-beta-validate`. The generated JSON is built from exact local Nix closure outputs, keeps `cdn_runtime` disabled, keeps `serve: false`, and derives a non-fixture release key from those inputs so `gitops/main.mcl` must select the release named by the enabled environment's `active_release`.

`gitops-desired-state-vm-serve-fixture` type-checks a generated local `vm-test` serving desired-state package. It uses tiny local store artifacts and verifies the generator emits `serve: true` only with API, Dolt service, site, and finalized CDN runtime closures present.

The VM test does not use real secrets, deploy scripts, remote SSH, Hetzner, Cloudflare, beta, or production hosts.
