# FishyStuff GitOps Mgmt Substrate

This directory is a clean-slate mgmt module repository for the next FishyStuff deployment substrate. It intentionally does not extend the old beta MCL graph under `mgmt/`.

The first milestone is local-only:

1. Decode a desired-state JSON file.
2. Type-check/unify the graph.
3. Express a host-local single-release candidate.
4. Run a NixOS VM test without touching beta, production, Hetzner, Cloudflare, SSH, or SecretSpec beta/prod profiles.

The default desired-state file is `gitops/fixtures/empty.desired.json`. Override it with:

```bash
FISHYSTUFF_GITOPS_STATE_FILE=gitops/fixtures/vm-single-host.example.desired.json
```

## Commands

```bash
just gitops-unify
just gitops-unify auto gitops/fixtures/beta-single-host.example.desired.json
just gitops-vm-test empty-unify
just gitops-vm-test single-host-candidate
just gitops-vm-test closure-roots
just gitops-vm-test json-status-escaping
just gitops-vm-test served-candidate
just gitops-vm-test generated-served-candidate
just gitops-vm-test served-symlink-transition
just gitops-vm-test served-rollback-transition
just gitops-vm-test missing-retained-release-refusal
just gitops-vm-test no-retained-release-refusal
just gitops-vm-test raw-cdn-serve-refusal
just gitops-vm-test missing-cdn-runtime-file-refusal
just gitops-vm-test missing-cdn-serving-manifest-entry-refusal
just gitops-vm-test missing-cdn-retained-root-refusal
just gitops-vm-test wrong-cdn-retained-root-refusal
```

The flake checks added by this milestone are:

```bash
nix build .#checks.x86_64-linux.gitops-empty-unify
nix build .#checks.x86_64-linux.gitops-single-host-candidate-vm
nix build .#checks.x86_64-linux.gitops-closure-roots-vm
nix build .#checks.x86_64-linux.gitops-json-status-escaping-vm
nix build .#checks.x86_64-linux.gitops-served-candidate-vm
nix build .#checks.x86_64-linux.gitops-generated-served-candidate-vm
nix build .#checks.x86_64-linux.gitops-served-symlink-transition-vm
nix build .#checks.x86_64-linux.gitops-served-rollback-transition-vm
nix build .#checks.x86_64-linux.gitops-desired-state-beta-validate
nix build .#checks.x86_64-linux.gitops-desired-state-vm-serve-fixture
nix build .#checks.x86_64-linux.gitops-desired-state-serve-without-retained-refusal
nix build .#checks.x86_64-linux.gitops-missing-retained-release-refusal
nix build .#checks.x86_64-linux.gitops-no-retained-release-refusal
nix build .#checks.x86_64-linux.gitops-raw-cdn-serve-refusal
nix build .#checks.x86_64-linux.gitops-missing-cdn-runtime-file-refusal
nix build .#checks.x86_64-linux.gitops-missing-cdn-serving-manifest-entry-refusal
nix build .#checks.x86_64-linux.gitops-missing-cdn-retained-root-refusal
nix build .#checks.x86_64-linux.gitops-wrong-cdn-retained-root-refusal
```

`.#gitops-desired-state-beta-validate` emits a validation-only desired-state JSON file from exact Nix build outputs: API bundle, Dolt service bundle, and site content. It deliberately keeps `cdn_runtime` disabled so normal repo checks do not depend on private or ignored CDN staging state. Its release key is derived from the exact available tuple by default. It sets `serve: false`, `mode: validate`, and a placeholder Dolt commit; it is not a deploy/apply command.

Real deployment desired state should import `nix/packages/gitops-desired-state.nix` from an operator/deployment flake and pass exact `doltCommit`, service bundles, site content, and finalized CDN serving roots as Nix values. The generated GitOps validation package intentionally does not use ambient environment variables for those deployment-critical inputs.

`.#gitops-desired-state-vm-serve-fixture` emits a local `vm-test` desired-state file with tiny store artifacts for API, Dolt service, site, a finalized CDN serving root, and one retained previous release object. The package generator refuses `serve: true` unless all four active release artifacts are present.

