# fishystuff

A very fishy website

## Development

### Prerequisites

This project uses [devenv](https://devenv.sh/) for the local development environment.

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

To update the pinned `devenv` inputs after intentional environment changes:

```bash
devenv update
```

### Commands

List commands

```bash
just -l
```
