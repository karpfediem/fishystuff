# Dolt Target Smoke

Status: draft

## Goal

Prove the narrow builder-to-host deployment path for the Dolt SQL service
without relying on a host-local `mgmt` service graph yet.

This smoke flow is intentionally small:

1. build the Dolt service bundle with Nix on the builder
2. copy the bundle closure to a target host with `nix copy`
3. keep the selected bundle alive with a GC root
4. install the rendered unit artifact into `/etc/systemd/system`
5. start the unit with `systemctl`
6. verify that Dolt answers `select 1` on the configured SQL listener

It does **not** yet prove:

- Dolt repo bootstrap or clone logic
- API deployment
- CDN deployment
- telemetry deployment
- resident `mgmt` activation of the service

It is only the first proof that a Nix-produced systemd closure can be copied to
one generic Linux host and activated successfully.

## Preconditions

- the target host is reachable over SSH
- the target host has `nix`, `systemd`, and `sudo`
- the local `beta-deploy` SecretSpec profile is configured
- the local builder can run:
  - `nix build`
  - `nix copy`
  - `ssh`
- the target host is acceptable for smoke use and may be modified

The repo keeps the required SSH material inside SecretSpec:

- `HETZNER_SSH_PRIVATE_KEY`
- `HETZNER_SSH_PUBLIC_KEY`

## Default Smoke Command

Use the helper recipe:

```bash
just mgmt-dolt-target-smoke target=root@<host-ip>
```

This recipe:

- builds `.#dolt-service-bundle`
- materializes the SSH private key from SecretSpec into a temporary file
- copies the bundle closure to the remote Nix store
- creates or updates the GC root at:
  - `/nix/var/nix/gcroots/mgmt/fishystuff/dolt-current`
- creates the expected `fishystuff-dolt` user, group, and state directories
- installs the unit from:
  - `.../artifacts/systemd/unit`
- runs `systemctl daemon-reload`
- enables and restarts `fishystuff-dolt.service`
- verifies that:
  - `systemctl is-enabled` succeeds
  - `systemctl is-active` succeeds
  - the bundled `dolt` client answers `select 1`

The current helper also uses `nix copy --no-check-sigs` for the builder-to-host
transfer. That is acceptable for this smoke proof, but it is not the desired
steady-state production trust model.

## Current Assumptions

The helper currently assumes the default Dolt listener:

- host: `127.0.0.1`
- port: `3306`

Override if needed:

```bash
just mgmt-dolt-target-smoke \
  target=root@203.0.113.10 \
  sql_host=127.0.0.1 \
  sql_port=3306
```

You can also override the GC root:

```bash
just mgmt-dolt-target-smoke \
  target=root@203.0.113.10 \
  gcroot=/nix/var/nix/gcroots/mgmt/fishystuff/dolt-smoke-current
```

## What The Helper Installs

The smoke path deliberately uses the bundle root as the install source.

Today that means the remote host consumes:

- `bundle.json`
- `store-paths`
- `registration`
- `artifacts/systemd/unit`
- `artifacts/exe/main`

The important proof is that the unit can be installed from a stable path under
the copied bundle root, not by reconstructing `ExecStart` or other service
details in MCL.

## Expected Result

After a successful run:

- `/nix/var/nix/gcroots/mgmt/fishystuff/dolt-current` points at the selected
  bundle in the remote Nix store
- `/etc/systemd/system/fishystuff-dolt.service` exists
- `fishystuff-dolt.service` is enabled and active
- the Dolt SQL server answers `select 1`

This is the current proof point for the deployment model:

```text
nix build
  -> nix copy
  -> GC root
  -> install rendered unit artifact
  -> daemon-reload
  -> systemd enable/start
  -> SQL health check
```

## Why This Exists

This smoke path is intentionally narrower than the longer-term `mgmt` design.

It helps separate two problems:

1. can a Nix-built systemd closure be deployed and activated on the target host?
2. what generic mgmt-side closure movement or install primitives are still worth
   upstreaming later?

If this smoke path fails, the issue is likely in one of:

- the closure contract
- the rendered unit
- the target host assumptions
- the service runtime itself

and not in a more ambitious host-local `mgmt` orchestration layer.
