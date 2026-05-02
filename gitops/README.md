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
just gitops-vm-test dolt-fetch-pin
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
nix build .#checks.x86_64-linux.gitops-dolt-fetch-pin-vm
nix build .#checks.x86_64-linux.gitops-dolt-admission-pin-vm
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

`gitops-dolt-fetch-pin-vm` boots one local NixOS VM, creates a local file-backed Dolt remote, and reconciles a `fetch_pin` desired state against it. The test first pins commit 1 in a persistent VM-local cache, then pushes commit 2 to the same local remote and changes desired state. It verifies the existing cache is fetched forward, the release ref points at the exact desired commit, and no `.dolt` snapshot/full closure path is used.

`gitops-dolt-admission-pin-vm` adds a local DB-backed fixture admission step to the `fetch_pin` path. Desired state includes `admission_probe.kind = "dolt_sql_fixture"` with a single-scalar SQL query and expected value. The graph runs the probe only after the Dolt materialization helper has pinned the exact release ref, and the helper refuses admission if the materialization status, ref hash, or query result does not match the desired commit tuple.

`gitops-json-status-escaping-vm` proves the VM-local JSON outputs preserve quote/backslash characters from the exact release identity tuple instead of emitting malformed JSON.

`gitops-unused-release-closure-noop-vm` boots a local NixOS VM in `vm-test-closures` mode with one selected release backed by real tiny store artifacts and one unselected release backed by bogus store paths. It proves the graph validates the release catalog but only realizes and roots releases requested by enabled environments as active or retained rollback releases.

`gitops-generated-served-candidate-vm` boots a local NixOS VM with that generated desired state. It verifies the graph can express a served candidate from generated JSON, checks the selected site/CDN runtime fixture, verifies the generated retained `previous-release` object, writes the VM-local route selection document, and confirms vm-test mode does not create real FishyStuff service state or gcroots.

`gitops-served-symlink-transition-vm` boots one local NixOS VM, serves one desired state, then serves a second desired state. It proves the VM-local active symlinks and route selection document move by reconciliation through desired state, not by an imperative deployment command.

`gitops-served-caddy-handoff-vm` boots one local NixOS VM, runs Caddy against the VM-local served site/CDN symlink roots, and then changes the served desired state. It proves the future Caddy-facing handoff shape can serve the selected release and then observe the next selected release through stable symlink roots without restarting Caddy.

`gitops-served-rollback-transition-vm` boots one local NixOS VM, serves a candidate, then rolls back to the previous release by changing desired state. It proves rollback is represented as another reconciled active-release transition while retaining the candidate CDN root for stale clients and updating the route selection document.

`gitops-failed-candidate-vm` boots a local NixOS VM with a failed admission fixture and `serve: false`. It proves candidate failure is status, not activation: instance/admission/status are published, but no active selection or served symlinks are created.

`gitops-failed-served-candidate-refusal` proves a desired state cannot request serving for a candidate whose admission fixture failed.

`gitops-local-apply-without-optin-refusal` proves `local-apply` desired state is refused unless the operator sets `FISHYSTUFF_GITOPS_ENABLE_LOCAL_APPLY=1`. This keeps the still-scaffolded host-local mode from mutating a machine because a fixture or operator file used the wrong mode.

`gitops-missing-active-artifact-refusal` proves graph-side serving checks require the active release to name the API, Dolt service, site, and CDN artifact paths even when desired state is hand-written.

`gitops-missing-retained-artifact-refusal` proves retained rollback releases must also name the rollback-critical API, Dolt service, site, and CDN artifact paths before anything can be served.

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
        "mode": "read_only",
        "materialization": "metadata_only",
        "remote_url": "",
        "cache_dir": "",
        "release_ref": ""
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
      "serve": false,
      "admission_fixture_state": ""
    }
  }
}
```

Supported modes:

- `validate`: decode, shape, and unify only. It does not write local state and does not run admission.
- `vm-test`: create only VM-local files under `/var/lib/fishystuff/gitops-test` and `/run/fishystuff/gitops-test`.
- `vm-test-closures`: VM-only mode that also verifies real Nix store paths with `nix:closure` and roots them under `/var/lib/fishystuff/gitops-test/gcroots`.
- `local-apply`: reserved for future host-local activation. It is refused unless `FISHYSTUFF_GITOPS_ENABLE_LOCAL_APPLY=1` is set, and the first milestone does not include fixtures that use it.

`admission_fixture_state` is a VM-only test hook for deterministic local admission behavior. It may be empty, `passed_fixture`, `failed_fixture`, or `not_run`; empty defaults to `passed_fixture` in VM modes and `not_run` in validate mode. It must not be used for beta/prod desired state.

`main.mcl` traverses the desired-state `environments` map generically. Every enabled environment must use the `single_active` strategy, name an enabled host, and select a release by key. The checked-in fixtures still use readable names such as `example-release`, while the generated beta validation package derives a different release key from exact inputs to prove the graph is not hardcoded to the fixture name. This milestone supports generic single-host environments; richer placement strategies should be new modules with their own VM tests.

## Dolt Materialization

The Dolt desired-state fields separate data identity from transport. `dolt.commit` is the exact data identity that may be served. `dolt.branch_context` is only the branch/ref context to fetch from. `dolt.materialization` controls how the host gets that commit locally:

- `metadata_only`: record the exact Dolt identity but do not realize data locally. This is the default for validation-only fixtures.
- `fetch_pin`: maintain a persistent host-local Dolt cache, fetch the requested branch from `remote_url`, and force `release_ref` to the exact `commit`. VM tests implement this through the `fishystuff_deploy dolt fetch-pin` helper against a local file remote only.
- `replica_pin`: reserved for a future read-replica cache that still pins and verifies the exact release commit before serving.
- `snapshot`: reserved for bootstrap or disaster recovery. It should not be the normal deploy path because shipping a `.dolt` snapshot in a Nix closure repeats the large database transfer.

`fetch_pin` is the intended normal deployment path. It avoids full clones on every deploy: expensive Dolt transfer happens as incremental fetch into a cache under `cache_dir`, while activation can only proceed after `release_ref` verifies to the exact desired commit. DoltHub may remain a source/public mirror, but production deployment should fetch from a faster FishyStuff-controlled remote or mirror.

The Rust deployment helper is packaged as `.#fishystuff-deploy`. It is intentionally a narrow host-local helper, not a plan/apply deployment command: mgmt still owns desired-state reconciliation, while the helper only executes Dolt clone/fetch/ref-pin/status-file operations requested by the graph.

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

