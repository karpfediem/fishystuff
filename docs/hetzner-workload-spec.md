# Hetzner Workload Spec

Status: draft

## Goal

Define the deployment model for the Hetzner-hosted beta and the later
production rollout.

This version replaces the earlier Fly-plus-Pages framing with a generic Linux
host model built around:

1. Nix-built software artifacts produced on a developer or CI machine
2. per-host `mgmt` convergence on the target machine
3. explicit public service roles for `api`, `cdn`, and `telemetry`
4. a beta rollout that can start on one VPS and later split by role without
   changing the service contracts

## Decisions

### 1. Artifact authority: Nix on the builder

All software artifacts should be built on a developer workstation or CI runner,
not on the Hetzner targets.

Reasons:

- the deploy targets are not good build machines
- the repo already contains modular service bundle work under `nix/services/`
- API and DB runtime closures are already moving toward immutable Nix outputs

The target hosts should only need:

- `nix` installed and running
- enough disk to receive copied closures
- `mgmt` installed locally
- the runtime payloads and secret overlays for the active release

### 2. Host authority: per-host `mgmt`

Each deployed VPS should run `mgmt` locally and use it to converge host state.

`mgmt` should own:

- baseline packages and service users
- directories, state roots, and mounts
- systemd unit files
- edge proxy config and service enablement
- runtime env files and other small host-specific overlays
- activation of exact Nix store paths selected for a release

`mgmt` should not own:

- compiling the software
- deriving CDN payload content from source game assets
- mutable Dolt contents
- mutable telemetry backend data

### 2a. Infrastructure authority: `mgmt` manages Hetzner itself

`mgmt` should also manage the Hetzner VPS resources in the first place.

That means the intended control flow is:

- a builder-side or operator-side `mgmt` run uses `hetzner:vm` to create and
  update the VPS inventory
- each created VPS then runs local `mgmt` for ongoing convergence

This keeps Hetzner machine creation inside the same automation system instead of
splitting it across manual console work and post-creation host automation.

### 3. Generic Linux means generic Linux with systemd

The near-term target is not "any arbitrary init system". It is generic Linux
distributions with:

- `systemd`
- `ssh`
- a working `nix-daemon`

This is an explicit constraint, not an incidental implementation detail.

Reason:

- the official `mgmt` `svc` resource is a systemd unit resource

If true non-systemd support is needed later, that is engine work.

### 4. Public deployment scope

The first-class public workload hostnames are:

- `api.beta.fishystuff.fish`
- `cdn.beta.fishystuff.fish`
- `telemetry.beta.fishystuff.fish`
- `api.fishystuff.fish`
- `cdn.fishystuff.fish`
- `telemetry.fishystuff.fish`

This spec is intentionally centered on those services.

Cloudflare DNS for those names should be managed declaratively by `mgmt`.
Until the `hetzner:vm` inventory resource exposes public IPs into the MCL
graph, the DNS layer should take explicit target IPs or hostnames as topology
inputs instead of trying to infer them implicitly. On the fishystuff side,
those explicit targets should be passed in one structured topology input rather
than as a separate environment variable per hostname.

For `cdn.beta.fishystuff.fish`, the DNS layer should be able to manage a
multi-value record set so multiple regional CDN nodes can sit behind one
hostname without dropping back to manual DNS edits.

If the root site later moves onto Hetzner too, that should be added as a
separate `site` role instead of being hidden inside the CDN or API role.

## Current Repo Facts

The current tree already constrains the correct design:

- sibling public endpoint derivation for `api`, `cdn`, and `telemetry` already
  exists in `site/scripts/write-runtime-config.mjs`,
  `tools/scripts/public-endpoints.sh`, and
  `lib/fishystuff_core/src/public_endpoints.rs`
- the API returns normalized relative asset paths and should not own CDN base
  URL resolution
- modular Nix service definitions already exist for `fishystuff-api` and
  `fishystuff-dolt`
