# api

Deployable API runtime.

This component should own:

- the Axum/Tower server crate
- runtime-only route, service, and store code
- Dolt-backed runtime data, schema integration, and schema history
- deployment configuration for the API process

Current contents:

- `api/fishystuff_server/`
- `api/sql/`
- `api/config.toml`
- `api/fly.toml`

Schema changes are tracked by Dolt commits. The repo no longer maintains a
numbered SQL migration chain. For workflow details, see
[`docs/dolt-schema-workflow.md`](/home/carp/code/fishystuff/docs/dolt-schema-workflow.md).

The API returns normalized relative asset paths and does not resolve CDN/public
asset base URLs itself.
Terrain URLs are configured explicitly and are not inferred by inspecting local
CDN staging files or directories at runtime.
Its CORS policy is an explicit origin allowlist, configured via
`[server].cors_allowed_origins` or `FISHYSTUFF_CORS_ALLOWED_ORIGINS`, so dev and
production use the same strict model instead of inferred site origins.
The local API process is started through the SecretSpec `api` profile so
`FISHYSTUFF_DATABASE_URL` comes from `/home/carp/code/fishystuff/secretspec.toml`
instead of dotenv shell loading.

## Fly deployment

The Fly deployment path now assumes:

- one Fly Machine runs both `dolt sql-server` and `fishystuff_server`
- the Dolt repo lives on a Fly volume mounted at `/data`
- the API connects only to the local Dolt SQL server on `127.0.0.1`
- production does not hold DoltHub write credentials

On first boot, the machine performs a shallow single-branch clone from DoltHub
into that volume. On later boots, it reuses the local clone and attempts a
`fetch` / `pull` from DoltHub before starting `dolt sql-server` in read-only
mode. If DoltHub sync fails, the API still starts from the last local clone.
The repo clone is the only persisted Dolt state; the local SQL privilege and
branch-control files under `/data/.doltcfg` are rebuilt on boot so a stale
volume-backed auth database cannot block the API's loopback user.
The loopback API user is granted broad non-admin SQL privileges because Dolt's
access model rejects some normal read traffic under a plain `SELECT` grant, but
the runtime server itself stays read-only in production.
The boot path also writes a local Dolt repo identity before `pull`, so Fly
machine restarts can fast-forward the on-volume clone without any interactive
user config.
The HTTP probes are split intentionally: `/healthz` is a pure liveness check for
Fly, while `/readyz` still exercises the local Dolt-backed store path for
readiness debugging.

Because the app uses a single attached Fly Volume, deployments should use an
`immediate` strategy rather than rolling replacement.

If the public API ever needs arbitrary historical Dolt refs locally, increase
`DOLT_CLONE_DEPTH` or drop shallow clone mode for that deployment.

The deployable artifacts are built through Nix:

- [flake.nix](/home/carp/code/fishystuff/flake.nix) package `api`
- [flake.nix](/home/carp/code/fishystuff/flake.nix) package `api-container`

This follows the same pattern as the bot container build and keeps the Dolt
package pinned through `flake.lock`.

Build the container image locally from the repo root with:

```bash
nix build .#api-container
```

Required deploy-time configuration:

- `DOLT_REMOTE_URL`
- `DOLT_REMOTE_BRANCH` defaults to `main`

Create the Fly volume once before the first deploy:

```bash
fly volumes create fishystuff_data --region fra --size 3 --app api-fishystuff-fish
```

Typical setup:

```bash
fly secrets set DOLT_REMOTE_URL='fishystuff/fishystuff'
```

Override the upstream branch for a specific deployment with an env var update,
for example:

```bash
fly secrets set DOLT_REMOTE_BRANCH='ingest_fishing_data'
```

Deploy through the repo recipe:

```bash
just deploy-api
```

That recipe deploys against the existing `api-fishystuff-fish` Fly app defined
in [api/fly.toml](/home/carp/code/fishystuff/api/fly.toml), disables Fly's
startup smoke checks, and gives the machine a longer wait timeout because first
boot may include a fresh Dolt clone before the API can bind `:8080`.

The Fly app name is `api-fishystuff-fish`, and the intended public hostname is
`https://api.fishystuff.fish`.

This component should not own:

- raw local developer data
- Bevy/browser rendering code
- offline ingestion or tile-generation pipelines