`gitops-desired-state-serve-without-retained-refusal` proves the generated desired-state helper refuses `serve: true` without at least one retained rollback release.

`gitops-json-status-escaping-vm` proves the VM-local JSON outputs preserve quote/backslash characters from the exact release identity tuple instead of emitting malformed JSON.

`gitops-generated-served-candidate-vm` boots a local NixOS VM with that generated desired state. It verifies the graph can express a served candidate from generated JSON, checks the selected site/CDN runtime fixture, verifies the generated retained `previous-release` object, and confirms vm-test mode does not create real FishyStuff service state or gcroots.

`gitops-served-symlink-transition-vm` boots one local NixOS VM, serves one desired state, then serves a second desired state. It proves the VM-local active symlinks move by reconciliation through desired state, not by an imperative deployment command.

`gitops-served-rollback-transition-vm` boots one local NixOS VM, serves a candidate, then rolls back to the previous release by changing desired state. It proves rollback is represented as another reconciled active-release transition while retaining the candidate CDN root for stale clients.

`gitops-missing-retained-release-refusal` proves retained rollback release IDs are not informational labels: each retained ID must reference a release object before candidate/admission/status/active state can be published.

`gitops-no-retained-release-refusal` proves serving is refused when no rollback release is retained.

`gitops-raw-cdn-serve-refusal` is a negative VM check. It proves a `serve: true` desired state cannot pass admission when `cdn_runtime` points at a raw runtime directory instead of a finalized CDN serving root with `cdn-serving-manifest.json`.

`gitops-missing-cdn-runtime-file-refusal` is a negative VM check. It proves a finalized-looking CDN serving root still cannot pass admission when the selected runtime manifest names a missing JS/WASM file.

`gitops-missing-cdn-serving-manifest-entry-refusal` is a negative VM check. It proves the finalized CDN serving manifest must account for the selected runtime JS/WASM asset, not merely exist beside it.

`gitops-missing-cdn-retained-root-refusal` is a negative VM check. It proves a serving environment that retains rollback releases must use a CDN serving root whose manifest records retained CDN roots.

`gitops-wrong-cdn-retained-root-refusal` is a negative VM check. It proves the retained CDN root must be the root required by the retained release, not merely any retained root.

## Desired State

The minimal JSON shape is:

```json
{
  "cluster": "local-test",
  "generation": 1,
  "mode": "validate",
  "hosts": {
    "vm-single-host": {
      "enabled": true,
      "role": "single-site",
      "hostname": "vm-single-host"
    }
  },
  "releases": {
    "example-release": {
      "generation": 1,
      "git_rev": "example",
      "dolt_commit": "example",
      "closures": {
        "api": {
          "enabled": true,
          "store_path": "/nix/store/example-api",
          "gcroot_path": "/var/lib/fishystuff/gitops/gcroots/example-release/api"
        }
      },
      "dolt": {
        "repository": "fishystuff/fishystuff",
        "commit": "example",
        "branch_context": "beta",
        "mode": "read_only"
      }
    }
  },
  "environments": {
    "local-test": {
      "enabled": true,
      "strategy": "single_active",
      "host": "vm-single-host",
      "active_release": "example-release",
      "retained_releases": [],
      "serve": false
    }
  }
}
```

Supported modes:

- `validate`: decode, shape, and unify only. It does not write local state and does not run admission.
- `vm-test`: create only VM-local files under `/var/lib/fishystuff/gitops-test` and `/run/fishystuff/gitops-test`.
- `vm-test-closures`: VM-only mode that also verifies real Nix store paths with `nix:closure` and roots them under `/var/lib/fishystuff/gitops-test/gcroots`.
- `local-apply`: reserved for future host-local activation. The first milestone does not include fixtures that use it.

The first milestone intentionally recognizes only one enabled environment at a time, currently the `local-test` or `beta` single-host environments used by the fixtures and generated validation state. The active release is selected by that environment's `active_release` key; the checked-in fixtures still use `example-release`, while the generated beta validation package derives a different release key from exact inputs to prove the graph is not hardcoded to the fixture name. General multi-environment traversal should be added with more mgmt language coverage and VM tests.