- service bundle generation already exists through
  `nix/services/mk-service-bundle.nix`
- the CDN payload is still staged from local build outputs and source-backed
  assets under `data/cdn/public`
- the telemetry stack exists today as a local multi-process development stack,
  not yet as deployable host services

Those facts point to a deployment model with reusable service bundles, explicit
host roles, and a separate contract for content payloads and mutable state.

## Service Closure Contract

The intended deploy boundary for a host-local service is:

1. a dedicated Nix service module
2. a Nix-built closure that is independently transferable to a target host
3. a small activation contract that a deploy tool can realize on a generic
   Linux host with `systemd`

This means a deploy artifact is not just "a binary". It is a closure plus
machine-readable install metadata.

### What the bundle should contain

For each service, the bundle should expose:

- the immutable closure roots needed by the service
- an explicit `bundle.json` contract
- the closure `registration` file
- the closure `store-paths` file
- immutable service artifacts such as:
  - executable path
  - immutable config path
  - rendered `systemd` unit file
- runtime activation metadata such as:
  - service user and group
  - directories that must exist
  - writable paths
  - runtime overlay targets

For `systemd` targets specifically, the bundle should also expose a backend
install contract describing:

- which unit artifact must be installed
- the target unit path under `/etc/systemd/system`
- whether `systemctl daemon-reload` is required after install

### What activation should do

Given one of these bundles, host activation should be able to:

1. ensure the closure exists on the target host
2. install or link the rendered unit artifact into `/etc/systemd/system`
3. create required users, groups, directories, and writable paths
4. write runtime overlays such as secret env files
5. reload `systemd`
6. start or restart the unit according to host policy

The closure should already contain the rendered unit file so that host tooling
does not need to reconstruct service manager configuration from scratch.

### Where `mgmt` should integrate

This contract leaves room for tighter `mgmt` integration without making the
service definitions themselves `mgmt`-specific.

The likely missing `mgmt` primitives are:

- a closure-transfer primitive, roughly equivalent to `nix copy`
- an install-root or profile-switch primitive, roughly equivalent to selecting
  an installed Nix generation

Existing `mgmt` host resources should still be able to own activation.

That means:

- Nix remains responsible for building the closure and rendering immutable
  artifacts such as the `systemd` unit file
- `mgmt` should become better at moving and installing those closures
- existing resources such as `file` and `svc` can still be used to install the
  rendered unit artifact and activate the service on the host

The important non-goal here is adding a separate "Nix service module" concept
inside the `mgmt` engine. The tighter integration points should stay around
build, transport, and install boundaries, while ordinary host-state resources
continue to manage activation.

## Management Topology

There are two distinct `mgmt` roles in the intended design.

### 1. Provisioner `mgmt`

This is the `mgmt` run that manages Hetzner Cloud resources directly.

Responsibilities:

- create, update, and remove VPSs via `hetzner:vm`
- choose datacenter, image, and server type
- ensure the desired beta topology exists in Hetzner at all
- inject the SSH key set needed for bootstrap

Preferred location:

- developer machine or CI runner

It does not need to be a permanent paid VPS in the first beta.

### 2. Host-local `mgmt`

This is the `mgmt` process running on each VPS after bootstrap.

Responsibilities:

- converge host packages, files, users, directories, and systemd units
- activate copied Nix closures and runtime overlays
- keep the host pinned to the intended release contract

### Why both layers are needed

Per-host `mgmt` cannot create itself out of nothing. Something still has to own
the Hetzner project resources and instantiate the machines.

So the correct shape is:

- `mgmt` creates the VPSs
- `mgmt` then converges the VPSs

not:

- Hetzner console creates the VPSs manually
- `mgmt` only starts after the fact

## Target Service Model

The correct deployment boundary is not "three opaque black-box services". It is
three public roles built from a small set of long-running services.

### API stack

Required services:

- `fishystuff-api`
- `fishystuff-dolt`

Responsibilities:

