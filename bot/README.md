# bot

Deployable Discord bot runtime.

This component should stay a clean binary boundary. Prefer shared contracts or a typed API client over duplicating API-server internals here.

Runtime secrets for local bot runs are declared in
`/home/carp/code/fishystuff/secretspec.toml` under the `bot` profile. Use:

- `just bot-run`
- or `secretspec run --profile bot -- cargo run --manifest-path bot/Cargo.toml`
