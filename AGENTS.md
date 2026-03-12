# AGENTS.md — fishystuff

Repository-level notes for working in this monorepo.

## Subtree instructions
- If you touch `zonegen/`, read [zonegen/AGENTS.md](/home/carp/code/fishystuff/zonegen/AGENTS.md) first.
- `zonegen/AGENTS.md` is the authoritative instruction file for the `zonegen` workspace and contains the map/runtime/build rules.

## Nix shells
- This repo defines multiple `devenv` shells in `flake.nix`. Pick the shell that matches the task instead of assuming every tool is in the default shell.
- For guided edits to `devenv.nix` files, refer to `devenv`'s LLM-oriented reference: <https://devenv.sh/llms.txt>.
- `nix develop .#default`
  - Lightweight repo shell.
  - Includes basic tools such as `just`, `dolt`, `gawk`, and `xlsx2csv`.
  - Do not assume it has `node`, `bun`, or the Rust toolchain.
- `nix develop .#site`
  - Use for site/frontend/browser-host work.
  - Includes the JavaScript runtime/tooling (`node` is available here), Bun, `zine`, `tailwindcss`, `watchexec`, and `just`.
  - Use this shell for commands like `node --check`, `node --test`, Bun tasks, Zine tasks, and Tailwind rebuilds.
- `nix develop .#zonegen`
  - Use for Rust, wasm, Bevy, and map pipeline work.
  - Includes the stable Rust toolchain, `cargo`, `clippy`, `rustfmt`, `rust-analyzer`, the `wasm32-unknown-unknown` target, `wasm-bindgen-cli_0_2_108`, `clang`, `dolt`, `mariadb`, and `imagemagick`.
  - Use this shell for `cargo check`, `cargo test`, wasm builds, and `zonegen/tools/build_map.sh`.
- `nix develop .#bot`
  - Separate shell for the bot workspace.

## Practical shell selection
- If a JS command needs `node`, run it in `.#site` even when the files live under `zonegen/map/`.
- If a Rust/wasm command needs `cargo`, the wasm target, or `wasm-bindgen`, run it in `.#zonegen`.
- For map runtime changes, the common split is:
  - JS host checks/tests in `.#site`
  - Rust/wasm checks and bundle rebuilds in `.#zonegen`

## Frontend references
- The site UI uses DaisyUI for frontend styling. For framework-oriented guidance and component conventions, refer to <https://daisyui.com/llms.txt>.
