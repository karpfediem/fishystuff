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
- `beta-deploy`
  - `HETZNER_API_TOKEN`
  - `HETZNER_SSH_KEY_NAME`
  - `HETZNER_SSH_PUBLIC_KEY`
  - `HETZNER_SSH_PRIVATE_KEY`
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

If you are working on the `cdn`, `beta-deploy`, or `bot` profiles, validate
those separately:

```bash
just secrets-check cdn
just secrets-check beta-deploy
just secrets-check bot
```

Typical runtime usage:

```bash
cargo run -p fishystuff_server -- --config api/config.toml
cargo run -p fishystuff_ingest -- --help
secretspec run --profile cdn -- ./tools/scripts/push_bunnycdn.sh
just mgmt-beta-bootstrap state=running converged_timeout=45
secretspec run --profile bot -- cargo run --manifest-path bot/Cargo.toml
```

The API server and `fishystuff_ingest` now resolve `FISHYSTUFF_DATABASE_URL`
through the SecretSpec Rust SDK against the repo's `api` profile, so they do
not need a `secretspec run` shell wrapper for the local Dolt connection.

The `beta-deploy` profile is for Hetzner provisioning/deploy tooling, including
the project SSH key injected at VM create time and the local `mgmt` graph that
owns beta VPS lifecycle. `HETZNER_SSH_KEY_NAME` defaults to
`fishystuff-beta-deploy`. `HETZNER_SSH_PUBLIC_KEY` is required for Hetzner VM
creation, and `HETZNER_SSH_PRIVATE_KEY` is required for the later resident
`mgmt` bootstrap and deploy steps over SSH. The helper recipes keep that
material inside the `beta-deploy` SecretSpec profile and only materialize the
private key into a temporary file for the lifetime of the `ssh` / `nix copy`
process that needs it. The local bootstrap helper runs mgmt in one-shot mode
with `--converged-timeout` and `--no-watch`, binds its embedded etcd only on
explicit loopback URLs, and exits after the Hetzner topology has stabilized
instead of remaining attached as a long-running polling process.

For local debugging, the bootstrap helper also supports optional Prometheus and
pprof output without enabling them by default:

```bash
just mgmt-beta-bootstrap \
  state=running \
  converged_timeout=45 \
  prometheus=true \
  prometheus_listen=127.0.0.1:39233 \
  pprof_path=/tmp/fishystuff-beta-bootstrap.pprof
```
