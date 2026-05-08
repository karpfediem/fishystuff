# GitOps Production Handoff

This note separates the current production reality from the future GitOps production path.

## Current Safe Path

Production is live outside the new `gitops/` reconciler. For now, production updates should remain explicit operator actions using the current production host access path and exact Nix outputs. DNS and remote infrastructure changes still require explicit operator confirmation.

The new GitOps graph must not be used to serve production yet. The current production-shaped GitOps package is validation-only:

```bash
nix build .#checks.x86_64-linux.gitops-desired-state-production-validate --no-link
```

That check proves the graph can decode and unify production-shaped desired state with:

- `mode: validate`
- `serve: false`
- production API and Dolt service bundles
- production site content
- `dolt.branch_context = "main"`

It does not write host state, start services, mutate DNS, or select a served release.

For the current local production tuple, generate an ignored operator handoff artifact with:

```bash
just gitops-production-current-desired
just gitops-check-desired-serving state_file=data/gitops/production-current.desired.json environment=production
just gitops-unify auto data/gitops/production-current.desired.json
```

That file records the local Dolt `main` commit, production API/Dolt service bundles, production site content, and finalized CDN serving root. It remains `mode: validate` and `serve: false`; it is a precise snapshot to inspect and review before a real retained rollback set and serving handoff exist.

When a previous production tuple is known exactly, add it with `FISHYSTUFF_GITOPS_RETAINED_RELEASES_JSON` or `FISHYSTUFF_GITOPS_RETAINED_RELEASES_FILE`. The retained object must name its release ID, generation, Git revision, Dolt commit, API bundle, Dolt service bundle, site content, and CDN runtime closure. The recipe refuses duplicate retained IDs, retained active release IDs, non-store closure paths, and credential-bearing Dolt remotes.

For already-published GitOps rollback-set members, derive the retained JSON from the member documents:

```bash
fishystuff_deploy gitops retained-releases-json \
  --rollback-set /var/lib/fishystuff/gitops/rollback-set/production.json \
  > /tmp/fishystuff-retained-releases.json

FISHYSTUFF_GITOPS_RETAINED_RELEASES_FILE=/tmp/fishystuff-retained-releases.json \
  just gitops-production-current-desired
```

Or use the repo recipe to read the rollback-set index and pass each member document automatically:

```bash
just gitops-retained-releases-json \
  environment=production \
  state_dir=/var/lib/fishystuff/gitops \
  > /tmp/fishystuff-retained-releases.json
```

This is read-only and refuses incomplete or inconsistent rollback member identities.

With retained rollback input available, the checked local handoff sequence is:

```bash
FISHYSTUFF_GITOPS_RETAINED_RELEASES_FILE=/tmp/fishystuff-retained-releases.json \
  just gitops-production-current-handoff
```

That recipe generates `data/gitops/production-current.desired.json`, runs the desired-serving preflight, verifies that every active and retained closure path exists locally, verifies that the active CDN serving manifest retains each rollback CDN root, runs `gitops-unify` against the exact generated file, writes `data/gitops/production-current.handoff-summary.json`, and verifies that summary before printing the ready marker. The summary records the desired-state SHA-256, active release, retained releases, Dolt commits, closure paths, CDN retention relationship, and the local checks that passed. It still does not write host state, start services, mutate DNS, or select a served release.

To re-check a previously generated handoff before it is consumed by later activation work:

```bash
just gitops-check-handoff-summary
```

To generate a serving draft, provide explicit admission evidence:

```bash
just gitops-write-activation-admission-evidence \
  output=/tmp/fishystuff-production-admission.json \
  api_upstream=http://127.0.0.1:18092 \
  api_meta_source=/tmp/fishystuff-api-meta.json \
  db_probe_file=/tmp/fishystuff-db-probe.json \
  site_cdn_probe_file=/tmp/fishystuff-site-cdn-probe.json

just gitops-production-activation-draft admission_file=/tmp/fishystuff-production-admission.json
```

This writes a local `local-apply` desired-state draft and verifies it with the same desired-serving preflight and mgmt unify path. The API upstream must be the host-local admitted candidate API, not the public API URL. It does not run mgmt apply, start services, reload Caddy, mutate DNS, or select a served release by itself.

