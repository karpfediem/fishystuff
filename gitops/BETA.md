# Beta GitOps Service Set

The next beta target is a distinct GitOps-managed service set, preferably on a new Hetzner host while the old beta host remains untouched as operational history. This page tracks the beta-specific contract as it is built out.

Hard boundary for the beta path:

- no production SSH key or host access
- no production service unit names
- no production GitOps state paths
- no production TLS credential paths
- no production public hostnames
- no Cloudflare or Hetzner mutation without an explicit separate confirmation

The first concrete artifacts are the beta GitOps handoff service bundles:

```bash
just gitops-beta-service-bundles
just gitops-beta-service-bundles-test
just gitops-beta-edge-handoff-bundle
just gitops-beta-edge-handoff-bundle-test
nix build .#checks.x86_64-linux.api-service-bundle-beta-gitops-handoff --no-link
nix build .#checks.x86_64-linux.dolt-service-bundle-beta-gitops-handoff --no-link
nix build .#checks.x86_64-linux.edge-service-bundle-beta-gitops-handoff --no-link
```

The API bundle validates:

- service ID `fishystuff-beta-api`
- systemd unit `fishystuff-beta-api.service`
- beta runtime env file:
  - `/var/lib/fishystuff/gitops-beta/api/beta.env`
- beta API listener:
  - `127.0.0.1:18192`
- beta deployment and OTEL environment labels
- no shared `/run/fishystuff/api/env`
- no production API user/group lines

The Dolt bundle validates:

- service ID `fishystuff-beta-dolt`
- systemd unit `fishystuff-beta-dolt.service`
- beta runtime env file:
  - `/var/lib/fishystuff/gitops-beta/dolt/beta.env`
- beta data directory:
  - `/var/lib/fishystuff/beta-dolt`
- beta SQL listener:
  - `127.0.0.1:3316`
- beta runtime user/group:
  - `fishystuff-beta-dolt`
- no shared `/run/fishystuff/api/env`
- no production Dolt user/group/state-directory lines

The edge bundle validates:

- service ID `fishystuff-beta-edge`
- systemd unit `fishystuff-beta-edge.service`
- beta hostnames only:
  - `beta.fishystuff.fish`
  - `api.beta.fishystuff.fish`
  - `cdn.beta.fishystuff.fish`
  - `telemetry.beta.fishystuff.fish`
- beta served roots:
  - `/var/lib/fishystuff/gitops-beta/served/beta/site`
  - `/var/lib/fishystuff/gitops-beta/served/beta/cdn`
- beta TLS credential directory:
  - `/run/fishystuff/beta-edge/tls`
- beta loopback API upstream:
  - `127.0.0.1:18192`

The validator refuses production hostnames, production served roots, production TLS paths, and production edge dependencies in the beta edge bundle.

Next pieces to add:

1. beta desired-state generation under `data/gitops/beta-*`
2. beta activation/admission/proof wrappers parameterized from the production path
3. a local-only beta host handoff plan
4. only then, a separate manually confirmed host bootstrap path
