# fishystuff

A very fishy website

## Development

### Prerequisites

This project uses [devenv](https://devenv.sh/) for the local development environment.
Runtime secrets are declared in [secretspec.toml](/home/carp/code/fishystuff/secretspec.toml)
and loaded with [SecretSpec](https://secretspec.dev/).

To install them you can follow this guide: https://devenv.sh/getting-started/

Once installed, enter the development environment with:

```bash
devenv shell
```

If you use `direnv`, run `direnv allow` once at the repo root and the environment
will activate automatically on entry.

To run the local development servers:

```bash
just dev-build
just up
```

`just up` runs `devenv up` and supervises the
long-lived local servers:

- `db` must become ready before `api`
- `caddy` serves `site/.out/` on `127.0.0.1:1990` and `data/cdn/public/` on `127.0.0.1:4040`

Builds and rebuilds are now explicit instead of being hidden inside `devenv up`:

- `just dev-build`
  - one-shot build of the map runtime, staged CDN payload, and site output
- `just dev-watch-map`
  - rebuild the wasm map runtime and restage CDN assets on map/lib changes
- `just dev-watch-cdn`
  - restage CDN-owned browser host assets on `site/assets/map` changes
- `just dev-watch-site`
  - rebuild `site/.out` on site source changes
- `just dev-watch-builds`
  - one command for the map/CDN/site rebuild watchers; use it with a running `just up`
- `just dev-watch-api`
  - restart the API on source changes; use it with `just dev-up-no-api`

If you want `devenv` to own the local rebuild/restart loop too, use the opt-in
watch profile instead of the default stack:

```bash
just watch
```

`just watch` runs `devenv up --profile watch` and adds:

- API auto-restart on backend changes
- map runtime rebuild plus CDN restaging on map/lib changes
- CDN host asset restaging on `site/assets/map` changes
- site output rebuilds on site source changes

The site build still emits `.out/runtime-config.js` from the current
environment. That file is the single local-development source of truth for the
site/API/CDN base URLs consumed by the browser host and Bevy runtime.

The API uses a strict explicit CORS allowlist. Production origins are declared
in [api/config.toml](/home/carp/code/fishystuff/api/config.toml), and `devenv`
adds the local site origins through `FISHYSTUFF_CORS_ALLOWED_ORIGINS`, so the
same CORS model is exercised in both dev and prod.

The API and other DB-backed Rust tooling use the repo's `secretspec.toml`
through repo-owned defaults, so local builds and runs do not require
`secretspec config init`, `FISHYSTUFF_DATABASE_URL`, or SecretSpec selector
environment variables.

Only the `cdn` and `bot` profiles still need an explicit provider setup when
you work on those paths. Check them with:

```bash
just secrets-check cdn
just secrets-check bot
```

To update the pinned `devenv` inputs after intentional environment changes:

```bash
devenv update
```

### Commands

List commands

```bash
just -l
```
