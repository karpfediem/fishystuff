# FishyStuff GitOps Mgmt Substrate

This directory is a clean-slate mgmt module repository for the next FishyStuff deployment substrate. It intentionally does not extend the old beta MCL graph under `mgmt/`.

See `PRODUCTION.md` for the current production handoff boundary and the checks that separate today's live production setup from the future GitOps serving path.

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

The flake checks added by this milestone are:

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
nix build .#checks.x86_64-linux.gitops-desired-state-local-apply-rollback
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

`.#gitops-desired-state-beta-validate` emits a validation-only desired-state JSON file from exact Nix build outputs: API bundle, Dolt service bundle, and site content. It deliberately keeps `cdn_runtime` disabled so normal repo checks do not depend on private or ignored CDN staging state. Its release key is derived from the exact available tuple by default. It sets `serve: false`, `mode: validate`, and a placeholder Dolt commit; it is not a deploy/apply command.

`.#gitops-desired-state-production-validate` is the production-shaped equivalent. It uses production API/Dolt service bundles, production site content, `dolt.branch_context = "main"`, `serve: false`, and `mode: validate`, so it can type-check production inputs without mutating production or selecting a served release.

`just gitops-production-current-desired` writes a local ignored desired-state snapshot to `data/gitops/production-current.desired.json`. It uses the local Dolt `main` commit, production API/Dolt service bundles, production site content, and the finalized CDN serving root. The snapshot is still `mode: validate` and `serve: false`; it is an operator handoff artifact for inspecting the exact current tuple, not a serving request and not a deployment command. Retained rollback releases can be supplied with `FISHYSTUFF_GITOPS_RETAINED_RELEASES_JSON` or `FISHYSTUFF_GITOPS_RETAINED_RELEASES_FILE`; the recipe requires exact release IDs, Dolt commits, and all four closure paths for each retained release instead of inventing a previous target.

`just gitops-production-current-handoff` is the checked version of that flow. It requires retained rollback input, generates the production-current desired file, runs `gitops-check-desired-serving`, verifies that every active and retained closure path exists locally, verifies that the active CDN serving manifest retains each rollback CDN root, runs `gitops-unify` against the same file, and verifies the written handoff summary before printing the ready marker. It is still local-only and does not serve production.

`.#gitops-desired-state-production-vm-serve-fixture` is a production-shaped VM fixture, not a production deployment. It uses production API/Dolt service bundles and production site content, but keeps `mode: vm-test` and uses fixture CDN serving roots. `gitops-desired-state-production-serve-shape-refusal` proves production-shaped serving desired state is refused when rollback retention or the CDN runtime closure is missing.

`.#gitops-desired-state-production-rollback-transition-fixture` is the production-shaped rollback companion. It serves `previous-production-release`, retains the exact candidate release ID derived from the production serve fixture, uses `dolt.branch_context = "main"` for both releases, and keeps the rolled-away candidate CDN root retained for stale clients.

`.#gitops-desired-state-production-api-meta-fixture` is the production-shaped API admission companion. It is `local-apply`, targets a loopback API upstream, asks mgmt to manage only the isolated candidate API service name, uses `fetch_pin` Dolt materialization, and requires `/api/v1/meta` to report the exact release ID, release identity, and Dolt commit before served state can publish.

Real deployment desired state should import `nix/packages/gitops-desired-state.nix` from an operator/deployment flake and pass exact `doltCommit`, service bundles, site content, and finalized CDN serving roots as Nix values. The generated GitOps validation package intentionally does not use ambient environment variables for those deployment-critical inputs.

`.#gitops-desired-state-vm-serve-fixture` emits a local `vm-test` desired-state file with tiny store artifacts for API, Dolt service, site, a finalized CDN serving root, and one retained previous release object. The package generator refuses `serve: true` unless all four active release artifacts are present.

`gitops-desired-state-admission-probe` proves the generated desired-state helper can emit `admission_probe.kind = "dolt_sql_scalar"` for a VM-only `fetch_pin` candidate and still unify through `gitops/main.mcl`. It does not run the probe or contact a remote.

`gitops-desired-state-http-admission-probe` proves the generated desired-state helper can emit a `local-apply` loopback API meta admission probe without requiring Dolt `fetch_pin` materialization.

`gitops-desired-state-local-apply-rollback` proves the generated desired-state helper can emit a serving `local-apply` rollback transition with loopback API meta admission and a managed candidate API service.

`gitops-desired-state-rollback-transition` proves the generated desired-state helper can emit an explicit rollback transition with the rolled-away release retained for rollback and stale CDN clients.

`gitops-desired-state-rollback-transition-retention-refusal` proves the generated desired-state helper refuses a rollback transition when `transition.from_release` is not retained.

