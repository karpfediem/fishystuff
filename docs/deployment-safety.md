# Deployment Safety Boundaries

Production DNS now points at the Hetzner origin, so beta deploys must fail before
they can affect production state.

The current imperative deployment tooling enforces these local guardrails before a
remote deploy can push closures or run mgmt:

- `beta` deploys must use SecretSpec profile `beta-deploy`.
- `production` deploys must use SecretSpec profile `production-deploy`.
- Beta public URLs must be exactly:
  - `https://beta.fishystuff.fish/`
  - `https://api.beta.fishystuff.fish/`
  - `https://cdn.beta.fishystuff.fish/`
  - `https://telemetry.beta.fishystuff.fish/`
- Production public URLs must be exactly:
  - `https://fishystuff.fish/`
  - `https://api.fishystuff.fish/`
  - `https://cdn.fishystuff.fish/`
  - `https://telemetry.fishystuff.fish/`
- Beta deploys must use Dolt branch `beta`.
- Production deploys must use Dolt branch `main`.
- Beta deploy target values must not mention production public hostnames or
  `site-nbg1-prod`.
- Production deploy target values must not mention beta public hostnames,
  `site-nbg1-beta`, or `telemetry-nbg1`.
- Before a deploy mutates a remote host, the SSH target must report the expected
  short hostname with `hostname -s`.

Run a local-only preflight with:

```sh
just deploy-safety-check beta
just deploy-safety-check production
```

This does not contact remote hosts. The actual deploy path repeats the same
configuration checks and then performs the remote hostname identity check before
copying closures or applying the resident graph.

The beta resident manifest no longer carries a default production hostname. A
production deploy targets production through its own `hostname`, not through a
deferred prod placeholder in a beta manifest.

Remaining hardening work:

- Move from shared root SSH to separate OS deploy identities with sudoers limited
  to each environment's service names and paths.
- Split remote gcroots and mutable roots by environment if beta and production
  ever share a machine.
- Scope Cloudflare tokens so beta can never edit production records.
- Move the long-term deployment path to the GitOps desired-state model with
  exact release identities and admission gates.