- `fishystuff-api` runs the Axum/Tower server
- `fishystuff-dolt` owns the local SQL server and persistent Dolt state
- the API talks to Dolt over loopback or private host networking

Why this split remains correct:

- the repo already models API and Dolt as distinct runtime concerns
- the current Fly entrypoint is a deployment convenience, not the right
  long-term supervision boundary
- the existing Nix service work already splits them

State:

- Dolt repo clone and SQL metadata live outside the Nix store
- API runtime env files and secrets live outside the Nix store

### CDN stack

Required service:

- `fishystuff-cdn`

Required payload:

- a staged CDN content tree rooted at `data/cdn/public`

Responsibilities:

- serve the staged static payload
- preserve cache semantics for `runtime-manifest*.json` and hashed JS/WASM
- serve map assets, terrain, imagery, GeoJSON, icons, and similar static data

Important boundary:

- the CDN server software should be a Nix-built artifact
- the CDN payload should remain a release payload, not a requirement that the
  target machine rebuild source-derived artifacts locally

That keeps the deployment compatible with source-backed assets that may only be
derivable on a developer machine.

### Telemetry stack

The telemetry public hostname should not be a single monolith. It should be a
small service family on one role.

Required services:

- `fishystuff-telemetry-edge`
- `fishystuff-vector`
- `fishystuff-otel-collector`
- `fishystuff-loki`
- `fishystuff-jaeger`
- `fishystuff-prometheus`

Optional operator service:

- `fishystuff-grafana`

Responsibilities:

- `fishystuff-telemetry-edge` exposes public OTLP HTTP endpoints with the
  required CORS behavior
- `fishystuff-vector` receives browser OTLP, normalizes logs, archives ingress,
  and forwards traces and metrics onward
- `fishystuff-otel-collector` handles trace export and spanmetrics generation
- `fishystuff-loki` stores structured logs
- `fishystuff-jaeger` stores and queries traces
- `fishystuff-prometheus` stores metrics derived from the collector
- `fishystuff-grafana`, if enabled, stays operator-only and should not be part
  of the public hostname contract

This follows the current local telemetry topology rather than inventing a new
one for production.

## Desired Beta Topology

The desired first complete beta topology is three VPSs in one Hetzner region
and one private network:

1. `beta-api-db`
2. `beta-cdn`
3. `beta-telemetry`

This is the recommended target shape for the first real beta, not a later
"scale-out someday" shape.

Reasons:

- it includes Dolt from the start
- it keeps the public services split by failure domain
- it avoids making telemetry compete directly with API latency and Dolt memory
- it still stays comfortably below the stated monthly budget

What this topology explicitly does **not** include:

- no separate `db-node` in the first beta
- no separate `mgmt` controller host
- no Hetzner load balancer
- no floating IP failover layer

Those can be added later if the beta justifies them, but they are not needed
for the first stable rollout.

### `beta-api-db`

Services:

- `fishystuff-dolt`
- `fishystuff-api`
- edge proxy for `api.*`

State:

- persistent Dolt state volume mounted under `/var/lib/fishystuff/dolt`

Topology rules:

- `api.beta.fishystuff.fish` resolves here
- Dolt is not split onto its own VPS for the first beta
- Dolt should listen only on loopback or the private network
- API and Dolt stay co-located to keep the SQL path simple and cheap

### Why Dolt is included here

The first complete beta must include Dolt. The correct initial placement is on
the API host, not on a separate database host.

Reasons:

- the current repo and runtime path already assume close API/Dolt coupling
- it keeps the number of machines low
- it avoids spending budget on an internal-only DB host before we have any HA
  or replica story that actually needs it
- it gives the beta a complete runtime model instead of a partial one

### `beta-cdn`

Services:

- `fishystuff-cdn`
- edge proxy for `cdn.*`

State:

- deployed CDN payload tree

Topology rules:

- `cdn.beta.fishystuff.fish` resolves here
- this host should stay stateless apart from logs and the current payload root