`gitops-desired-state-serve-without-retained-refusal` proves the generated desired-state helper refuses `serve: true` without at least one retained rollback release.

`gitops-desired-state-active-retained-refusal` proves the generated desired-state helper refuses a retained rollback set that includes the active release.

`gitops-desired-state-transition-shape-refusal` proves the generated desired-state helper refuses contradictory explicit transition intent, such as a `candidate` transition with `serve: true`, an `activate` transition with `serve: false`, or `from_release` on a non-rollback transition.

`gitops-dolt-fetch-pin-vm` boots one local NixOS VM, creates a local file-backed Dolt remote, and reconciles a `fetch_pin` desired state against it. The test first pins commit 1 in a persistent VM-local cache, then pushes commit 2 to the same local remote and changes desired state. It verifies the existing cache is fetched forward, the release ref points at the exact desired commit, and no `.dolt` snapshot/full closure path is used.

`gitops-dolt-admission-pin-vm` adds a local DB-backed admission step to the `fetch_pin` path. Desired state includes `admission_probe.kind = "dolt_sql_scalar"` with a single-scalar SQL query and expected value. The graph runs the probe only after the Dolt materialization helper has pinned the exact release ref, and the helper refuses admission if the materialization status, ref hash, or query result does not match the desired commit tuple.

`gitops-served-retained-dolt-fetch-pin-vm` proves the rollback data side of serving. It creates a local Dolt remote, serves a candidate release, retains multiple rollback releases, and verifies the active and all retained rollback release refs are pinned in the same VM-local Dolt cache before served active/status/route documents publish. The rollback readiness document still records the first retained release as the primary rollback target, and rollback-set member documents record every retained release's exact Dolt status path.

`gitops-json-status-escaping-vm` proves the VM-local JSON outputs preserve quote/backslash characters from the exact release identity tuple instead of emitting malformed JSON.

`gitops-unused-release-closure-noop-vm` boots a local NixOS VM in `vm-test-closures` mode with one selected release backed by real tiny store artifacts and one unselected release backed by bogus store paths. It proves the graph validates the release catalog but only realizes and roots releases requested by enabled environments as active or retained rollback releases.

`gitops-generated-served-candidate-vm` boots a local NixOS VM with that generated desired state. It verifies the graph can express a served candidate from generated JSON, checks the selected site/CDN runtime fixture, verifies the generated retained `previous-release` object, writes the VM-local route selection document, and confirms vm-test mode does not create real FishyStuff service state or gcroots.

`gitops-production-vm-serve-fixture-vm` boots a local NixOS VM with the generated `.#gitops-desired-state-production-vm-serve-fixture` JSON. It uses production API/Dolt service bundles and production site content with fixture CDN serving roots, checks served state under the VM-local `production` environment, and asserts `/var/lib/fishystuff/gitops`, `/srv/fishystuff`, and real FishyStuff services remain untouched.

`gitops-production-rollback-transition-vm` boots a local NixOS VM with the generated production rollback desired state. It checks the VM-local `production` served state after rollback, verifies the candidate release remains retained as the primary rollback target, and runs the read-only served-state helper against `environment=production`.

`gitops-production-api-meta-vm` boots a local NixOS VM with the generated production `local-apply` API-meta desired-state shape, then writes an exact runtime desired state from a local file-backed Dolt remote. It pins active and retained rollback commits into `/var/lib/fishystuff/gitops/dolt-cache`, starts an isolated loopback candidate API service against the pinned active release ref, checks `/api/v1/meta` for the exact release identity and Dolt commit, and verifies the served state remains local to the VM.

`gitops-served-symlink-transition-vm` boots one local NixOS VM, serves one desired state, then serves a second desired state. It proves the VM-local active symlinks and route selection document move by reconciliation through desired state, not by an imperative deployment command.

`gitops-served-caddy-handoff-vm` boots one local NixOS VM, runs Caddy against the VM-local served site/CDN symlink roots, and then changes the served desired state. It proves the future Caddy-facing handoff shape can serve the selected release and then observe the next selected release through stable symlink roots without restarting Caddy.

`gitops-served-caddy-rollback-transition-vm` boots one local NixOS VM, runs Caddy against the same stable served site/CDN roots, serves a candidate, then rolls back to the previous release. It proves the Caddy-facing handoff observes the rollback target over HTTP while the rolled-away candidate CDN root remains retained for stale clients.

`gitops-served-rollback-transition-vm` boots one local NixOS VM, serves a candidate, then rolls back to the previous release by changing desired state with `transition.kind = "rollback"`. It proves rollback is represented as another reconciled active-release transition while retaining the candidate CDN root for stale clients and updating the active/status transition fields, route selection, primary rollback readiness, and rollback-set documents.

