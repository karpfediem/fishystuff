# api

Deployable API runtime.

This component should own:

- the Axum/Tower server crate
- runtime-only route, service, and store code
- SQL schema and migrations used by the API deployment path
- deployment configuration for the API process

Current migration contents:

- `api/fishystuff_server/`
- `api/sql/`
- `api/config.toml`
- `api/fly.toml`

The API returns normalized relative asset paths and does not resolve CDN/public
asset base URLs itself.
Its CORS policy is an explicit origin allowlist, configured via
`[server].cors_allowed_origins` or `FISHYSTUFF_CORS_ALLOWED_ORIGINS`, so dev and
production use the same strict model instead of inferred site origins.
The local API process is started through the SecretSpec `api` profile so
`FISHYSTUFF_DATABASE_URL` comes from `/home/carp/code/fishystuff/secretspec.toml`
instead of dotenv shell loading.

## Fly deployment

The Fly deployment path now assumes:

- one Fly Machine runs both `dolt sql-server` and `fishystuff_server`
- the Dolt repo is cloned fresh on boot into ephemeral local storage
- the API connects only to the local Dolt SQL server on `127.0.0.1`
- production does not hold Dolt write credentials

That means there is no persistent Fly Volume in the initial deployment model.
Each machine boot performs a shallow single-branch clone from DoltHub and then
starts the local Dolt SQL server in read-only mode before the API starts.
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
boot includes a fresh Dolt clone before the API can bind `:8080`.

The Fly app name is `api-fishystuff-fish`, and the intended public hostname is
`https://api.fishystuff.fish`.

This component should not own:

- raw local developer data
- Bevy/browser rendering code
- offline ingestion or tile-generation pipelines