## Graph Shape

The graph is structured around this desired-state flow:

```text
desired state object
  -> exact release identity
  -> Nix closures realized/rooted
  -> candidate host-local instance
  -> app-specific admission probes
  -> active route/symlink/service selection
  -> status/health publication
```

The current `release_id` is the desired-state release key. Generated desired state derives that key from a content hash of the exact release tuple; fixture desired state may still use readable names such as `example-release`. The graph also emits `release_identity`, a deterministic string derived from the release key, generation, Git revision, Dolt identity/mode, and closure paths. The tuple is recorded directly in candidate, admission, active, and status documents so mismatched activation inputs are visible.

## Release Artifact Contract

A release candidate is the exact tuple of:

- Git revision
- Dolt commit/data identity
- API closure
- Dolt service closure
- site content closure
- CDN runtime/serving-root closure
- retained rollback release IDs for the selected environment
- admission result for that exact tuple

The `cdn_runtime` closure is expected to be the CDN serving root that Caddy can point at directly. For real deployments this should be built from the current CDN content plus retained immutable assets from prior CDN roots, for example with `.#cdn-serving-root` or an equivalent derivation constructed from exact store paths. The `cdn-serving-root` derivation validates the current root's runtime manifest when present and refuses a root whose selected JS/WASM files are missing. The GitOps graph should receive that final store path as desired state; it should not infer prior roots from a mutable remote host during activation. Serving admission requires this root to include `cdn-serving-manifest.json`, which records the current root and retained roots. When the desired environment retains rollback releases, serving admission checks that the active CDN serving manifest accounts for the CDN root required by each retained release. If a retained release's `cdn_runtime` is itself a serving root, admission checks its recorded `current_root`; otherwise it checks the retained `cdn_runtime` path directly.

`retained_releases` on an environment records the releases intentionally kept hot for rollback and for stale client HTML/runtime references. Each retained ID must reference a release object in desired state, and serving requires at least one retained rollback release. Activation records this list in the local active/status documents so operators can tell which rollback set was selected with the active release.

Source maps are public in production because the project is open source. They are emitted with content-hashed filenames and retained as immutable CDN assets, but generated HTML/runtime manifests do not eagerly reference them, so normal users do not fetch them.

## Safety Defaults

This graph does not import Hetzner, Cloudflare, or SSH providers. It does not call deploy scripts. It does not start FishyStuff system services. The VM fixture disables closure realization, so it never tries to realize fake `/nix/store` paths.

`gitops/modules/fishy/nix.mcl` emits `nix:closure` and `nix:gcroot` only in `vm-test-closures` and future `local-apply` mode. In `validate` and plain `vm-test`, enabled artifacts are validation no-ops. The flake checks and `gitops-unify` default to the pinned local `~/code/mgmt-fishystuff-beta/` commit recorded in `flake.lock`/`scripts/recipes/gitops-unify.sh` because it contains the integrated Nix primitives needed to type-check this graph.

The VM runtime test binds mgmt's embedded etcd to `127.0.0.1` inside the test VM. It does not connect to beta, production, Hetzner, Cloudflare, SSH, or operator SecretSpec profiles.

`gitops-closure-roots-vm` generates desired state from tiny real Nix store artifacts inside the test derivation. It proves closure verification and gcroot creation without using fake enabled store paths or serving anything.

The closure and gcroot resources are both declared for each enabled artifact. A strict `nix:closure -> nix:gcroot` resource edge is intentionally deferred: the pinned mgmt build verified closures but did not progress the dependent gcroot behind that edge in the VM test. Reintroduce that edge only with a VM regression test proving the ordered behavior.

`gitops-served-candidate-vm` keeps activation local and synthetic. When desired state requests `serve: true` in `vm-test` mode, fixture admission must be `passed_fixture`; the local admission fixture also reads the selected site root, CDN runtime manifest, runtime JS/WASM files, and CDN serving manifest from the exact store paths in the release tuple. The graph then writes an active selection document under `/var/lib/fishystuff/gitops-test/active/<environment>.json` and VM-local served symlinks under `/var/lib/fishystuff/gitops-test/served/{site,cdn}`. This is the first safe shape of the future route/symlink switch. It does not start FishyStuff services, write `/srv/fishystuff`, or touch real beta/prod state.