To re-check a saved activation draft before a later apply path consumes it:

```bash
just gitops-check-activation-draft admission_file=/tmp/fishystuff-production-admission.json
just gitops-review-activation-draft admission_file=/tmp/fishystuff-production-admission.json
```

The review command is read-only. It prints the exact release ID, release identity, Dolt commit, closure paths, API upstream, admission probe names, retained rollback set, and proof hashes that an operator would be about to apply.

The guarded local apply consumer is intentionally awkward to run:

```bash
FISHYSTUFF_GITOPS_ENABLE_PRODUCTION_APPLY=1 \
FISHYSTUFF_GITOPS_ENABLE_LOCAL_APPLY=1 \
FISHYSTUFF_GITOPS_APPLY_DRAFT_SHA256=<activation_draft_sha256 from review output> \
  just gitops-apply-activation-draft admission_file=/tmp/fishystuff-production-admission.json
```

It re-runs the activation review, requires the reviewed draft SHA-256 to match the file it is about to consume, then runs a local one-shot mgmt reconciliation of `gitops/main.mcl` with the activation draft. It does not SSH to remote hosts, call the old deploy script, mutate DNS, or run cloud provider commands. It is still not the recommended production path until the remaining real-host service/Caddy handoff is in place.

After a guarded local reconciliation, verify the local served documents against the same checked tuple:

```bash
just gitops-verify-activation-served admission_file=/tmp/fishystuff-production-admission.json
```

This re-checks the activation draft, then validates status, active selection, rollback-set, rollback readiness, admission, route, and root-readiness documents under the selected local state directories. It is read-only and ties the served state back to the activation draft's generation, release ID, host, API upstream, and admission URL.

Once production GitOps has a served rollback-set document, the repeatable cycle is one command:

```bash
just gitops-production-current-from-served state_dir=/var/lib/fishystuff/gitops
```

That derives `data/gitops/production-current.retained-releases.json` from the served rollback-set members, then runs the checked handoff with that exact retained input. This keeps the next handoff tied to the previously published rollback set instead of an operator's memory.

The handoff recipe has a fast local regression check:

```bash
just gitops-production-current-handoff-test
just gitops-production-edge-handoff-bundle-test
```

The tests use explicit closure/Dolt overrides, fake mgmt, and fake Caddy bundles, so they check recipe and validator composition without real Nix builds or host mutation.

The first production-shaped serving artifact is still VM-only:

```bash
nix build .#checks.x86_64-linux.gitops-desired-state-production-vm-serve-fixture --no-link
nix build .#checks.x86_64-linux.gitops-desired-state-production-rollback-transition --no-link
nix build .#checks.x86_64-linux.gitops-desired-state-production-api-meta --no-link
nix build .#checks.x86_64-linux.gitops-desired-state-production-serve-shape-refusal --no-link
nix build .#checks.x86_64-linux.gitops-production-vm-serve-fixture-vm --no-link
nix build .#checks.x86_64-linux.gitops-production-rollback-transition-vm --no-link
nix build .#checks.x86_64-linux.gitops-production-api-meta-vm --no-link
nix build .#checks.x86_64-linux.gitops-production-edge-handoff-vm --no-link
```

It uses production API/Dolt service bundles and production site content, but keeps real serving confined to local fixtures: `vm-test` for symlink/rollback shape and `local-apply` only inside the NixOS VM for loopback API admission. The rollback check proves the production-shaped transition back to `previous-production-release` retains the exact candidate release ID and its CDN root. The API-meta check proves active and retained Dolt commits are pinned into a host-local cache, the candidate API reads the pinned active release ref, and `/api/v1/meta` must report the exact release identity and Dolt commit before served state publishes. The edge-handoff check runs the actual production GitOps Caddy bundle against GitOps-managed served symlinks and a loopback API-meta fixture. The refusal check proves production-shaped serving desired state is rejected when rollback retention or the CDN runtime closure is missing. The VM checks prove the serve, rollback, API admission, and edge handoff shapes write only local VM state.

`gitops-local-apply-fetch-pin-vm` separately proves that `fetch_pin` can run in local-apply mode against a warm host-local cache without using VM-test paths.

## Static Separation Checks

These checks guard common beta/prod mixups in service bundles:

