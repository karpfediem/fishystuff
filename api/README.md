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

This component should not own:

- raw local developer data
- Bevy/browser rendering code
- offline ingestion or tile-generation pipelines