`gitops-failed-candidate-vm` boots a local NixOS VM with a failed admission fixture and `serve: false`. It proves candidate failure is status, not activation: instance/admission/status are published, but no active selection or served symlinks are created.

`gitops-failed-served-candidate-refusal` proves a desired state cannot request serving for a candidate whose admission fixture failed.

`gitops-local-apply-without-optin-refusal` proves `local-apply` desired state is refused unless the operator sets `FISHYSTUFF_GITOPS_ENABLE_LOCAL_APPLY=1`. This keeps the still-scaffolded host-local mode from mutating a machine because a fixture or operator file used the wrong mode.

`gitops-local-apply-candidate-vm` boots one local NixOS VM with `FISHYSTUFF_GITOPS_ENABLE_LOCAL_APPLY=1` and a non-serving `local-apply` candidate. It proves local-apply writes candidate/status facts under `/var/lib/fishystuff/gitops` and `/run/fishystuff/gitops`, not the VM-test directories, while still avoiding `/srv/fishystuff` and real service mutation.

`gitops-local-apply-fetch-pin-vm` boots one local NixOS VM with `FISHYSTUFF_GITOPS_ENABLE_LOCAL_APPLY=1`, creates a local file-backed Dolt remote, and proves `fetch_pin` uses `/var/lib/fishystuff/gitops/dolt-cache` plus `/run/fishystuff/gitops/dolt` in local-apply mode. It fetches a second commit into the same cache to prove updates do not reclone from scratch.

`gitops-local-apply-http-admission-vm` boots one local NixOS VM with `FISHYSTUFF_GITOPS_ENABLE_LOCAL_APPLY=1`, defines a loopback-only candidate API service backed by a tiny VM-local Dolt SQL fixture, and serves a local-apply candidate only after mgmt starts the `api_service` and `fishystuff_deploy http probe-json-scalars` verifies the real `fishystuff_server` `/api/v1/meta` reports the exact release ID, release identity, Dolt commit, and fixture-backed meta rows under the declared `api_upstream`. The candidate service reads the GitOps-written `/var/lib/fishystuff/gitops/api/<environment>.env` `EnvironmentFile`. The test switches desired state to a second release, then expresses rollback back to the retained first release, reusing the same service name and verifying mgmt restarts the candidate API before admission can pass for each new identity. It verifies status, active symlinks, route selection, rollback transition fields, rollback-set state, candidate API config, and admission request/status files under `/var/lib/fishystuff/gitops` and `/run/fishystuff/gitops`, while still avoiding VM-test paths, `/srv/fishystuff`, and real FishyStuff services.

`gitops-missing-active-artifact-refusal` proves graph-side serving checks require the active release to name the API, Dolt service, site, and CDN artifact paths even when desired state is hand-written.

`gitops-missing-retained-artifact-refusal` proves retained rollback releases must also name the rollback-critical API, Dolt service, site, and CDN artifact paths before anything can be served.

`gitops-missing-retained-release-refusal` proves retained rollback release IDs are not informational labels: each retained ID must reference a release object before candidate/admission/status/active state can be published.

`gitops-no-retained-release-refusal` proves serving is refused when no rollback release is retained.

`gitops-active-retained-release-refusal` proves a hand-written desired-state file that lists the active release as retained cannot publish candidate, status, active, route, or rollback state.