### `beta-telemetry`

Services:

- `fishystuff-telemetry-edge`
- `fishystuff-vector`
- `fishystuff-otel-collector`
- `fishystuff-loki`
- `fishystuff-jaeger`
- `fishystuff-prometheus`
- optional `fishystuff-grafana`

State:

- telemetry backend storage under a stable host path

Topology rules:

- `telemetry.beta.fishystuff.fish` resolves here
- Grafana, if enabled, should be operator-only and not publicly exposed by
  default
- browser OTLP ingress is public, but the backend storage and UIs are not

### Smoke topology

There is still value in a one-box smoke topology for bootstrap and migration
testing:

- one VPS
- one public IPv4
- all services co-located

But that is only the bootstrap path for deployment testing. It is not the
desired first stable beta topology.

## Host Roles

Host roles should still be composable. The same services should be able to run
co-located for smoke testing or split across the desired beta topology without
changing service contracts.

### `beta-all-in-one`

For bootstrap and smoke deploys, support a single host that combines:

- API stack
- CDN stack
- telemetry stack

Public hostnames can all resolve to the same machine. Host-header routing or
per-service listeners can separate traffic.

This is useful because:

- one Hetzner test host already exists
- it minimizes bootstrap complexity for the very first deployment proof
- it keeps the service contracts split even when the machines are not

### `api-node`

Services:

- `fishystuff-dolt`
- `fishystuff-api`
- edge proxy for `api.*`

State:

- persistent Dolt state volume

Topology rules:

- this is the reusable role behind `beta-api-db`
- Dolt remains co-located with the API unless a later HA or replication design
  proves that a split DB host is warranted

### `cdn-node`

Services:

- `fishystuff-cdn`

State:

- deployed CDN payload tree

### `telemetry-node`

Services:

- `fishystuff-telemetry-edge`
- `fishystuff-vector`
- `fishystuff-otel-collector`
- `fishystuff-loki`
- `fishystuff-jaeger`
- `fishystuff-prometheus`
- optional `fishystuff-grafana`

State:

- telemetry backend storage under a stable host path

### Production split

Once beta is stable, production can either:

- remain co-located if traffic is small, or
- split into `api-node`, `cdn-node`, and `telemetry-node`

No service contract should change when that split happens. Only the host-role
composition changes.

## Artifact Contract

### Nix-built software bundles

Software should be published as immutable Nix outputs with explicit bundle
metadata.

Existing bundle work already establishes the right direction for:

- `fishystuff-api`
- `fishystuff-dolt`

The repo should grow equivalent deployable artifacts for:

- `fishystuff-cdn`
- `fishystuff-telemetry-edge`
- `fishystuff-vector`
- `fishystuff-otel-collector`
- `fishystuff-loki`
- `fishystuff-jaeger`
- `fishystuff-prometheus`
- optional `fishystuff-grafana`

### Release payloads outside the store

Some release material should remain outside the Nix store:

- Dolt repo contents
- Dolt SQL privilege and branch-control files
- runtime secret env files
- TLS private keys or ACME state
- telemetry backend data
- CDN content payload staged from `data/cdn/public`

That is not a failure of the model. It is the correct split between immutable
software and mutable runtime content.

### Generated host release input

Near term, the deploy builder should generate a host-specific release input
that contains:

- the exact software bundle selections
- the exact public hostnames for that environment
- systemd unit definitions or the unit inputs needed to render them
- expected runtime overlay paths
- the CDN payload manifest or tarball reference for hosts that serve CDN

The cleanest short-term representation is a generated host-specific `mcl`
program plus small accompanying data files.

Reason:

- the existing Nix bundle format is strong enough to describe the services
- the current `mgmt` branch does not yet provide a clean built-in bridge from
  bundle metadata to release activation on a remote host

## Budget Envelope

The budget target is below `50 EUR/month`.

The recommended three-VPS beta topology fits comfortably under that ceiling.

