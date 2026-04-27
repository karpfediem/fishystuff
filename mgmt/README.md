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
      fishystuff-beta-access/
      fishystuff-beta-layout/
      fishystuff-mgmt-control-key/
      hetzner-vm-observed/
      hetzner-location/
    providers/
      cloudflare-dnsmanager/
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
  - exports nested firewall policy classes `catalog:ssh`, `catalog:http_01`,
    and `catalog:https`
  - exports nested host label class `catalog:host(<role>, <region>)`
- `modules/lib/fishystuff-mgmt-control-key/`
  - exports `subscriber(<host>)`
  - generates per-subscriber SSH tunnel keys on the control host
  - renders VM userdata that installs the matching private key on subscribers
  - installs restricted `authorized_keys` entries on `mgmt-root`
- `modules/lib/hetzner-vm-observed/`
  - exports `public_ipv4(<server>)`
- `modules/lib/hetzner-location/`
  - exports `catalog`
  - exports nested `catalog:lookup(<location>)`

Internal provider wrapper:

- `modules/providers/cloudflare-dnsmanager/`
- `modules/providers/hetzner-firewall/`
- `modules/providers/hetzner-network/`
- `modules/providers/hetzner-ssh-key/`
- `modules/providers/hetzner-vm/`
- `modules/providers/hetzner-vm-network/`
- `modules/providers/hetzner-volume/`
- `modules/providers/hetzner-vm-volume/`

Current scope:

- manage the named beta VPS inventory in Hetzner for the current `nbg1`
  topology
- manage the project SSH key used for bootstrap access on new hosts
- manage cluster-scoped Hetzner firewalls for SSH and public HTTP/HTTPS access
- manage the `nbg1` private core network in Hetzner
- manage the Dolt data volume on `site-nbg1-beta`
- attach the `nbg1` core hosts to the private network
- optionally manage beta Cloudflare DNS records for `beta`, `api.beta`,
  `cdn.beta`, and `telemetry.beta`
- label each managed server with cluster, region, role, and firewall selector
  labels
- generate and install per-subscriber mgmt control SSH tunnel keys
- bootstrap a resident `mgmt` service on a host over SSH after the VM exists
- support future host-local `mgmt deploy` updates over SSH without exposing etcd
- keep the desired first beta topology explicit:
  - `mgmt-root` as the single embedded etcd node
  - `site-nbg1-beta` as the beta site/API/Dolt host
  - `telemetry-nbg1` as shared telemetry for beta and later prod traffic
  - `site-nbg1-prod` as a deferred prod host
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

- `mgmt-root` is the only embedded etcd member and also runs ACME solvers
- `site-nbg1-beta` carries beta site, API, Dolt, and edge serving
- `telemetry-nbg1` carries the shared telemetry stack and its public edge route
- `site-nbg1-prod` is modeled but intentionally disabled until beta works again
- `ash` and `sin` CDN hosts are disabled until we have real geo-routing such as
  GeoDNS or BGP

Safety defaults:

- `mgmt/main.mcl` defaults `FISHYSTUFF_HETZNER_STATE` to `absent`
- `mgmt/main.mcl` defaults `FISHYSTUFF_HETZNER_HTTP01_HOST` to empty, so
  public port `80` stays closed unless you explicitly target a host for an
  `http-01` issuance window
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
`/home/carp/code/mgmt-fishystuff-beta`.

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
  target=mgmt-root \
  host=mgmt-root

just mgmt-resident-kickstart-remote \
  target=root@<beta-nbg1-public-ip-or-name> \
  host=site-nbg1-beta
```

The default resident handoff now builds `mgmt` from
`/home/carp/code/mgmt-fishystuff-beta#minimal`, which keeps the remote closure
small enough for weak Hetzner VPS targets. Override
`mgmt_flake=` or `mgmt_package=` if you need a different checkout or package
output.

Routine operator entrypoints:

```bash
just deploy beta
just status beta
```

The beta control target defaults to `mgmt-root`, so routine deploys
write the resident graph once through `mgmt-root`. Nix closures are copied to
the beta site target and telemetry target before deployment; GC-root reuse is
looked up on the host that owns each service family. The resident graph is
scoped by the runtime hostname before activating workload resources, so
`mgmt-root` can participate in the world without applying the `site-nbg1-beta`
workload.