```bash
nix build .#checks.x86_64-linux.api-service-bundle .#checks.x86_64-linux.api-service-bundle-production --no-link
nix build .#checks.x86_64-linux.dolt-service-bundle .#checks.x86_64-linux.dolt-service-bundle-production --no-link
nix build .#checks.x86_64-linux.edge-service-bundle .#checks.x86_64-linux.edge-service-bundle-production .#checks.x86_64-linux.edge-service-bundle-production-gitops-handoff --no-link
nix build .#checks.x86_64-linux.vector-agent-service-bundle .#checks.x86_64-linux.vector-agent-service-bundle-production --no-link
```

They assert production bundles carry production environment labels and production edge hostnames. The production edge check rejects `beta.fishystuff.fish` in the generated Caddyfile.

The `edge-service-bundle-production-gitops-handoff` package is the first production edge shape for the future GitOps path. It points Caddy at `/var/lib/fishystuff/gitops/served/production/site` and `/var/lib/fishystuff/gitops/served/production/cdn`, uses a stable local candidate API upstream, and leaves those content roots out of activation-created directories so it cannot replace GitOps-managed symlinks with empty directories. This is a bundle/check shape only; it is not deployed by the GitOps scripts above.

Before any operator deploys that edge handoff bundle, inspect the exact local package with:

```bash
just gitops-production-edge-handoff-bundle
just gitops-production-host-handoff-plan admission_file=/tmp/fishystuff-production-admission.json
just gitops-production-preflight admission_file=/tmp/fishystuff-production-admission.json
just gitops-production-preflight admission_file=/tmp/fishystuff-production-admission.json served_state_dir=/var/lib/fishystuff/gitops
```

The bundle check builds or accepts a local `edge-service-bundle-production-gitops-handoff` path and verifies the Caddyfile uses GitOps-managed production served symlinks, loopback candidate API routing, credential-directory TLS files, CDN runtime cache headers, and no legacy `/srv/fishystuff`, fixed `/nix/store` serving root, or beta hostname. It also cross-checks `bundle.json`, the systemd unit, and the bundle artifact symlinks so the recorded Caddy executable/config, run/reload commands, TLS credentials, required served paths, and runtime overlays agree exactly. Finally, it runs `caddy validate` against the generated Caddyfile with temporary placeholder TLS credentials and isolated local state directories.

The host handoff plan composes the reviewed activation draft, admission evidence, and verified edge bundle metadata into the exact host-local steps an operator would later run: guarded local GitOps apply, served-state verification, `fishystuff-edge.service` unit install, systemd daemon reload, edge restart, and final public smoke inspection. It only prints the plan and refusal conditions. It does not write host state, install the unit, restart Caddy, SSH to a host, mutate DNS, or call cloud provider commands.

The production preflight is the aggregate local operator proof. It verifies the handoff summary, activation draft, admission evidence, edge handoff bundle, and dry-run host plan together, then runs the fast helper regressions unless `run_helper_tests=false` is passed. When `served_state_dir` or `rollback_set_path` is supplied, it also derives retained rollback releases from the served rollback-set documents and compares release IDs, commits, closure paths, and Dolt materialization against the handoff summary. It intentionally does not apply the draft, install units, restart services, contact remote hosts, mutate DNS, or call cloud provider commands.

The dry-run host plan has a fast local regression check:

```bash
just gitops-production-host-handoff-plan-test
just gitops-production-preflight-test
```

## Before GitOps May Serve Real Production

The VM-only equivalents now exist for production-shaped generated serve, rollback, and loopback API-meta admission. Do not point the GitOps graph at real production host paths or services until the remaining real-host pieces exist:

- A real production Dolt remote or mirror policy for `fetch_pin`, with operator-selected exact active and retained commits.
- A rollback set with rooted artifacts and retained CDN roots, validated before active symlink or route handoff.
- A real-host desired-state package from exact API, Dolt service, site, finalized CDN serving root, and at least one retained rollback release.
- Real-host deployment wiring for the production GitOps Caddy handoff bundle.

Activation should still be only the small reconciled switch:

- active release selection
- active site/CDN symlinks
- Caddy route/upstream handoff
- small service restart or reload

It should not build, clone, fetch a full database, or discover rollback assets during an incident.
