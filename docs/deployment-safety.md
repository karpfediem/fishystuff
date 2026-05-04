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
  `site-nbg1-beta`, `telemetry-nbg1`, or the beta `mgmt-root` control target.
- Beta deploys require a complete beta service set, including the dedicated
  `telemetry-nbg1` host reached through `telemetry.beta.fishystuff.fish`; beta
  telemetry must not be collapsed onto the beta site host.
- Before a deploy mutates a remote host, the SSH target must report the expected
  short hostname with `hostname -s`.
- Beta and production use different SSH keys. The beta key must not be
  authorized on production hosts, and the production key must not be authorized
  on beta hosts.
- Production resident access defaults to `root@fishystuff.fish` after DNS
  cutover; Hetzner host discovery is only a special-case override path.
- Production control defaults to the production site target after DNS cutover;
  it must not use the beta `mgmt-root` control path.
- Production observability is intentionally lightweight until a production-owned
  telemetry host exists: default status/deploy paths cover the vector agent but
  not Loki, OTel collector, Prometheus, Jaeger, or Grafana.

Run a local-only preflight with:

```sh
just deploy-safety-check beta
just deploy-safety-check production
just deploy-safety-test
```

This does not contact remote hosts. The actual deploy path repeats the same
configuration checks and then performs the remote hostname identity check before
copying closures or applying the resident graph.

After a deploy, `just smoke <deployment>` checks more than basic HTTP liveness:
it verifies the generated runtime config points at that deployment's site/API/CDN
and telemetry origins, the site HTML carries generated CSP and SRI metadata, the
asset manifest contains SRI entries, and the CDN runtime manifest points at
available content-hashed JS/WASM assets. For remote deployments it also verifies
that mutable runtime pointers are `no-store` while hashed map runtime JS/WASM
assets are served with immutable cache headers.

The key boundary can be verified with:

```sh
just deploy-key-boundary-check
```

That check is non-mutating. It uses the beta key to confirm access to the beta
site and telemetry hosts, confirms the beta key is denied by production, then
uses the production key to confirm access to production and denial by both beta
hosts.

The status, wait, and private tunnel recipes use the same target-boundary checks.
Production observability tunnels intentionally do not fall back to beta telemetry;
they require production telemetry configuration or fail closed.

The beta resident manifest no longer carries a default production hostname. A
production deploy targets production through its own `hostname`, not through a
deferred prod placeholder in a beta manifest.

The resident MCL graph also validates the manifest before including any host
classes. It refuses mismatched environment names, public URLs, Dolt branches, and
beta manifests that carry production host identity. This keeps the graph
fail-closed even if someone bypasses the shell recipe and runs the packaged MCL
directly.

Remaining hardening work:

- Ensure the production host only has the production deploy public key in its
  authorized keys, and beta hosts only have the beta deploy public key.
- Keep beta and production on separate hosts/service sets. Do not share telemetry
  or operator/control services between beta and production.
- Scope Cloudflare tokens so beta can never edit production records.
- Move the long-term deployment path to the GitOps desired-state model with
  exact release identities and admission gates.
