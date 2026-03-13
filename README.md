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

To run the local development stack managed by `devenv` processes:

```bash
devenv up
```

The managed stack now starts the API directly and reclaims stale local API/CDN
listeners before rebinding, so repeated `devenv up` runs are less likely to get
stuck on old background processes. It also uses the native `devenv` process
graph with explicit readiness checks:

- `db` must become ready before `api`
- `map-build` must become ready before `cdn-stage`, which must become ready
  before `cdn`
- `site-tailwind` must become ready before `site-build`, which must become
  ready before `site`

The long-running server wrappers share one readiness helper instead of each
implementing their own ad hoc startup polling.

Initialize your local SecretSpec provider and check the repo profiles you need:

```bash
secretspec config init
just secrets-check api
just secrets-check cdn
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