`gitops-rollback-transition-retention-refusal` proves a rollback transition cannot publish state unless the release being rolled away from remains retained after rollback.

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
          "gcroot_path": "/nix/var/nix/gcroots/fishystuff/gitops/example-release/api"
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
      "api_upstream": "",
      "api_service": "",
      "admission_fixture_state": "",
      "admission_probe": {
        "kind": "",
        "probe_name": "",
        "url": "",
        "expected_status": 0,
        "timeout_ms": 0,
        "query": "",
        "expected_scalar": "",
        "json_pointer": ""
      },
      "transition": {
        "kind": "",
        "from_release": "",
        "reason": ""
      }
    }
  }
}
```

Supported modes:

- `validate`: decode, shape, and unify only. It does not write local state and does not run admission.
- `vm-test`: create only VM-local files under `/var/lib/fishystuff/gitops-test` and `/run/fishystuff/gitops-test`.
- `vm-test-closures`: VM-only mode that also verifies real Nix store paths with `nix:closure` and roots them under `/nix/var/nix/gcroots/fishystuff/gitops-test`.
- `local-apply`: opt-in host-local mode. It is refused unless `FISHYSTUFF_GITOPS_ENABLE_LOCAL_APPLY=1` is set. Candidate/status facts, served symlinks, rollback documents, and route handoff files use `/var/lib/fishystuff/gitops` and `/run/fishystuff/gitops`. Serving with HTTP admission requires `api_upstream`, and the admission URL must target that upstream. Current probe support is loopback-only.

`admission_fixture_state` is a VM-only test hook for deterministic local admission behavior. It may be empty, `passed_fixture`, `failed_fixture`, or `not_run`; empty defaults to `passed_fixture` in VM modes and `not_run` in validate mode. It must not be used for beta/prod desired state.

`transition` is optional audit intent for the selected environment. Empty kind defaults to `candidate` when `serve: false` and `activate` when `serve: true`. `rollback` is accepted only when `serve: true`, `active_release` names the rollback target, and `from_release` is retained after rollback. Active/status documents record the transition kind and rollback fields so a rollback is visible as an intentional reconciled state transition instead of an ambiguous active-release edit.

The generated desired-state helper supports the same transition object and refuses to emit rollback desired state unless `transition.from_release` remains in the retained rollback set.

`api_upstream` is the candidate API endpoint selected for a served environment. For the current local-only HTTP admission bridge it should be a loopback HTTP origin such as `http://127.0.0.1:18082` without a trailing slash. When set, HTTP admission probe URLs must equal that upstream or live below it; the generated desired-state helper enforces the same relationship.

`api_service` is optional and local-writing-mode only. When set, it must be a mgmt `svc` name such as `fishystuff-gitops-candidate-api-local-test`, not a systemd unit filename ending in `.service`. The graph writes `/var/lib/fishystuff/gitops/api/<environment>.json` plus `/var/lib/fishystuff/gitops/api/<environment>.env`, starts that candidate service through `svc`, and only then runs HTTP admission. The env file contains `FISHYSTUFF_RELEASE_ID`, `FISHYSTUFF_RELEASE_IDENTITY`, `FISHYSTUFF_DOLT_COMMIT`, and `FISHYSTUFF_DEPLOYMENT_ENVIRONMENT`. The env file notifies the service, and candidate services use `refresh_action => "try-restart"` so a changed env file restarts a running service before the next HTTP admission. The current VM test consumes this env file through an isolated real `fishystuff_server` service backed by a local Dolt SQL fixture. Real FishyStuff service names are still not started by default.

`main.mcl` traverses the desired-state `environments` map generically. Every enabled environment must use the `single_active` strategy, name an enabled host, and select a release by key. The checked-in fixtures still use readable names such as `example-release`, while the generated beta validation package derives a different release key from exact inputs to prove the graph is not hardcoded to the fixture name. This milestone supports generic single-host environments; richer placement strategies should be new modules with their own VM tests.

## Dolt Materialization

The Dolt desired-state fields separate data identity from transport. `dolt.commit` is the exact data identity that may be served. `dolt.branch_context` is only the branch/ref context to fetch from. `dolt.materialization` controls how the host gets that commit locally:

- `metadata_only`: record the exact Dolt identity but do not realize data locally. This is the default for validation-only fixtures.
- `fetch_pin`: maintain a persistent host-local Dolt cache, fetch the requested branch from `remote_url`, and force `release_ref` to the exact `commit`. VM tests implement this through the `fishystuff_deploy dolt fetch-pin` helper against local file remotes.
- `replica_pin`: reserved for a future read-replica cache that still pins and verifies the exact release commit before serving.
- `snapshot`: reserved for bootstrap or disaster recovery. It should not be the normal deploy path because shipping a `.dolt` snapshot in a Nix closure repeats the large database transfer.

The current GitOps graph requires `dolt.mode = "read_only"` for every release. Mutable Dolt workflows should stay in ingestion/admin tooling, not in the serving deployment substrate.
`replica_pin` and `snapshot` are accepted only in `validate` mode until their materialization behavior is implemented with local tests.
Dolt remote URLs must not embed credentials because desired state and status documents are not secret stores.

`fetch_pin` is the intended normal deployment path. It avoids full clones on every deploy: expensive Dolt transfer happens as incremental fetch into a cache under `cache_dir`, while activation can only proceed after `release_ref` verifies to the exact desired commit. The helper reconciles the persistent cache's `origin` remote to the desired `remote_url` before fetching, so switching from DoltHub to a faster FishyStuff-controlled mirror is an explicit desired-state change instead of stale local cache configuration. The helper also holds a Unix advisory lock beside the cache while mutating it, so active and retained release refs that share a host-local cache serialize at the tool boundary. DoltHub may remain a source/public mirror, but production deployment should fetch from a faster FishyStuff-controlled remote or mirror.