Using currently listed Hetzner Cloud prices for Germany/Finland:

- a cost-optimized `CX23` is listed at `€3.49/month`
- a cost-optimized `CX33` is listed at `€5.49/month`
- a cost-optimized `CX43` is listed at `€9.49/month`
- a Primary IPv4 is `€0.50/month`
- Cloud block storage is `€0.044/GB/month`

That makes these reasonable beta shapes:

### Lean beta

- `beta-api-db`: `CX33` + `20 GB` volume + one Primary IPv4
- `beta-cdn`: `CX23` + one Primary IPv4
- `beta-telemetry`: `CX33` + one Primary IPv4

Approximate monthly total:

- servers: `€14.47`
- Primary IPv4s: `€1.50`
- Dolt volume: `€0.88`
- total: `€16.85/month`

### Conservative beta

- `beta-api-db`: `CX33` + `40 GB` volume + one Primary IPv4
- `beta-cdn`: `CX23` + one Primary IPv4
- `beta-telemetry`: `CX43` + one Primary IPv4

Approximate monthly total:

- servers: `€18.47`
- Primary IPv4s: `€1.50`
- Dolt volume: `€1.76`
- total: `€21.73/month`

Even with room for a larger telemetry host or a later floating IPv4, the first
beta stays well below the stated budget.

Interpretation:

- the current single VPS around `5 EUR/month` is still useful for smoke deploys
- the desired first stable beta should spend a bit more to buy clean isolation
  between API/Dolt, CDN, and telemetry

## Mgmt Operational Pattern

The near-term deployment flow should be:

1. Build software artifacts locally with Nix.
2. Run the provisioner `mgmt` graph that owns the Hetzner VPS inventory.
3. Produce the host release input locally for the target role and environment.
4. Produce the CDN payload locally if that release serves CDN content.
5. Bootstrap `nix` and host-local `mgmt` onto newly created VPSs.
6. Copy Nix closures to each target host with `nix copy`.
7. Copy the generated `mcl` release input and any non-store release payloads to
   the target host over SSH.
8. Let local `mgmt` on each host converge the machine to the new release.
9. Run service and endpoint health checks.

This keeps the deployment targets out of the build business while still making
`mgmt` the authority for host convergence.

### Bootstrap sequence for a new host

Each fresh target should be bootstrapped in this order:

1. create the VPS via provisioner `mgmt` and `hetzner:vm`
2. install `nix` and start `nix-daemon`
3. copy a locally built `mgmt` binary closure to the host
4. install a long-lived host `mgmt` systemd service
5. install baseline users, groups, and directories
6. copy the first desired release
7. hand off convergence to the long-lived host `mgmt`

The Fedora 43 test VPS is compatible with this model. Nix boots there cleanly,
and the `mgmt` binary can be delivered as a copied closure rather than built on
the host.

## What Mgmt Should Manage Per Role

### API role

`mgmt` should manage:

- `fishystuff-api.service`
- `fishystuff-dolt.service`
- required users and groups
- `/var/lib/fishystuff/dolt`
- runtime env files under `/run/fishystuff/api` and `/run/fishystuff/dolt`
- edge proxy config for the public API hostname

### CDN role

`mgmt` should manage:

- `fishystuff-cdn.service`
- the on-host current CDN payload root selection
- edge/static server config for the public CDN hostname

### Telemetry role

`mgmt` should manage:

- systemd units for the telemetry stack
- storage directories for Loki, Prometheus, Jaeger, and Vector state
- edge proxy config and CORS routing for the public telemetry hostname
- operator-only UI exposure rules if Grafana is enabled

## Edge And TLS

Public hostnames still need a front door. This should be treated as host-role
composition, not as an excuse to collapse the service boundaries.

Near term:

- each role may own its own edge proxy
- the all-in-one beta host may use one edge proxy with multiple vhosts

The edge layer should terminate TLS and route by hostname:

