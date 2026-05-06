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

It uses production API/Dolt service bundles and production site content, but keeps real serving confined to local fixtures: `vm-test` for symlink/rollback shape and `local-apply` only inside the NixOS VM for loopback API admission. The rollback check proves the production-shaped transition back to `previous-production-release` retains the exact candidate release ID and its CDN root. The API-meta check proves `/api/v1/meta` must report the exact release identity and Dolt commit before served state publishes. The refusal check proves production-shaped serving desired state is rejected when rollback retention or the CDN runtime closure is missing. The VM checks prove the serve, rollback, and API admission shapes write only local VM state.

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

- A Dolt materialization path that fetches or replicates to an already-warm host-local cache and pins an exact commit before admission.
- A rollback set with rooted artifacts and retained CDN roots, validated before active symlink or route handoff.
- A read-only served-state check that validates status, active, rollback-set, and primary rollback readiness documents.
- A real-host desired-state package from exact API, Dolt service, site, finalized CDN serving root, and at least one retained rollback release.
- A real-host service/Caddy handoff that uses production-local paths and does not reuse beta service state.

Activation should still be only the small reconciled switch:

- active release selection
- active site/CDN symlinks
- Caddy route/upstream handoff
- small service restart or reload

It should not build, clone, fetch a full database, or discover rollback assets during an incident.