All release objects are checked as catalog entries, but artifact realization is intentionally narrower: `nix:closure` and `nix:gcroot` are emitted only for releases selected by enabled environments as `active_release` or listed in `retained_releases`. This lets desired state carry preview, future, or stale release metadata without trying to root unused artifacts.

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

Dolt cache state is not itself a release artifact. A release artifact names the exact `dolt.commit`; the selected host materializes that commit through `fetch_pin`, `replica_pin`, or a bootstrap snapshot before admission can claim the tuple is ready.

The `cdn_runtime` closure is expected to be the CDN serving root that Caddy can point at directly. For real deployments this should be built from the current CDN content plus retained immutable assets from prior CDN roots, for example with `.#cdn-serving-root` or an equivalent derivation constructed from exact store paths. The `cdn-serving-root` derivation validates the current root's runtime manifest when present and refuses a root whose selected JS/WASM files are missing. The GitOps graph should receive that final store path as desired state; it should not infer prior roots from a mutable remote host during activation. Serving admission requires this root to include `cdn-serving-manifest.json`, which records the current root and retained roots. When the desired environment retains rollback releases, serving admission checks that the active CDN serving manifest accounts for the CDN root required by each retained release. If a retained release's `cdn_runtime` is itself a serving root, admission checks its recorded `current_root`; otherwise it checks the retained `cdn_runtime` path directly.

`retained_releases` on an environment records the releases intentionally kept hot for rollback and for stale client HTML/runtime references. Each retained ID must reference a release object in desired state, and serving requires at least one retained rollback release. Activation records this list in the local active/status documents so operators can tell which rollback set was selected with the active release.

For serving desired state, both the active release and each retained rollback release must include non-empty `store_path` values for `api`, `dolt_service`, `site`, and `cdn_runtime`. In plain `vm-test` mode these paths are not realized/rooted, but they still make the exact deployment tuple explicit. `vm-test-closures` and future local/production modes can add realization and gcroot guarantees on top of the same tuple.

Source maps are public in production because the project is open source. They are emitted with content-hashed filenames and retained as immutable CDN assets, but generated HTML/runtime manifests do not eagerly reference them, so normal users do not fetch them.

## Safety Defaults

This graph does not import Hetzner, Cloudflare, or SSH providers. It does not call deploy scripts. It does not start FishyStuff system services. The VM fixture disables closure realization, so it never tries to realize fake `/nix/store` paths.

`gitops/modules/fishy/nix.mcl` emits `nix:closure` and `nix:gcroot` only in `vm-test-closures` and future `local-apply` mode. In `validate` and plain `vm-test`, enabled artifacts are validation no-ops. The flake checks and `gitops-unify` default to the pinned local `~/code/mgmt-fishystuff-beta/` commit recorded in `flake.lock`/`scripts/recipes/gitops-unify.sh` because it contains the integrated Nix primitives needed to type-check this graph.

The VM runtime tests bind mgmt's embedded etcd to `127.0.0.1` inside the test VM and set `virtualisation.memorySize = 2048` for enough headroom with the pinned mgmt build. They do not connect to beta, production, Hetzner, Cloudflare, SSH, or operator SecretSpec profiles.

`gitops-closure-roots-vm` generates desired state from tiny real Nix store artifacts inside the test derivation. It proves closure verification and gcroot creation without using fake enabled store paths or serving anything.

`gitops-multi-environment-candidates-vm` boots one local NixOS VM with two enabled preview-like single-host environments. It proves `main.mcl` traverses arbitrary enabled environment keys, publishes separate candidate/admission/status files for each, and does not create served state when both environments are non-serving candidates.

