# Secrets

This repo uses [SecretSpec](https://secretspec.dev/) for local secret management.

The committed secret contract lives in:

- `/home/carp/code/fishystuff/secretspec.toml`

Supported local profiles:

- `api`
  - `FISHYSTUFF_DATABASE_URL`
- `cdn`
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

Typical setup:

```bash
devenv shell
secretspec config init
secretspec check --profile api
secretspec check --profile cdn
```

Typical runtime usage:

```bash
secretspec run --profile api -- cargo run -p fishystuff_server -- --config api/config.toml
secretspec run --profile api -- cargo run -p fishystuff_ingest -- --help
secretspec run --profile cdn -- ./tools/scripts/push_bunnycdn.sh
secretspec run --profile bot -- cargo run --manifest-path bot/Cargo.toml
```

`devenv up` already uses the SecretSpec `api` profile for the local API process.
