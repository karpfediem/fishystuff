# GitOps Tests

Local checks:

```bash
nix build .#checks.x86_64-linux.gitops-empty-unify
nix build .#checks.x86_64-linux.gitops-single-host-candidate-vm
nix build .#checks.x86_64-linux.gitops-closure-roots-vm
nix build .#checks.x86_64-linux.gitops-served-candidate-vm
nix build .#checks.x86_64-linux.gitops-desired-state-beta-validate
```

Recipe wrappers:

```bash
just gitops-vm-test empty-unify
just gitops-vm-test single-host-candidate
just gitops-vm-test closure-roots
just gitops-vm-test served-candidate
```

`gitops-empty-unify` type-checks `gitops/main.mcl` with `fixtures/empty.desired.json` and asserts that no local test state paths are created.

`gitops-single-host-candidate-vm` boots a local NixOS VM, runs mgmt against `fixtures/vm-single-host.example.desired.json`, and checks only VM-local state under:

- `/var/lib/fishystuff/gitops-test`
- `/run/fishystuff/gitops-test`

`gitops-closure-roots-vm` boots a local NixOS VM, generates desired state with tiny real Nix store artifacts, and checks that `nix:closure` verifies them and `nix:gcroot` roots them under `/var/lib/fishystuff/gitops-test/gcroots`.

`gitops-served-candidate-vm` boots a local NixOS VM with `serve: true` in `vm-test` mode. It checks fixture admission, candidate state, served status, exact release identity, retained rollback release IDs, and the VM-local active selection file under `/var/lib/fishystuff/gitops-test/active/local-test.json`. Its admission fixture reads the selected site root and CDN runtime manifest from the exact release store paths. Its CDN fixture uses the real `cdn-serving-root` derivation to prove current runtime files and retained source-map/runtime files can coexist in one Caddy-facing root. It still asserts that no real FishyStuff services or deployment directories are touched.

`gitops-desired-state-beta-validate` type-checks the validation-only generated desired-state package from `.#gitops-desired-state-beta-validate`. The generated JSON is built from exact local Nix closure outputs and keeps `serve: false`.

The VM test does not use real secrets, deploy scripts, remote SSH, Hetzner, Cloudflare, beta, or production hosts.