The Rust deployment helper is packaged as `.#fishystuff-deploy`. It is intentionally a narrow host-local helper, not a plan/apply deployment command: mgmt still owns desired-state reconciliation, while the helper only executes Dolt clone/fetch/ref-pin/status-file operations requested by the graph. The same helper owns the `needs-*` freshness checks used by mgmt `exec.ifcmd`, so rerun decisions compare the structured request/status tuple instead of partial shell/JQ snippets.

The helper also provides a read-only local status validator:

```bash
fishystuff_deploy gitops check-served \
  --status /var/lib/fishystuff/gitops/status/local-test.json \
  --active /var/lib/fishystuff/gitops/active/local-test.json \
  --rollback-set /var/lib/fishystuff/gitops/rollback-set/local-test.json \
  --rollback /var/lib/fishystuff/gitops/rollback/local-test.json \
  --environment local-test
```

This only reads local GitOps documents. It verifies status, active selection, rollback-set, and primary rollback readiness documents agree on the served generation/release and that rollback readiness is available with at least one retained release.

The same local check is available through:

```bash
just gitops-check-served environment=local-test state_dir=/var/lib/fishystuff/gitops
```

Before treating a desired-state snapshot as a serving candidate, run the desired-state preflight:

```bash
just gitops-check-desired-serving \
  state_file=data/gitops/production-current.desired.json \
  environment=production
```

This is read-only. It accepts validate-mode snapshots, but it still requires the selected environment to have an enabled host, an active release, at least one retained rollback release, no active/retained overlap, exact API/site/CDN/Dolt-service closure store paths for the active and retained releases, read-only Dolt identity, and `fetch_pin` materialization for production-shaped releases.

For a human-readable summary of the served release, primary rollback identity, and retained rollback releases:

```bash
just gitops-served-summary environment=local-test state_dir=/var/lib/fishystuff/gitops
```

For the stricter operator handoff view, include the local run-state documents as well:

```bash
just gitops-inspect-served \
  environment=local-test \
  state_dir=/var/lib/fishystuff/gitops \
  run_dir=/run/fishystuff/gitops
```

This is still read-only. It first runs the served-state consistency check, then verifies the admission status, route selection, and `roots-ready` files under `run_dir` agree with the exact served release and every retained rollback release.

To generate a local production-current desired-state snapshot:

```bash
just gitops-production-current-desired
just gitops-check-desired-serving state_file=data/gitops/production-current.desired.json environment=production
just gitops-unify auto data/gitops/production-current.desired.json
```

The recipe is local-only. It builds or reuses the production service/site/CDN outputs, reads the local Dolt `main` hash, and writes an ignored file under `data/gitops/`. It supports environment overrides such as `FISHYSTUFF_GITOPS_DOLT_COMMIT`, `FISHYSTUFF_GITOPS_GIT_REV`, and `FISHYSTUFF_GITOPS_*_CLOSURE` for exact operator-controlled snapshots or fast tests.

Retained rollback releases use this JSON shape:

```json
[
  {
    "release_id": "previous-production-release",
    "generation": 1,
    "git_rev": "exact-previous-git-rev",
    "dolt_commit": "exact-previous-dolt-commit",
    "api_closure": "/nix/store/...-api",
    "site_closure": "/nix/store/...-site",
    "cdn_runtime_closure": "/nix/store/...-cdn",
    "dolt_service_closure": "/nix/store/...-dolt-service"
  }
]
```

Optional retained fields are `dolt_materialization`, `dolt_remote_url`, `dolt_cache_dir`, and `dolt_release_ref`. The active release cannot appear in the retained set, and retained IDs must be unique.

If rollback-set member documents already exist for the retained releases, derive this JSON from those documents instead of hand-typing the tuple:

```bash
fishystuff_deploy gitops retained-releases-json \
  --rollback-set /var/lib/fishystuff/gitops/rollback-set/production.json \
  > /tmp/fishystuff-retained-releases.json

FISHYSTUFF_GITOPS_RETAINED_RELEASES_FILE=/tmp/fishystuff-retained-releases.json \
  just gitops-production-current-handoff
```

The equivalent recipe can read the rollback-set index and pass every member document automatically:

```bash
just gitops-retained-releases-json \
  environment=production \
  state_dir=/var/lib/fishystuff/gitops \
  > /tmp/fishystuff-retained-releases.json
```

The helper is read-only. It can also accept repeated `--rollback-member` arguments for explicitly selected member documents. In both modes it requires each rollback-set member identity to match the member's release ID, generation, Git revision, Dolt commit, and API/site/CDN/Dolt-service paths exactly before emitting production-current input.

For the normal future cycle, derive retained input from the served rollback-set and run the checked handoff in one step:

```bash
just gitops-production-current-from-served state_dir=/var/lib/fishystuff/gitops
```