Deploy only a selected service while reusing the currently rooted remote store
paths for the rest of the resident manifest:

```bash
just deploy beta api
```

Open a public or tunneled service view:

```bash
just open beta api
just open beta grafana
```

Copy one or more closures to a host explicitly:

```bash
just push-closure root@<host-ip> .#minimal
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

The generated deployment marker is derived from the manifest contents and the
temporary deploy graph contents. A repeated deploy of the same Nix outputs and
MCL therefore reuses the same marker instead of forcing every host to chase a
new random value.

Bundle push behavior:

- each bundle now carries planner-facing `materialization.json` plus
  `mode-substitute.txt`, `mode-realise.txt`, and `mode-verify.txt`
- `push-fishystuff-bundles-remote.sh` transfers all selected bundle paths,
  store paths, and derivations in one `nix copy --substitute-on-destination`
  call per target, so cacheable
  dependencies can still be fetched by the target instead of uploaded from the
  builder
- resident mgmt owns materialization, bundle liveness, and selection through
  `nix:closure` plus `nix:gcroot`; the push helper does not realize paths or
  mutate GC roots itself

Default topology inputs:

- cluster: `beta`
- poll interval: `60s`
- wait interval: `5s`
- wait timeout: `600s`
- optional Cloudflare DNS input:
  - `CLOUDFLARE_API_TOKEN`
- beta DNS targets are now derived inside the MCL graph from observed
  `hetzner:vm.publicipv4` metadata:
  - `beta` follows `site-nbg1-beta`
  - `api.beta` follows `site-nbg1-beta`
  - `cdn.beta` follows `site-nbg1-beta` for now
  - `telemetry.beta` follows `telemetry-nbg1`, falling back to
    `site-nbg1-beta` only while telemetry has no observed public IPv4
- `nbg1` core region:
  - private network: `beta-nbg1-private`
  - private network range: `10.0.0.0/16`
  - private subnet range: `10.0.0.0/24`
  - private IPs:
    - `mgmt-root`: `10.0.0.2`
    - `site-nbg1-beta`: `10.0.0.3`
    - `telemetry-nbg1`: `10.0.0.4`
    - `site-nbg1-prod`: `10.0.0.5` when prod is enabled later
  - Dolt data volume: `site-nbg1-beta-dolt-data` at `20 GB`
  - server plans:
    - `site-nbg1-beta`: `cx33`
    - `telemetry-nbg1`: `cx33`
    - `site-nbg1-prod`: `cx33` when prod is enabled later
- `ash` and `sin` edge regions are known to the layout library but disabled in
  the current beta topology

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

- the current topology is intentionally restricted to the `nbg1` region
- only `mgmt-root` exposes an embedded etcd member, and only on loopback
- workload hosts subscribe over SSH tunnels to `mgmt-root`
- only `nbg1` carries the private network and Dolt state volume
- geo-replicated CDN hosts are intentionally out of scope until a real
  geo-routing mechanism exists
- if we later add API or telemetry capacity outside `nbg1`, that should be done
  by adding another regional instance with its own local network and state
  resources, not by sharing the `nbg1` private network

That bootstrap path intentionally uses mgmt's `--converged-timeout` option and
`--no-watch` to support a one-shot local bootstrap flow.

Resident mgmt operation:

- `mgmt-root` is the only embedded etcd member and listens on loopback:
  `127.0.0.1:2379` and `127.0.0.1:2380`
- `site-nbg1-beta`, `telemetry-nbg1`, and later `site-nbg1-prod` are
  SSH-tunneled subscribers with `--ssh-url=root@204.168.223.57` and
  `--seeds=http://127.0.0.1:2379`; they do not expose etcd ports
- ACME solver resources are scoped to `mgmt-root`; workload hosts materialize
  the finalized certificate bundle from shared world state
- later updates are pushed by SSHing to `mgmt-root` and running `mgmt deploy`
  against `--seeds=http://127.0.0.1:2379`
- this keeps the control surface SSH-only for now and avoids exposing etcd on
  the public internet
- the subscriber path uses MCL-generated per-host SSH identities installed by
  VM userdata and restricted by `permitopen="127.0.0.1:2379"` on `mgmt-root`
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
