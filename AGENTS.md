# AGENTS.md — fishystuff

Repository notes for working in this monorepo.

## Layout
- `api/`: Axum/Tower API runtime, SQL migrations, and deployment config.
- `bot/`: Discord bot runtime.
- `data/`: Local developer input/output state. Most contents should stay gitignored.
- `lib/`: Shared Rust crates only: contracts, ids, math, transforms, and small support crates.
- `map/`: Bevy WASM runtime, browser bridge, rendering, and local interaction logic.
- `site/`: Zine site and deployable static assets.
- `tools/`: Offline/admin Rust tooling crates and thin scripts under `tools/scripts/`.

## Boundaries
- `lib/*` crates may be depended on by `api/`, `bot/`, `map/`, and `tools/`.
- `map/` depends on `lib/*` crates, not on `api/fishystuff_server`.
- `api/` internals are not depended on by `map/`, `bot/`, or `tools/`.
- `tools/` should prefer `lib/*` crates over runtime internals where avoidable.
- `data/` is not a runtime dependency.

## Dev Environment
- This repo uses one top-level `devenv`: `devenv.nix`, `devenv.yaml`, and `devenv.lock`.
- Use the `devenv` MCP server first for processes, ports, packages, and config. If that is insufficient, fall back to <https://devenv.sh/llms.txt>.
- Prefer `direnv` activation or `devenv shell` over ad hoc shell bootstrapping.
- Start the local stack from the repo root with:
  - `devenv up --no-tui` for foreground
  - `devenv up -d --no-tui` for background `process-compose`
  - `devenv up --profile watch --no-tui` for the stack plus rebuild/restart watchers
- The local observability frontend is Grafana on `127.0.0.1:3000`.
- The first provisioned dashboard is `Fishystuff Local Observability` at `/d/fishystuff-local-observability/fishystuff-local-observability`.
- Use `just open grafana` or `just open loki` for logs-first Explore, `just open jaeger` for Jaeger's native trace/SPM UI, and `just open loki-status` only when you need Loki's raw module status page.
- Use `just open dashboard` when you want the provisioned local overview dashboard instead of Explore.
- Use one-shot local build commands without starting the stack:
  - `just build-map` rebuilds the map runtime plus map-serving CDN assets such as fields, waypoints, and `minimap_visual`
  - `just cdn-stage-icons` rebuilds only the source-backed item icon payload
  - `just cdn-stage` refreshes the broader staged CDN payload, including source-backed item icons
  - `just build-site` rebuilds the site output
  - `just build` runs `build-map`, `cdn-stage-icons`, and `build-site` concurrently
- Use the same `devenv` environment for both JS host checks and Rust/WASM map builds.
- The managed local API uses SecretSpec profile `api`.
- Local API CORS is set explicitly through `FISHYSTUFF_CORS_ALLOWED_ORIGINS`. Do not reintroduce inferred site-origin CORS logic.
- Avoid `--impure` Nix inspection on this repo unless there is no practical alternative.
- In particular, do not run commands like `nix eval --impure --expr '(builtins.getFlake "/home/carp/code/fishystuff")...'`.
- This repo often carries large ignored local state under `data/`, `data/cdn/public/`, and `.devenv`; `--impure` local-path flake evaluation can create very large transient `/nix/store/tmp-*` copies and cause disk spikes.
- Prefer these safer alternatives when you only need package or tool information:
  - `devenv shell -- bash -lc 'command -v <tool>'`
  - `devenv shell -- bash -lc '<tool> --version'`
  - `devenv shell -- bash -lc 'grafana server -v'`
  - `devenv shell -- bash -lc 'vector --version'`
- If Nix-level inspection is still necessary, prefer Git-filtered CLI flake refs such as `nix flake metadata . --json` or `nix eval .#...` over `builtins.getFlake` on an absolute local path, and do not run multiple local-flake evals in parallel.
- If disk is already tight, clean generated local state such as stale `result*`, `.devenv`, and staged CDN payloads before any heavy Nix work.