This writes `production-current.retained-releases.json`, `production-current.desired.json`, and `production-current.handoff-summary.json` under `data/gitops/` by default. The summary records `desired_state_sha256` for the exact checked JSON file and includes `cdn_retention`, which records the active CDN serving root, its underlying `current_root`, retained roots, and the expected retained CDN root for each rollback release. It is still local-only; it reads served GitOps documents and writes operator artifacts, but does not mutate hosts.

Before using a handoff summary as input to any later activation work, verify it is still attached to the exact desired-state file and active CDN manifest it records:

```bash
just gitops-check-handoff-summary
```

The next local-only bridge is `just gitops-production-activation-draft`. It reads the verified handoff summary plus an explicit admission evidence JSON file and writes `data/gitops/production-activation.draft.desired.json` by default. The draft changes the selected environment to `mode: local-apply`, `serve: true`, `transition.kind: activate`, and `admission_probe.kind: api_meta`; it does not run mgmt apply or mutate production. Admission evidence must use schema `fishystuff.gitops.activation-admission.v1` and match the exact handoff summary SHA-256, desired-state SHA-256, release ID, release identity, Dolt commit, API meta response, a representative DB-backed probe, and a site/CDN runtime probe.

Backup/restore and replication are separate transport classes:

- Dolt backups are appropriate for bootstrap and disaster recovery, not routine deployment. They move a point-in-time repository state, which is still heavier and more disruptive than fetching into an already-warm cache and pinning a release ref.
- Dolt SQL-server replication should be modeled as a future `replica_pin` materialization. It must still publish the exact commit/ref that admission will query, and it needs its own local test before any real host uses it.
- The old service-style `DOLT_FETCH()` plus branch-tip `DOLT_RESET()` refresh shape is not precise enough for serving by itself. GitOps serving needs an exact commit verification gate before admission can pass.

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

All release objects are checked as catalog entries, but artifact realization is intentionally narrower: `nix:closure` verification and managed Nix GC-root symlinks are emitted only for releases selected by enabled environments as `active_release` or listed in `retained_releases`. This lets desired state carry preview, future, or stale release metadata without trying to root unused artifacts.

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

`retained_releases` on an environment records the releases intentionally kept hot for rollback and for stale client HTML/runtime references. Each retained ID must reference a release object in desired state, retained IDs must be unique, and the active release must not be listed as retained. Serving requires at least one retained rollback release. Activation records this list and the retained Dolt materialization status paths in the local active/status documents so operators can tell which rollback set was selected with the active release and where each retained pin was verified.

The rollback set is also published locally as an index document at `/var/lib/fishystuff/gitops-test/rollback-set/<environment>.json` in VM modes, plus one retained-member document under `/var/lib/fishystuff/gitops-test/rollback-set/<environment>/<release-id>.json`. The index records the selected retained release IDs and member document paths. Each member document records the retained release's exact API, Dolt service, site, CDN runtime, Dolt commit, Dolt materialization/cache/ref tuple, and Dolt status path when `fetch_pin` is used.

For serving desired state, both the active release and each retained rollback release must include non-empty `store_path` values for `api`, `dolt_service`, `site`, and `cdn_runtime`. In plain `vm-test` mode these paths are not realized/rooted, but they still make the exact deployment tuple explicit. `vm-test-closures` and future local/production modes add realization and GC-root guarantees on top of the same tuple. Serving modes that manage closure roots also require those active and retained artifacts to be explicitly enabled.

Source maps are public in production because the project is open source. They are emitted with content-hashed filenames and retained as immutable CDN assets, but generated HTML/runtime manifests do not eagerly reference them, so normal users do not fetch them.

## Safety Defaults

This graph does not import Hetzner, Cloudflare, or SSH providers. It does not call deploy scripts. It does not start FishyStuff system services. The VM fixture disables closure realization, so it never tries to realize fake `/nix/store` paths.

`gitops/modules/fishy/nix.mcl` emits `nix:closure` verification and file-managed GC-root symlinks only in `vm-test-closures` and future `local-apply` mode. The symlinks live under `/nix/var/nix/gcroots/fishystuff/gitops-test/<release>/...` in closure VM tests and `/nix/var/nix/gcroots/fishystuff/gitops/<release>/...` in local-apply mode, so they are real Nix GC roots rather than ordinary state-file links. In `validate` and plain `vm-test`, enabled artifacts are validation no-ops. The flake checks and `gitops-unify` default to the pinned local `~/code/mgmt-fishystuff-beta/` commit recorded in `flake.lock`/`scripts/recipes/gitops-unify.sh` because it contains the integrated Nix closure primitive needed to type-check this graph.