`gitops-multi-environment-served-vm` boots one local NixOS VM with two served preview-like environments on the same host. It proves active symlinks are environment-scoped under `/var/lib/fishystuff/gitops-test/served/<environment>/{site,cdn}` so one served preview cannot overwrite another preview's selected site/CDN tuple.

`gitops-served-closure-roots-vm` combines the served candidate shape with `vm-test-closures`. It verifies and roots active and retained rollback API, Dolt service, site, and CDN artifacts under `/var/lib/fishystuff/gitops-test/gcroots`, then checks the VM-local active symlinks and route selection. It still does not write `/srv/fishystuff` or start real FishyStuff services.

The closure and gcroot resources are both declared for each enabled artifact. A strict `nix:closure -> nix:gcroot` resource edge is intentionally deferred: the pinned mgmt build verified closures but did not progress the dependent gcroot behind that edge in the VM test. Reintroduce that edge only with a VM regression test proving the ordered behavior.

`gitops-served-candidate-vm` keeps activation local and synthetic. When desired state requests `serve: true` in `vm-test` mode, fixture admission must be `passed_fixture`; the local admission fixture also reads the selected site root, CDN runtime manifest, runtime JS/WASM files, and CDN serving manifest from the exact store paths in the release tuple. The graph then writes an active selection document under `/var/lib/fishystuff/gitops-test/active/<environment>.json`, VM-local served symlinks under `/var/lib/fishystuff/gitops-test/served/<environment>/{site,cdn}`, and a route selection document under `/run/fishystuff/gitops-test/routes/<environment>.json`. This is the first safe shape of the future route/symlink switch. It does not start FishyStuff services, write `/srv/fishystuff`, or touch real beta/prod state.

`gitops-served-caddy-handoff-vm` adds a real local Caddy consumer for that handoff shape. Caddy serves the stable symlink roots while mgmt reconciles the underlying active release. The test verifies site content, current CDN runtime files, and retained prior CDN runtime files over HTTP before and after the selected release changes.

`gitops-dolt-fetch-pin-vm` keeps Dolt transfer local and synthetic. It uses the `fishystuff_deploy dolt fetch-pin` helper, backed by Dolt's own `clone`, `fetch`, and local branch pinning against a file remote inside the VM, to prove the GitOps graph can express "exact commit present locally" without sending a full `.dolt` closure per release or contacting DoltHub.

`gitops-dolt-admission-pin-vm` keeps admission local and synthetic while making it DB-backed. The optional VM-only `admission_probe.kind = "dolt_sql_fixture"` path writes a probe request, waits for `fetch_pin`, verifies the pinned materialization status, and executes a one-row/one-column Dolt SQL query through `fishystuff_deploy dolt probe-sql-fixture` before writing the admission document.

Fallbacks introduced: none to the old beta deployment graph. The validation no-op is a mode-specific safety guard, not compatibility with an old code path.

## Admission

Admission is modeled separately from graph acceptance. In `validate`, admission is `not_run` and must not be treated as success. In `vm-test`, admission is a deterministic local fixture written under `/run/fishystuff/gitops-test/admission/`; by default it is `passed_fixture`, and tests may explicitly request `failed_fixture` through `admission_fixture_state`. A VM test environment may also request `admission_probe.kind = "dolt_sql_fixture"` to run a configured single-scalar SQL probe against the exact pinned Dolt cache/ref before admission is published. For serving fixtures, this local probe must be able to read the selected `site/index.html`, `cdn_runtime/map/runtime-manifest.json`, the selected runtime JS/WASM files, and `cdn_runtime/cdn-serving-manifest.json`. The serving manifest must also account for `runtime-manifest.json` and the selected runtime JS/WASM asset paths.

Future real admission should probe the exact candidate tuple:

- API `readyz`
- API `/api/v1/meta`
- A representative DB-backed API route that would catch schema/data mismatches such as the previous `languagedata` versus `languagedata_en` issue
- Branch-qualified Dolt behavior when branch context matters
- The candidate API is connected to the exact pinned Dolt commit/ref, not merely the current branch tip
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
- `dolt_commit`
- `dolt_materialization`
- `dolt_cache_dir`
- `dolt_release_ref`
- `served`
- `failure_reason`

Local active selection is written only when a VM/local desired state is explicitly serving:

```text
/var/lib/fishystuff/gitops-test/active/<environment>.json
/var/lib/fishystuff/gitops-test/served/<environment>/site
/var/lib/fishystuff/gitops-test/served/<environment>/cdn
/run/fishystuff/gitops-test/routes/<environment>.json
```

The active selection document includes the desired generation that selected the served symlinks so route state can be correlated with the desired-state object that produced it.

The route selection document is the local-only handoff shape for future Caddy integration. It records the selected release, the active selection document path, and the stable site/CDN symlink roots that Caddy would serve, without starting or reloading Caddy in VM tests. The route document is declared after the active selection so a future file-watching edge does not observe a selected route before the active symlinks and active JSON exist.

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
- Dolt snapshot materialization preserving wrong ownership/mode: the tested path pins an exact commit through a host-local Dolt cache; snapshot mode remains documented but not implemented.
- Diagnostic/manual processes conflicting with managed services: the first graph does not start real services.
