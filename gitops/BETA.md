# Beta GitOps Service Set

The next beta target is a distinct GitOps-managed service set, preferably on a new Hetzner host while the old beta host remains untouched as operational history. This page tracks the beta-specific contract as it is built out.

Hard boundary for the beta path:

- no production SSH key or host access
- no production service unit names
- no production GitOps state paths
- no production TLS credential paths
- no production public hostnames
- no Cloudflare or Hetzner mutation without an explicit separate confirmation

The first concrete artifact is the beta edge handoff bundle:

```bash
just gitops-beta-edge-handoff-bundle
just gitops-beta-edge-handoff-bundle-test
nix build .#checks.x86_64-linux.edge-service-bundle-beta-gitops-handoff --no-link
```

It validates:

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

1. beta API and Dolt service bundle identities
2. beta desired-state generation under `data/gitops/beta-*`
3. beta activation/admission/proof wrappers parameterized from the production path
4. a local-only beta host handoff plan
5. only then, a separate manually confirmed host bootstrap path