## Browser Telemetry
- Browser OTLP goes through the local Caddy telemetry ingress at `http://telemetry.localhost:1990/v1/{traces,metrics,logs}`.
- This is intentional: raw Vector OTLP ingest is not the local browser contract because it does not own the deploy-time public CORS behavior for `telemetry.*`; the edge does.
- Start in DevTools MCP: reload the page, inspect `list_network_requests`, and evaluate `window.__fishystuffOtel`.
- A healthy local pipeline should show CORS preflight success plus HTTP `200` `POST` requests for logs, metrics, and traces via `telemetry.localhost`, and `curl -fsS http://127.0.0.1:8686/health` should succeed.
- Use `tools/scripts/vector-tap.sh` as the first live observability entrypoint:
  - `browser-logs`
  - `raw-metrics`
  - `raw-traces`
- Downstream checks:
  - Grafana: `http://127.0.0.1:3000/explore`
  - Prometheus: `http://127.0.0.1:9090/api/v1/query?...`
  - Jaeger: `http://127.0.0.1:16686/api/services` and `http://127.0.0.1:16686/api/traces?...`
  - Browser log archive: `data/vector/archive/otel-logs/YYYY-MM-DD.ndjson`
- The map page continuously exports browser metrics such as `fishystuff_map_bevy_fps` and `fishystuff_map_runtime_visible_layers`.
- Useful DevTools probes:
  - `window.__fishystuffOtel.emitError(...)`
  - `window.__fishystuffOtel.withSpanAsync(...)`
  - `window.__fishystuffOtel.getMeter(...).createCounter(...).add(...)`

## Performance
- Do not make performance claims without running the native profiling harness or the relevant benchmark target.
- Prefer measured improvements over speculative optimization.

## Secrets
- Secret requirements live in `secretspec.toml`.
- Use SecretSpec at runtime:
  - `secretspec check --profile api|cdn|bot`
  - `secretspec run --profile api|cdn|bot -- ...`
- Do not add new dotenv-based secret loading to the repo.

## Data And Source Inputs
- Keep committed documentation under `data/spec/`.
- Small tracked landmark/reference CSVs may live under `data/landmarks/`.
- Treat `data/` as local developer input/output state, not a serving root.
- Stage CDN publish payloads under `data/cdn/`.
- Prefer original game/source files over derived, external, or legacy intermediates whenever original files are available.
- The target end state is that the repo can bootstrap most, and ideally all, derived state from available original files.
- Do not add or preserve legacy-support code paths when the original-file path is available and sufficient.
- When replacing an external or legacy input, remove the old dependency instead of keeping dual-path support unless a user explicitly asks for a temporary migration path.

## Generated Outputs
- Hand-edited map host source:
  - `site/assets/map/loader.js`
  - `site/assets/map/map-host.js`
  - `site/assets/map/map-host.test.mjs`
- Hand-edited icon build source: `site/scripts/build-icon-sprite.mjs`
- Copied Bevy UI stylesheet: `site/assets/map/ui/fishystuff.css`
- Generated map runtime bundle outputs:
  - `data/cdn/public/map/runtime-manifest.json`
  - `data/cdn/public/map/fishystuff_ui_bevy.<hash>.js`
  - `data/cdn/public/map/fishystuff_ui_bevy_bg.<hash>.wasm`
- Generated site icon sprite: `site/assets/img/icons.svg`
- Generated site runtime config: `site/.out/runtime-config.js`
- Runtime-served image, tile, terrain, GeoJSON, and icon assets live under `data/cdn/public/`.
- Treat the contents of `data/cdn/public/` as local CDN payload state; keep only `.gitkeep` placeholders tracked there.
- `site/` should reference CDN-served runtime assets rather than owning a second copy under `site/assets/`.
- `api/` should return normalized relative asset paths and should not resolve CDN base URLs itself.
- Keep raw imagery, terrain inputs, and scratch outputs under `data/`, not under `site/assets/`.
- Do not hand-edit generated bundle outputs.
- Do not commit unrelated generated build outputs.

## Frontend
- The site UI uses DaisyUI for frontend styling. For framework-oriented guidance and component conventions, use the daisyui-blueprint MCP server and optionally refer to <https://daisyui.com/llms.txt>.
- The site is static. SVG icons must be static at runtime.
- Do not add browser-side Iconify usage such as `iconify-icon`, runtime SVG generation, client-side icon fetches, or MutationObserver-based icon patching.
- When adding or replacing site icons, generate static SVGs at build time with `@iconify/utils` via `site/scripts/build-icon-sprite.mjs`, update `site/assets/img/icons.svg`, and reference icons from templates/JS with `<svg><use href="/img/icons.svg#..."></use></svg>`.
