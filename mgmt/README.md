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
    fishystuff-beta-dns/
    fishystuff-beta-region/
    lib/
      fishystuff-beta-layout/
      hetzner-vm-observed/
      hetzner-location/
    providers/
      cloudflare-dns-record/
      hetzner-firewall/
      hetzner-network/
      hetzner-ssh-key/
      hetzner-vm/
      hetzner-vm-network/
      hetzner-volume/
      hetzner-vm-volume/
```

Public module:

- `modules/fishystuff-beta/`

Reusable composition module:

- `modules/fishystuff-beta-region/`
- `modules/fishystuff-beta-dns/`

Shared helper module:

- `modules/lib/fishystuff-beta-layout/`
  - exports `catalog`
  - exports nested regional layout classes `catalog:nbg1`, `catalog:ash`, and
    `catalog:sin`
- `modules/lib/fishystuff-beta-access/`
  - exports `catalog`
  - exports nested firewall policy classes `catalog:ssh` and
    `catalog:public_http`
  - exports nested host label class `catalog:host(<role>, <region>)`
- `modules/lib/hetzner-vm-observed/`
  - exports `public_ipv4(<server>)`
- `modules/lib/hetzner-location/`
  - exports `catalog`
  - exports nested `catalog:lookup(<location>)`

Internal provider wrapper:

- `modules/providers/cloudflare-dns-record/`
- `modules/providers/hetzner-firewall/`
- `modules/providers/hetzner-network/`
- `modules/providers/hetzner-ssh-key/`
- `modules/providers/hetzner-vm/`
- `modules/providers/hetzner-vm-network/`
- `modules/providers/hetzner-volume/`
- `modules/providers/hetzner-vm-volume/`

Current scope:

- manage the named beta VPS inventory in Hetzner across three regions
- manage the project SSH key used for bootstrap access on new hosts
- manage cluster-scoped Hetzner firewalls for SSH and public HTTP/HTTPS access
- manage the `nbg1` private core network in Hetzner
- manage the Dolt data volume on `beta-nbg1-api-db`
- attach the `nbg1` core hosts to the private network
- optionally manage beta Cloudflare DNS records for `beta`, `api.beta`,
  `cdn.beta`, and `telemetry.beta`
- label each managed server with cluster, region, role, and firewall selector
  labels
- bootstrap a resident `mgmt` service on a host over SSH after the VM exists
- support future host-local `mgmt deploy` updates over SSH without exposing etcd
- keep the desired first stable beta topology explicit:
  - `beta-nbg1-api-db`
  - `beta-ash-cdn`
  - `beta-sin-cdn`
- provide a local bootstrap entrypoint that reads `HETZNER_API_TOKEN` from the
  SecretSpec `beta-deploy` profile
- keep the initial beta inventory within the current project primary-IP ceiling

Current engine limitation:

- the current graph still does not model host bootstrap as an intrinsic
  lifecycle phase of `hetzner:vm`
- floating IPs and richer post-create provisioning are still not modeled in
  this repo's inventory graph
- the current inventory graph can create VMs, networks, and volumes, but it
  still cannot trigger the resident host bootstrap as part of VM creation
- the current `hetzner:vm` resource always creates servers with public IPv4 and
  IPv6, so private-only internal hosts are not yet expressible in this graph
- that means new hosts currently require a separate SSH kickstart step after
  they appear in Hetzner

As a result, this module now owns the beta Hetzner inventory up through
project SSH key, firewall policy, private network, and volume attachment, and
it now provides a resident host bootstrap path, but it does not yet model a
first-class post-create lifecycle or the full edge-hardening story.

Current compact beta shape:

- `beta-nbg1-api-db` is the single `nbg1` core host
- that core host is intended to carry Dolt, the API, and telemetry service
  placement in the first beta
- `beta-ash-cdn` and `beta-sin-cdn` are the public CDN edge hosts
- there is intentionally no dedicated `beta-nbg1-cdn` or
  `beta-nbg1-telemetry` host in this initial shape

Safety defaults:

- `mgmt/main.mcl` defaults `FISHYSTUFF_HETZNER_STATE` to `absent`
- destructive rebuilds remain blocked unless
  `FISHYSTUFF_HETZNER_ALLOW_REBUILD=ifneeded` is set explicitly

Typical local validation:

```bash
just mgmt-beta-unify
```

At the moment, validation and apply runs for this topology require an `mgmt`
binary that includes the local `hetzner:ssh_key` and `hetzner:firewall`
resource work. Until that lands in your default `mgmt` checkout, point
`mgmt-beta-unify` and `mgmt-beta-bootstrap` at a binary built from
`/home/carp/code/playground/mgmt-missing-features`.

Typical local bootstrap run:

```bash
just mgmt-beta-bootstrap
```

To request actual server creation, override the target state explicitly:

```bash
just mgmt-beta-bootstrap state=running converged_timeout=45
```

The bootstrap helper runs with explicit loopback etcd URLs so it does not
collide with an already-running resident `mgmt` on `127.0.0.1:2379` and
`127.0.0.1:2380`. The `beta-deploy` SecretSpec profile must provide both the
Hetzner SSH public key used at VM create time and the matching private key used
later for resident `mgmt` bootstrap and deploy over SSH. Prometheus and pprof
output remain optional:

```bash
just mgmt-beta-bootstrap \
  state=running \
  converged_timeout=45 \
  prometheus=true \
  prometheus_listen=127.0.0.1:39233 \
  pprof_path=/tmp/fishystuff-beta-bootstrap.pprof