- `api.*` -> `fishystuff-api`
- `cdn.*` -> `fishystuff-cdn`
- `telemetry.*` -> `fishystuff-telemetry-edge`

TLS state remains mutable host state managed outside the Nix store.

## Explicit Roadblocks

These are current blockers or edge gaps, not design preferences.

### 1. `mgmt deploy --ssh-url` is not usable yet

In the current `feature/mgmt-fmt-baseline` branch, `--ssh-url` is parsed but
still returns `--ssh-url is not implemented yet`.

Impact:

- the intended "run `mgmt deploy` from the developer machine through an SSH hop"
  path is not ready
- near-term deploy orchestration must use explicit `ssh` plus `nix copy`

This does **not** block using `mgmt` to manage Hetzner VPS inventory. It blocks
using `mgmt deploy` itself as the remote transport.

### 2. No first-class Nix bundle activation resource

The repo already produces service bundle metadata, but `mgmt` does not yet have
a first-class resource or function for:

- reading a service bundle contract
- ensuring the closure is present and registered
- rendering units and overlays from that contract
- atomically switching a host release to a new bundle set

Near-term workaround:

- generate host-specific `mcl` from the build machine

Preferred medium-term engine work:

- a resource or function that can consume the existing `bundle.json`,
  `registration`, and `store-paths` contract directly

### 3. `svc` is systemd-specific

This means the supported near-term definition of "generic Linux" is:

- distro-flexible
- systemd-based

If that is too narrow, a new service backend or resource family is required.

### 4. CDN payload sync is still a release concern

The software side of CDN deployment fits the bundle model cleanly.
The content side still needs a first-class release transport story:

- exact manifest plus file sync
- or tarball plus atomic unpack/symlink swap

This is still manageable today, but it is not fully abstracted by the current
`mgmt` deploy path.

### 5. Hetzner prerequisites are still partly manual

The official `hetzner:vm` resource documentation explicitly says the Hetzner
project, API token, and project SSH keys must be prepared manually first.

Impact:

- `mgmt` can own VPS lifecycle once the project exists
- initial project bootstrap is still not fully zero-touch

### 6. `hetzner:vm` does not yet model the full beta topology

The current engine implementation of `hetzner:vm` provisions server instances,
but its create config still leaves these unset:

- Volumes
- Networks
- Firewalls
- Placement groups

Impact:

- `mgmt` can own the beta VPS inventory today
- the desired private network and attached Dolt volume are not yet fully
  expressible through the current resource surface
- the first mgmt topology module should therefore own VPS instances now and
  treat private networking and volume attachment as follow-up work or engine
  work

## Immediate Repo Work

The repo should add or complete the following:

1. `fishystuff-cdn` service module and bundle output
2. telemetry service modules and bundle outputs for the stack listed above
3. a generated host-role release description, preferably emitted as `mcl`
4. a builder-side deploy helper that:
   - builds the selected bundles
   - copies the closures
   - copies the release input
   - copies the CDN payload when needed
5. host `mgmt` bootstrap definitions for:
   - all-in-one beta
   - split API role
   - split CDN role
   - split telemetry role

## First Deploy Sequence

The recommended order for the first end-to-end Hetzner rollout is:

1. write a provisioner `mgmt` graph that owns the Hetzner beta VPS inventory
2. use that graph to create the beta machines
3. bootstrap `nix` and host-local `mgmt` on the created VPSs
4. prove local `mgmt` can own a systemd unit that points at a copied Nix
   closure
5. deploy a one-box smoke topology on the existing test host or one created
   beta host, including Dolt
6. move to the desired three-host beta topology:
   - `beta-api-db`
   - `beta-cdn`
   - `beta-telemetry`
7. validate public hostnames, private networking, Dolt persistence, and
   telemetry ingress end-to-end
8. once the three-host beta is stable, decide whether production starts
   co-located or already split

That order gives the fastest feedback on the parts most likely to require mgmt
engine adjustments.
