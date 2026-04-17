# Secrets

This repo uses [SecretSpec](https://secretspec.dev/) for local secret management.

The committed secret contract lives in:

- `/home/carp/code/fishystuff/secretspec.toml`

Supported local profiles:

- `api`
  - `FISHYSTUFF_DATABASE_URL`
- `cdn`
  - `BUNNY_STORAGE_ENDPOINT`
  - `BUNNY_STORAGE_ZONE`
  - `BUNNY_STORAGE_ACCESS_KEY`
  - `BUNNY_FTP_HOST`
  - `BUNNY_FTP_PORT`
  - `BUNNY_FTP_USER`
  - `BUNNY_FTP_PASSWORD`
- `bot`
  - `DISCORD_TOKEN`
  - `MOD_INFO_CHANNEL_ID`
  - `TRAP_CHANNEL_ID`
  - `TRAP_PURGE_WINDOW_S`
  - `TRAP_FALLBACK_TIMEOUTM`

Local API/tooling setup:

```bash
devenv shell
cargo check
cargo test
```

The API path uses repo-owned defaults:

- `.cargo/config.toml` pins `FISHYSTUFF_SECRETSPEC_PATH` at the committed
  `/home/carp/code/fishystuff/secretspec.toml`
- cargo commands use a repo-local `XDG_CONFIG_HOME`, so they do not read a
  developer's global SecretSpec config under `$HOME`
- the `api` profile defaults `FISHYSTUFF_DATABASE_URL` to the local Dolt DSN

If you are working on the `cdn` or `bot` profiles, validate those separately:

```bash
just secrets-check cdn
just secrets-check bot
```

Typical runtime usage:

```bash
cargo run -p fishystuff_server -- --config api/config.toml
cargo run -p fishystuff_ingest -- --help
secretspec run --profile cdn -- ./tools/scripts/push_bunnycdn.sh
secretspec run --profile bot -- cargo run --manifest-path bot/Cargo.toml
```

The API server and `fishystuff_ingest` now resolve `FISHYSTUFF_DATABASE_URL`
through the SecretSpec Rust SDK against the repo's `api` profile, so they do
not need a `secretspec run` shell wrapper for the local Dolt connection.
