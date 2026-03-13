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

Local development should point `images_public_base_url` at the local CDN server (default `http://127.0.0.1:4040` in `api/config.toml`). Production deploys should override that to `https://cdn.fishystuff.fish`.
The local API process is started through the SecretSpec `api` profile so
`FISHYSTUFF_DATABASE_URL` comes from `/home/carp/code/fishystuff/secretspec.toml`
instead of dotenv shell loading.

This component should not own:

- raw local developer data
- Bevy/browser rendering code
- offline ingestion or tile-generation pipelines
