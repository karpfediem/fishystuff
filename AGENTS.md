# AGENTS.md — fishystuff

Repository-level notes for working in this monorepo.

## Subtree instructions
- If you touch `zonegen/`, read [zonegen/AGENTS.md](/home/carp/code/fishystuff/zonegen/AGENTS.md) first.
- `zonegen/AGENTS.md` is now for the temporary migration area only. `zonegen/` is no longer the primary Rust workspace.

## Component responsibilities
- `api/`
  Deployable Axum/Tower API runtime, SQL migrations, and API deployment config.
- `bot/`
  Deployable Discord bot runtime.
- `data/`
  Local developer data working directory. Most contents should remain gitignored.
- `lib/`
  Shared Rust crates only: contracts, ids, math, transforms, and small reusable support crates.
- `map/`
  Bevy WASM runtime, browser bridge, rendering, and local interaction logic.
- `site/`
  Zine site and deployable static assets.
- `tools/`
  Offline/admin Rust tooling crates and thin scripts under `tools/scripts/`.
- `zonegen/`
  Temporary migration residue only: legacy data/images/docs/devenv files still being phased out.

## Dependency rules
- `lib/*` crates may be depended on by `api/`, `bot/`, `map/`, and `tools/`.
- `api/` internals are not depended on by `map/`, `bot/`, or `tools/`.
- `map/` depends on `lib/*` crates, not on `api/fishystuff_server`.
- `tools/` depend on `lib/*` crates rather than runtime internals where avoidable.
- `data/` is not a runtime dependency.
- Do not add new Rust crates back under `zonegen/`.

## Nix shells
- This repo defines multiple `devenv` shells in `flake.nix`. The default shell now includes the full local development stack, while the focused shells remain available when you want a narrower environment.
- For guided edits to `devenv.nix` files, refer to `devenv`'s LLM-oriented reference: <https://devenv.sh/llms.txt>.
- `nix develop .#default`
  - Full local development shell.
  - Includes `just`, `curl`, `dolt`, `gawk`, `lftp`, `python`, `rsync`, `xlsx2csv`, Node/Bun, `zine`, `tailwindcss`, `watchexec`, the stable Rust toolchain, `wasm-bindgen`, `clang`, `mariadb`, and `imagemagick`.
  - `devenv up` from the repo root starts the local process stack: Dolt SQL, map bundle watcher, CDN staging watcher, CDN file server, API server, Zine rebuild watcher, Tailwind watcher, and the local site server.
- `nix develop .#site`
  - Use for site/frontend/browser-host work.
  - Includes the JavaScript runtime/tooling (`node` is available here), Bun, `zine`, `tailwindcss`, `watchexec`, and `just`.
  - Use this shell for commands like `node --check`, `node --test`, Bun tasks, Zine tasks, and Tailwind rebuilds.
- `nix develop .#zonegen`
  - Use for Rust, wasm, Bevy, and map pipeline work.
  - Includes the stable Rust toolchain, `cargo`, `clippy`, `rustfmt`, `rust-analyzer`, the `wasm32-unknown-unknown` target, `wasm-bindgen-cli_0_2_108`, `clang`, `dolt`, `mariadb`, and `imagemagick`.
  - Use this shell for `cargo check`, `cargo test`, wasm builds, and `tools/scripts/build_map.sh`.
- `nix develop .#bot`
  - Separate shell for bot-focused work if needed.

## Practical shell selection
- The default shell is sufficient for full-stack local development and `devenv up`.
- `nix develop .#default` needs `--impure` with this flake pattern so Nix can use the local `devenv-root`.
- If a JS command needs `node`, `bun`, or `zine` and you want a narrower shell, use `.#site`.
- If a Rust/wasm command needs `cargo`, the wasm target, or `wasm-bindgen` and you want a narrower shell, use `.#zonegen`.
- For map runtime changes, the common split is:
  - JS host checks/tests in `.#site`
  - Rust/wasm checks and bundle rebuilds in `.#zonegen`

## Data policy
- Keep committed documentation under `data/spec/`.
- Small tracked landmark/reference CSVs may live under `data/landmarks/`.
- Treat `data/` as local developer input/output state, not a serving root.
- Stage CDN publish payloads under `data/cdn/`.
- Some legacy local inputs still remain under `zonegen/data/` during migration. Do not make runtime components depend on them.

## Generated artifact policy
- Hand-edited map host source lives under:
  - `site/assets/map/loader.js`
  - `site/assets/map/map-host.js`
  - `site/assets/map/map-host.test.mjs`
- The copied Bevy UI stylesheet lives under:
  - `site/assets/map/ui/fishystuff.css`
- Generated map runtime bundle outputs live under:
  - `data/cdn/public/map/runtime-manifest.json`
  - `data/cdn/public/map/fishystuff_ui_bevy.<hash>.js`
  - `data/cdn/public/map/fishystuff_ui_bevy_bg.<hash>.wasm`
- Runtime-served image, tile, terrain, GeoJSON, and icon assets live under `data/cdn/public/`.
- Treat the contents of `data/cdn/public/` as local CDN payload state; keep only `.gitkeep` placeholders tracked there.
- `site/` should reference CDN-served runtime assets rather than owning a second copy under `site/assets/`.
- Keep raw imagery, terrain inputs, and scratch outputs under `data/`, not under `site/assets/`.
- Do not hand-edit generated bundle outputs.
- Do not commit unrelated generated build outputs.

## Frontend references
- The site UI uses DaisyUI for frontend styling. For framework-oriented guidance and component conventions, refer to <https://daisyui.com/llms.txt>.