The GitOps flake checks apply `nix/patches/mgmt-recwatch-bound-watch-path-index.patch` to that pinned mgmt package. The patch is a small local backport from the adjacent mgmt checkout that prevents a recursive watcher index panic while the graph creates nested local state directories. The MCL graph also creates explicit parent directories and avoids recursive directory management for GitOps status trees.

The VM runtime tests bind mgmt's embedded etcd to `127.0.0.1` inside the test VM and set explicit VM memory because the pinned mgmt build can use several GiB while converging this graph. They do not connect to beta, production, Hetzner, Cloudflare, SSH, or operator SecretSpec profiles.

`gitops-closure-roots-vm` generates desired state from tiny real Nix store artifacts inside the test derivation. It proves closure verification and gcroot creation without using fake enabled store paths or serving anything.

`gitops-multi-environment-candidates-vm` boots one local NixOS VM with two enabled preview-like single-host environments. It proves `main.mcl` traverses arbitrary enabled environment keys, publishes separate candidate/admission/status files for each, and does not create served state when both environments are non-serving candidates.

`gitops-multi-environment-served-vm` boots one local NixOS VM with two served preview-like environments on the same host. It proves active symlinks are environment-scoped under `/var/lib/fishystuff/gitops-test/served/<environment>/{site,cdn}` so one served preview cannot overwrite another preview's selected site/CDN tuple.

`gitops-served-closure-roots-vm` combines the served candidate shape with `vm-test-closures`. It verifies and roots active and retained rollback API, Dolt service, site, and CDN artifacts under `/nix/var/nix/gcroots/fishystuff/gitops-test`, confirms Nix reports those paths as GC roots, waits for the `roots-ready` facts under `/run/fishystuff/gitops-test/roots`, then checks the VM-local active symlinks and route selection. It still does not write `/srv/fishystuff` or start real FishyStuff services.

The originally intended `nix:gcroot` resource is not used by the current graph. The graph keeps the concrete retention guarantee by verifying each closure with `nix:closure`, managing direct symlinks under `/nix/var/nix/gcroots/fishystuff/...`, and publishing one `roots-ready` status file per active/retained release after `fishystuff_deploy gitops roots-ready` verifies the symlink targets, store paths, and `nix-store --gc --print-roots` output. Served status, active selection, route handoff, and rollback documents are ordered behind those `roots-ready` exec resources in modes that manage closure roots.

`gitops-served-candidate-vm` keeps activation local and synthetic. When desired state requests `serve: true` in `vm-test` mode, fixture admission must be `passed_fixture`; the local admission fixture also reads the selected site root, CDN runtime manifest, runtime JS/WASM files, and CDN serving manifest from the exact store paths in the release tuple. The graph then writes an active selection document under `/var/lib/fishystuff/gitops-test/active/<environment>.json`, VM-local served symlinks under `/var/lib/fishystuff/gitops-test/served/<environment>/{site,cdn}`, and a route selection document under `/run/fishystuff/gitops-test/routes/<environment>.json`. This is the first safe shape of the future route/symlink switch. It does not start FishyStuff services, write `/srv/fishystuff`, or touch real beta/prod state.

`gitops-served-caddy-handoff-vm` adds a real local Caddy consumer for that handoff shape. Caddy serves the stable symlink roots while mgmt reconciles the underlying active release. The test verifies site content, current CDN runtime files, and retained prior CDN runtime files over HTTP before and after the selected release changes.

`gitops-served-caddy-rollback-transition-vm` applies the same Caddy consumer to rollback. It verifies the active site and runtime manifest switch back to the previous release over HTTP, and that the CDN serving root still serves the rolled-away candidate runtime asset after rollback.

`gitops-dolt-fetch-pin-vm` keeps Dolt transfer local and synthetic. It uses the `fishystuff_deploy dolt fetch-pin` helper, backed by Dolt's own `clone`, `fetch`, and local branch pinning against a file remote inside the VM, to prove the GitOps graph can express "exact commit present locally" without sending a full `.dolt` closure per release or contacting DoltHub.

`gitops-dolt-admission-pin-vm` keeps admission local and synthetic while making it DB-backed. The optional VM-only `admission_probe.kind = "dolt_sql_scalar"` path writes a probe request, waits for `fetch_pin`, verifies the pinned materialization status, and executes a one-row/one-column Dolt SQL query through `fishystuff_deploy dolt probe-sql-scalar` before writing the admission document.

`gitops-served-retained-dolt-fetch-pin-vm` covers the rollback data path: when a served environment retains rollback releases whose Dolt materialization is `fetch_pin`, every retained release is materialized through the same VM-local cache and its own release-ref status path. Served active/status/route files and the rollback-set documents depend on every retained materialization, and the primary rollback readiness file depends on the first retained release's materialization, so served state does not claim a rollback set whose Dolt commits were never pinned locally.

