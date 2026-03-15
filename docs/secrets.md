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

Typical setup:

```bash
devenv shell
secretspec config init
just secrets-check api
just secrets-check cdn
```

The `just` helper also tolerates `profile=` if you type it out of habit:

```bash
just secrets-check profile=api
```

Typical runtime usage:

```bash
secretspec run --profile api -- cargo run -p fishystuff_server -- --config api/config.toml
secretspec run --profile api -- cargo run -p fishystuff_ingest -- --help
secretspec run --profile cdn -- ./tools/scripts/push_bunnycdn.sh
secretspec run --profile bot -- cargo run --manifest-path bot/Cargo.toml
```

`devenv up` pins SecretSpec to the repo's `api` profile for the local API
process, so it does not depend on your personal SecretSpec default profile.
