# Fishystuff Mgmt Topology

This directory is a self-contained mgmt module repository for the Hetzner beta
topology.

Repository shape:

```text
mgmt/
  metadata.yaml
  main.mcl
  modules/
    fishystuff-beta/
    providers/
      hetzner-network/
      hetzner-vm/
      hetzner-vm-network/
      hetzner-volume/
      hetzner-vm-volume/
```

Public module:

- `modules/fishystuff-beta/`

Internal provider wrapper:

- `modules/providers/hetzner-network/`
- `modules/providers/hetzner-vm/`
- `modules/providers/hetzner-vm-network/`
- `modules/providers/hetzner-volume/`
- `modules/providers/hetzner-vm-volume/`

Current scope:

- manage the named beta VPS inventory in Hetzner
- manage the private beta network in Hetzner
- manage the Dolt data volume in Hetzner
- attach the three beta hosts to the private network
- attach the Dolt data volume to `beta-api-db`
- keep the desired first stable beta topology explicit:
  - `beta-api-db`
  - `beta-cdn`
  - `beta-telemetry`
- provide a local bootstrap entrypoint that reads `HETZNER_API_TOKEN` from the
  SecretSpec `beta-deploy` profile

Current engine limitation:

- the current `hetzner:vm` resource still does not expose firewalls, floating
  IPs, labels, or richer server bootstrap lifecycle as first-class mgmt
  resources
- host bootstrap and long-lived per-host mgmt convergence are still follow-up
  work after inventory creation

As a result, this module now owns the beta Hetzner inventory up through
private network and volume attachment, but it does not yet model the full host
bootstrap or edge-hardening story.

Safety defaults:

- `mgmt/main.mcl` defaults `FISHYSTUFF_HETZNER_STATE` to `absent`
- destructive rebuilds remain blocked unless
  `FISHYSTUFF_HETZNER_ALLOW_REBUILD=ifneeded` is set explicitly

Typical local validation:

```bash
just mgmt-beta-unify
```

Typical local bootstrap run:

```bash
just mgmt-beta-bootstrap
```

To request actual server creation, override the target state explicitly:

```bash
just mgmt-beta-bootstrap state=running converged_timeout=45
```

Default topology inputs:

- cluster: `beta`
- datacenter: `fsn1-dc14`
- location: `fsn1`
- private network: `beta-private`
- private network range: `10.42.0.0/16`
- private subnet range: `10.42.0.0/24`
- poll interval: `60s`
- wait interval: `5s`
- private IPs:
  - `beta-api-db`: `10.42.0.10`
  - `beta-cdn`: `10.42.0.20`
  - `beta-telemetry`: `10.42.0.30`
- Dolt data volume: `beta-dolt-data` at `20 GB`

That bootstrap path intentionally uses mgmt's `--converged-timeout` option and
`--no-watch` to support a one-shot local bootstrap flow.

Convergence note:

- poll-driven resources mark themselves dirty on each poll wakeup before
  `CheckApply`
- for a one-shot run to exit on `--converged-timeout`, the converged timeout
  must therefore stay lower than the poll interval
- with the current defaults (`poll=60s`, `converged_timeout=45s`), the absent
  bootstrap validation exits cleanly before the next Hetzner poll cycle