Fallbacks introduced: none to the old beta deployment graph. The validation no-op is a mode-specific safety guard, not compatibility with an old code path.

## Admission

Admission is modeled separately from graph acceptance. In `validate`, admission is `not_run` and must not be treated as success. In `vm-test`, admission is a deterministic local fixture (`passed_fixture`) written under `/run/fishystuff/gitops-test/admission/`. For serving fixtures, this local probe must be able to read the selected `site/index.html`, `cdn_runtime/map/runtime-manifest.json`, the selected runtime JS/WASM files, and `cdn_runtime/cdn-serving-manifest.json`. The serving manifest must also account for `runtime-manifest.json` and the selected runtime JS/WASM asset paths.

Future real admission should probe the exact candidate tuple:

- API `readyz`
- API `/api/v1/meta`
- A representative DB-backed API route that would catch schema/data mismatches such as the previous `languagedata` versus `languagedata_en` issue
- Branch-qualified Dolt behavior when branch context matters
- Site content references the selected CDN runtime assets

No hand-maintained API/schema compatibility contract should be added. Compatibility should be inferred by admission probes against the exact candidate API and Dolt state.

## Status

The first milestone writes local status only in `vm-test`/future local modes:

```text
/var/lib/fishystuff/gitops-test/status/<environment>.json
```

Status includes:

- `desired_generation`
- `release_id`
- `environment`
- `host`
- `phase`
- `admission_state`
- `served`
- `failure_reason`

Local active selection is written only when a VM/local desired state is explicitly serving:

```text
/var/lib/fishystuff/gitops-test/active/<environment>.json
/var/lib/fishystuff/gitops-test/served/site
/var/lib/fishystuff/gitops-test/served/cdn
```

The active selection document includes the desired generation that selected the served symlinks so route state can be correlated with the desired-state object that produced it.

KV publication can be added later when the status consumer is clear.

## Fast Deployment Invariant

A release may become served only after expensive work is already complete:

- closures realized
- gcroots present
- Dolt data identity known/materialized
- candidate admission either passed or intentionally not required in non-serving validate mode
- previous rollback target retained

Activation should later be limited to small state transitions:

- `active_release` pointer
- active symlink
- Caddy route/upstream switch
- small service restart/reload

There should not be an imperative plan/apply deployment tool.

## Future Preview Architecture

Top-level mgmt should eventually watch Git and Dolt state through small primitives or sidecar facts. Feature branches become preview environment desired-state objects.

Default preview placement should be an already-warm beta host. Branch-specific Hetzner VMs are optional placement when isolation is needed. `hetzner:vm` reconciliation should be for placement/provisioning, not fast activation.

Local NixOS VM tests should cover the host-local preview shape before real preview infrastructure exists.

## Future Production Blue/Green

Production should eventually have blue and green instances. The inactive color is preloaded and privately admitted. Served color changes by route/symlink switch.

Automatic rollback is a state transition back to the last healthy color. The old color must remain hot, and rollback must not require fetching or building during the incident.

Future VM tests should simulate:

- candidate pass
- candidate fail
- rollback to previous served color
- missing rollback artifact refusal
- noisy health signal debouncing

## Failure Classes Addressed

This first milestone addresses prior deployment failure classes by structure rather than production machinery:

- Unauthorized beta deploys during validation: no remote providers or deploy scripts are imported.
- API moving live against stale Dolt data: release, Dolt identity, instance, and admission are separate objects.
- Confusing graph acceptance with target health: admission is explicit and `not_run` in validate mode.
- Site content moving live without matching CDN runtime content: site and CDN runtime are part of the same release candidate.
- Slow rollback due to missing rooted previous closures: release closure/gcroot work is a prerequisite for future serving.
- Dolt snapshot materialization preserving wrong ownership/mode: this graph records Dolt identity but does not materialize snapshots yet.
- Diagnostic/manual processes conflicting with managed services: the first graph does not start real services.
