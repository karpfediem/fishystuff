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

## Static Separation Checks

These checks guard common beta/prod mixups in service bundles:

```bash
nix build .#checks.x86_64-linux.api-service-bundle .#checks.x86_64-linux.api-service-bundle-production --no-link
nix build .#checks.x86_64-linux.dolt-service-bundle .#checks.x86_64-linux.dolt-service-bundle-production --no-link
nix build .#checks.x86_64-linux.edge-service-bundle .#checks.x86_64-linux.edge-service-bundle-production --no-link
nix build .#checks.x86_64-linux.vector-agent-service-bundle .#checks.x86_64-linux.vector-agent-service-bundle-production --no-link
```

They assert production bundles carry production environment labels and production edge hostnames. The production edge check rejects `beta.fishystuff.fish` in the generated Caddyfile.

## Before GitOps May Serve Production

Do not add a production `serve: true` path until all of these exist:

- A generated production serving desired-state package from exact API, Dolt service, site, finalized CDN serving root, and at least one retained rollback release.
- A local NixOS VM test that uses the same production-shaped fields but VM-local paths.
- A loopback admission test for the candidate API that verifies `/api/v1/meta` against the exact release ID, release identity, and Dolt commit.
- A Dolt materialization path that fetches or replicates to an already-warm host-local cache and pins an exact commit before admission.
- A rollback set with rooted artifacts and retained CDN roots, validated before active symlink or route handoff.
- A read-only served-state check that validates status, active, rollback-set, and primary rollback readiness documents.

Activation should still be only the small reconciled switch:

- active release selection
- active site/CDN symlinks
- Caddy route/upstream handoff
- small service restart or reload

It should not build, clone, fetch a full database, or discover rollback assets during an incident.