```

Resident host bootstrap validation:

```bash
just mgmt-resident-bootstrap-unify
```

Resident host kickstart over SSH:

```bash
just mgmt-resident-kickstart-remote \
  target=root@<host-ip> \
  host=beta-nbg1-api-db
```

The default resident handoff now builds `mgmt` from
`/home/carp/code/playground/mgmt-missing-features#minimal`, which keeps the
remote closure small enough for weak Hetzner VPS targets. Override
`mgmt_flake=` or `mgmt_package=` if you need a different checkout or package
output.

Resident graph deploy over SSH:

```bash
just mgmt-resident-deploy-remote \
  target=root@<host-ip> \
  dir=mgmt/resident-deploy-probe
```

Resident bundle-backed systemd probe:

```bash
just mgmt-resident-dolt-bundle-probe target=mgmt-root
```

The resident `beta` graph now treats the Nix bundle as the source of truth for
the rendered systemd unit. Host-local mgmt still owns runtime env files,
service ordering, and mutable state preparation, but it no longer reconstructs
`ExecStart` or the unit body from `supervision.argv`.

The resident deploy graph is now manifest-driven. The push helpers generate
`files/resident-manifest.json` inside the temporary deploy graph, `main.mcl`
loads it via `deploy.readfile(...)`, and the resident core loops the manifest's
`services` map to activate bundle-backed units. That keeps the manifest in the
deploy filesystem instead of installing a separate host-local config file or
expanding one environment variable per service input.

Bundle push behavior:

- each bundle now carries planner-facing `materialization.json` plus
  `mode-substitute.txt`, `mode-realise.txt`, and `mode-verify.txt`
- `push-fishystuff-bundles-remote.sh` pre-materializes `substitute` roots on
  the target with `nix-store --realise --max-jobs 0`
- `substitute-or-build` roots are driven through `mode-realise.txt`, which now
  prefers a derivation path when the bundle exports one
- the final transfer uses `nix copy --substitute-on-destination`, so cacheable
  dependencies can still be fetched by the target instead of uploaded from the
  builder
- resident mgmt now owns bundle liveness and selection through
  `nix:closure` plus `nix:gcroot`; the push helper no longer mutates GC roots
  itself
- override `remote_nix_max_jobs=` in `just mgmt-resident-push-api-db` or
  `just mgmt-resident-push-full-stack` if you want target-side builds for the
  `substitute-or-build` class; `0` means fetch-only

Default topology inputs:

- cluster: `beta`
- poll interval: `60s`
- wait interval: `5s`
- wait timeout: `600s`
- optional Cloudflare DNS input:
  - `CLOUDFLARE_API_TOKEN`
- beta DNS targets are now derived inside the MCL graph from observed
  `hetzner:vm.publicipv4` metadata:
  - `beta` follows `beta-nbg1-api-db`
  - `api.beta` follows `beta-nbg1-api-db`
  - `telemetry.beta` follows `beta-nbg1-api-db` unless a dedicated telemetry
    host is enabled later
  - `cdn.beta` is a multi-value record set assembled from the enabled CDN hosts
- `nbg1` core region:
  - private network: `beta-nbg1-private`
  - private network range: `10.42.0.0/16`
  - private subnet range: `10.42.0.0/24`
  - private IPs:
    - `beta-nbg1-api-db`: `10.42.0.10`
  - Dolt data volume: `beta-nbg1-dolt-data` at `20 GB`
  - server plans:
    - `beta-nbg1-api-db`: `cx33`
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
- only `beta-nbg1-api-db` is created in the `nbg1` region in the current
  compact beta shape
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
- `--converged-timeout` is a bootstrap-time `mgmt run` concern; the current
  `mgmt deploy` CLI does not expose that flag

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
