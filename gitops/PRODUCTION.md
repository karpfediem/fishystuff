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
just gitops-unify auto data/gitops/production-current.desired.json
just gitops-check-desired-serving state_file=data/gitops/production-current.desired.json environment=production
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

The first production-shaped serving artifact is still VM-only:

```bash
nix build .#checks.x86_64-linux.gitops-desired-state-production-vm-serve-fixture --no-link
nix build .#checks.x86_64-linux.gitops-desired-state-production-rollback-transition --no-link
nix build .#checks.x86_64-linux.gitops-desired-state-production-api-meta --no-link
nix build .#checks.x86_64-linux.gitops-desired-state-production-serve-shape-refusal --no-link
nix build .#checks.x86_64-linux.gitops-production-vm-serve-fixture-vm --no-link
nix build .#checks.x86_64-linux.gitops-production-rollback-transition-vm --no-link
nix build .#checks.x86_64-linux.gitops-production-api-meta-vm --no-link
```

It uses production API/Dolt service bundles and production site content, but keeps real serving confined to local fixtures: `vm-test` for symlink/rollback shape and `local-apply` only inside the NixOS VM for loopback API admission. The rollback check proves the production-shaped transition back to `previous-production-release` retains the exact candidate release ID and its CDN root. The API-meta check proves active and retained Dolt commits are pinned into a host-local cache, the candidate API reads the pinned active release ref, and `/api/v1/meta` must report the exact release identity and Dolt commit before served state publishes. The refusal check proves production-shaped serving desired state is rejected when rollback retention or the CDN runtime closure is missing. The VM checks prove the serve, rollback, and API admission shapes write only local VM state.

`gitops-local-apply-fetch-pin-vm` separately proves that `fetch_pin` can run in local-apply mode against a warm host-local cache without using VM-test paths.

## Static Separation Checks

These checks guard common beta/prod mixups in service bundles:

```bash
nix build .#checks.x86_64-linux.api-service-bundle .#checks.x86_64-linux.api-service-bundle-production --no-link
nix build .#checks.x86_64-linux.dolt-service-bundle .#checks.x86_64-linux.dolt-service-bundle-production --no-link
nix build .#checks.x86_64-linux.edge-service-bundle .#checks.x86_64-linux.edge-service-bundle-production --no-link
nix build .#checks.x86_64-linux.vector-agent-service-bundle .#checks.x86_64-linux.vector-agent-service-bundle-production --no-link
```

They assert production bundles carry production environment labels and production edge hostnames. The production edge check rejects `beta.fishystuff.fish` in the generated Caddyfile.

## Before GitOps May Serve Real Production

The VM-only equivalents now exist for production-shaped generated serve, rollback, and loopback API-meta admission. Do not point the GitOps graph at real production host paths or services until the remaining real-host pieces exist:

- A real production Dolt remote or mirror policy for `fetch_pin`, with operator-selected exact active and retained commits.
- A rollback set with rooted artifacts and retained CDN roots, validated before active symlink or route handoff.
- A read-only served-state inspection command that validates status, active, rollback-set, primary rollback readiness, admission, route selection, and root-readiness documents before an operator treats a release as served.
- A real-host desired-state package from exact API, Dolt service, site, finalized CDN serving root, and at least one retained rollback release.
- A real-host service/Caddy handoff that uses production-local paths and does not reuse beta service state.

Activation should still be only the small reconciled switch:

- active release selection
- active site/CDN symlinks
- Caddy route/upstream handoff
- small service restart or reload

It should not build, clone, fetch a full database, or discover rollback assets during an incident.
