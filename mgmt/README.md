# Fishystuff Mgmt Topology

This directory is a self-contained mgmt module repository for the Hetzner beta
topology.

Repository shape:

```text
mgmt/
  metadata.yaml
  main.mcl
  resident-bootstrap/
  resident-probe/
  scripts/
  modules/
    fishystuff-beta/
    fishystuff-beta-region/
    lib/
      fishystuff-beta-layout/
      hetzner-location/
    providers/
      hetzner-network/
      hetzner-vm/
      hetzner-vm-network/
      hetzner-volume/
      hetzner-vm-volume/
```

Public module:

- `modules/fishystuff-beta/`

Reusable composition module:

- `modules/fishystuff-beta-region/`

Shared helper module:

- `modules/lib/fishystuff-beta-layout/`
  - exports `catalog`
  - exports nested regional layout classes `catalog:nbg1`, `catalog:ash`, and
    `catalog:sin`
- `modules/lib/hetzner-location/`
  - exports `catalog`
  - exports nested `catalog:lookup(<location>)`

Internal provider wrapper:

- `modules/providers/hetzner-network/`
- `modules/providers/hetzner-vm/`
- `modules/providers/hetzner-vm-network/`
- `modules/providers/hetzner-volume/`
- `modules/providers/hetzner-vm-volume/`

Current scope:

- manage the named beta VPS inventory in Hetzner across three regions
- manage the `nbg1` private core network in Hetzner
- manage the Dolt data volume on `beta-nbg1-api-db`
- attach the `nbg1` core hosts to the private network
- bootstrap a resident `mgmt` service on a host over SSH after the VM exists
- support future host-local `mgmt deploy` updates over SSH without exposing etcd
- keep the desired first stable beta topology explicit:
  - `beta-nbg1-api-db`
  - `beta-nbg1-cdn`
  - `beta-nbg1-telemetry`
  - `beta-ash-cdn`
  - `beta-sin-cdn`
- provide a local bootstrap entrypoint that reads `HETZNER_API_TOKEN` from the
  SecretSpec `beta-deploy` profile

Current engine limitation:

- the current `hetzner:vm` resource still does not expose firewalls, floating
  IPs, labels, or richer server bootstrap lifecycle as first-class mgmt
  resources
- the current inventory graph can create VMs, networks, and volumes, but it
  still cannot trigger the resident host bootstrap as part of VM creation
- that means new hosts currently require a separate SSH kickstart step after
  they appear in Hetzner

As a result, this module now owns the beta Hetzner inventory up through
private network and volume attachment, and it now provides a resident host
bootstrap path, but it does not yet model a first-class post-create lifecycle
or the full edge-hardening story.

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

Resident host bootstrap validation:

```bash
just mgmt-resident-bootstrap-unify
```

Resident host kickstart over SSH:

```bash
just mgmt-resident-kickstart-remote target=mgmt-root host=beta-nbg1-api-db
```

Resident graph deploy over SSH:

```bash
just mgmt-resident-deploy-remote target=mgmt-root dir=mgmt/resident-deploy-probe
```

Default topology inputs:

- cluster: `beta`
- poll interval: `60s`
- wait interval: `5s`
- wait timeout: `600s`
- `nbg1` core region:
  - private network: `beta-nbg1-private`
  - private network range: `10.42.0.0/16`
  - private subnet range: `10.42.0.0/24`
  - private IPs:
    - `beta-nbg1-api-db`: `10.42.0.10`
    - `beta-nbg1-telemetry`: `10.42.0.30`
  - Dolt data volume: `beta-nbg1-dolt-data` at `20 GB`
  - server plans:
    - `beta-nbg1-api-db`: `cx33`
    - `beta-nbg1-cdn`: `cx23`
    - `beta-nbg1-telemetry`: `cx33`
- `ash` edge region:
  - server plan:
    - `beta-ash-cdn`: `cpx11`
- `sin` edge region:
  - server plan:
    - `beta-sin-cdn`: `cpx12`

Current region guidance:

- exposed by `modules/lib/hetzner-location/` under the imported `locations`
  scope
- `locations.preferred_deploy_locations` = `["nbg1", "ash", "sin"]`
- `locations.avoid_initial_locations` = `["fsn1", "hel1", "hil"]`
- datacenter and network-zone mapping remain derived from the location lookup
  class:
  - `nbg1` -> `nbg1-dc3`, `eu-central`
  - `ash` -> `ash-dc1`, `us-east`
  - `sin` -> `sin-dc1`, `ap-southeast`

Topology constraint:

- the current topology is intentionally split into separate regional instances
- only `nbg1` carries the private core network and Dolt state volume
- `ash` and `sin` are currently public CDN edge hosts only
- this avoids trying to stretch one Hetzner private network across different
  network zones
- if we later add API or telemetry capacity outside `nbg1`, that should be done
  by adding another regional instance with its own local network and state
  resources, not by sharing the `nbg1` private network

That bootstrap path intentionally uses mgmt's `--converged-timeout` option and
`--no-watch` to support a one-shot local bootstrap flow.

Resident mgmt operation:

- each host runs one loopback-only resident `mgmt` service under systemd
- the service starts embedded etcd on `127.0.0.1:2379` and `127.0.0.1:2380`
- later updates are pushed by SSHing to the host and running `mgmt deploy`
  against `--seeds=http://127.0.0.1:2379`
- this keeps the control surface SSH-only for now and avoids exposing etcd on
  the public internet
- the current resident bootstrap assumes a systemd-based Linux host because
  mgmt's `svc` resource is systemd-specific

Bootstrap flow:

1. run the inventory graph locally to create or reconcile the Hetzner objects
2. build `mgmt` locally with Nix and copy the closure to the target host with
   `nix copy`
3. run `mgmt/resident-bootstrap/` once over SSH to install and start the
   `fishystuff-mgmt.service` systemd unit
4. push future host graphs with `mgmt deploy` over the same SSH jump path

Current resident bootstrap artifacts:

- `resident-bootstrap/`
  - self-contained graph that installs the long-lived `fishystuff-mgmt`
    systemd service
- `resident-probe/`
  - tiny deployable graph used to validate that the resident service accepts
    one-shot `mgmt run` execution on a host
- `resident-deploy-probe/`
  - tiny deployable graph used to validate that the long-lived resident
    service accepts later `mgmt deploy` updates

Convergence note:

- poll-driven resources mark themselves dirty on each poll wakeup before
  `CheckApply`
- for a one-shot run to exit on `--converged-timeout`, the converged timeout
  must therefore stay lower than the poll interval
- with the current defaults (`poll=60s`, `converged_timeout=45s`), the absent
  bootstrap validation exits cleanly before the next Hetzner poll cycle