Fallbacks introduced: none to the old beta deployment graph. The validation no-op is a mode-specific safety guard, not compatibility with an old code path.

## Admission

Admission is modeled separately from graph acceptance. In `validate`, admission is `not_run` and must not be treated as success. In `vm-test`, admission is a deterministic local fixture written under `/run/fishystuff/gitops-test/admission/`; by default it is `passed_fixture`, and tests may explicitly request `failed_fixture` through `admission_fixture_state`. A VM test environment may also request `admission_probe.kind = "dolt_sql_scalar"` to run a configured single-scalar SQL probe against the exact pinned Dolt cache/ref before admission is published. `local-apply` and VM modes may request `admission_probe.kind = "http_status"`, `admission_probe.kind = "http_json_scalar"`, or `admission_probe.kind = "api_meta"` to run a loopback-only HTTP GET through the Rust helper before status/active/route files publish. `api_meta` is the FishyStuff-specific admission shape: it verifies `/api/v1/meta` reports the selected release ID, release identity, and Dolt commit in one admission result. If `api_service` is set, HTTP admission depends on that candidate service being reconciled to `running`; if the release uses `fetch_pin`, the candidate API environment also receives `FISHYSTUFF_DEFAULT_DOLT_REF` for the selected pinned release ref. For serving fixtures, local admission must be able to read the selected `site/index.html`, `cdn_runtime/map/runtime-manifest.json`, the selected runtime JS/WASM files, and `cdn_runtime/cdn-serving-manifest.json`. The serving manifest must also account for `runtime-manifest.json` and the selected runtime JS/WASM asset paths.

The Rust deployment helper also provides local HTTP admission probe building blocks:

- `fishystuff_deploy http probe-status`
- `fishystuff_deploy http needs-probe-status`
- `fishystuff_deploy http probe-json-scalar`
- `fishystuff_deploy http needs-probe-json-scalar`
- `fishystuff_deploy http probe-json-scalars`
- `fishystuff_deploy http needs-probe-json-scalars`

These helpers intentionally support only HTTP GET against loopback targets (`localhost`, `127.0.0.1`, or `::1`). They reject credential-bearing URLs, cap response bodies, and write structured status documents with the exact request tuple. This is the intended bridge for future host-local API admission probes until mgmt has a dedicated HTTP client probe primitive.

`http_status` requires `probe_name`, `url`, and `expected_status`; `timeout_ms` defaults to 2000 when omitted or zero. `http_json_scalar` adds `json_pointer` and a string `expected_scalar` for now. `api_meta` derives expected `/release_id`, `/release_identity`, and `/dolt_commit` values from the selected desired-state release and executes the generic multi-scalar HTTP helper. The desired-state generator allows HTTP admission in `local-apply`, `vm-test`, and `vm-test-closures`; Dolt SQL admission remains VM-only because it is still a local integration-test bridge. For served environments, HTTP admission requires `api_upstream`, and the probe URL must target that upstream so admission and route selection cannot drift apart.

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
- `retained_release_ids`
- `rollback_available`
- `rollback_primary_release_id`
- `rollback_retained_count`
- `served`
- `failure_reason`

Local active selection is written only when a VM/local desired state is explicitly serving:

```text
/var/lib/fishystuff/gitops-test/active/<environment>.json
/var/lib/fishystuff/gitops-test/served/<environment>/site
/var/lib/fishystuff/gitops-test/served/<environment>/cdn
/run/fishystuff/gitops-test/routes/<environment>.json
/var/lib/fishystuff/gitops-test/rollback/<environment>.json
```

The active selection document includes the desired generation that selected the served symlinks so route state can be correlated with the desired-state object that produced it.

The route selection document is the local-only handoff shape for future Caddy integration. It records the selected release, the active selection document path, and the stable site/CDN symlink roots that Caddy would serve, without starting or reloading Caddy in VM tests. The route document is declared after the active selection so a future file-watching edge does not observe a selected route before the active symlinks and active JSON exist.

The rollback readiness document records the primary retained rollback release, currently the first `retained_releases` entry, with its exact API, Dolt service, site, CDN runtime, Dolt commit, Dolt materialization/cache/ref tuple, and release identity. The rollback-set index and member documents record the full retained set. This keeps rollback availability explicit instead of inferring it from an operator's memory of the desired-state object.

KV publication can be added later when the status consumer is clear.

## Fast Deployment Invariant

A release may become served only after expensive work is already complete:

- closures realized
- gcroots present and verified by `roots-ready` facts when closure roots are managed
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
