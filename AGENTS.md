# AGENTS.md — fishystuff

Repository-level notes for working in this monorepo.

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

## Dependency rules
- `lib/*` crates may be depended on by `api/`, `bot/`, `map/`, and `tools/`.
- `api/` internals are not depended on by `map/`, `bot/`, or `tools/`.
- `map/` depends on `lib/*` crates, not on `api/fishystuff_server`.
- `tools/` depend on `lib/*` crates rather than runtime internals where avoidable.
- `data/` is not a runtime dependency.

## Devenv
- This repo uses one top-level `devenv` environment rooted in:
  - `/home/carp/code/fishystuff/devenv.nix`
  - `/home/carp/code/fishystuff/devenv.yaml`
  - `/home/carp/code/fishystuff/devenv.lock`
- A `devenv` MCP server is available in this environment. Use the `devenv` MCP tools/resources first when inspecting `devenv` options, packages, processes, ports, and related configuration.
- If the `devenv` MCP server does not expose the needed information, fall back to `devenv`'s LLM-oriented reference: <https://devenv.sh/llms.txt>.
- Use `devenv shell` for the interactive development environment.
- Use `devenv up --no-tui` from the repo root to start the local services:
  - Dolt SQL
  - API server
  - CDN file server
  - local site server
- Use `devenv up -d --no-tui` when you want the same stack in the background
  under `process-compose`.
- Use `devenv up --profile watch --no-tui` when you want the same stack plus
  rebuild/restart watches for the API, map runtime, CDN staging, and site
  output.
- Use `just build`, `just build-map`, and `just build-site` for one-shot local
  output builds without starting the stack.
- For live local observability inspection, prefer
  `tools/scripts/vector-tap.sh` as the first entrypoint before falling back to
  Loki queries or archive greps.
- The managed stack uses SecretSpec's `api` profile by default for the local API.
- Local API CORS origins are injected explicitly through
  `FISHYSTUFF_CORS_ALLOWED_ORIGINS`. Do not reintroduce inferred site-origin
  CORS logic.

## Practical environment usage
- The top-level `devenv` environment is the supported development entrypoint.
- Prefer `direnv` activation or `devenv shell` over ad hoc shell bootstrapping.
- For map runtime changes, use the same `devenv` environment for both:
  - JS host checks/tests
  - Rust/wasm checks and bundle rebuilds

## Browser telemetry workflow
- The browser exports OTLP logs, metrics, and traces through the local site at
  `/telemetry/v1/logs`, `/telemetry/v1/metrics`, and `/telemetry/v1/traces`.
- Start in DevTools MCP: reload the page, inspect `list_network_requests`, and
  evaluate `window.__fishystuffOtel` to confirm the bridge is initialized and
  the page is posting to the `/telemetry/v1/*` endpoints.
- Healthy browser telemetry should show HTTP `200` `POST` requests for logs,
  metrics, and traces. Confirm the local pipeline is up with
  `curl -fsS http://127.0.0.1:8686/health`.
- Use `devenv shell -- tools/scripts/vector-tap.sh browser-logs`,
  `raw-metrics`, and `raw-traces` as the first live inspection step. This is
  the central local entrypoint for browser telemetry.
- Query Prometheus with `http://127.0.0.1:9090/api/v1/query?...` for RED and
  browser metrics. The map page continuously exports values such as
  `fishystuff_map_bevy_fps` and `fishystuff_map_runtime_visible_layers`.
- Query Jaeger with `http://127.0.0.1:16686/api/services` and
  `http://127.0.0.1:16686/api/traces?...`. Browser spans land under service
  `fishystuff-site-local`.
- Browser logs flow to Loki, and raw OTLP browser log payloads are archived at
  `data/vector/archive/otel-logs/YYYY-MM-DD.ndjson` for grep-friendly
  inspection.
- For ad hoc probes from DevTools, use
  `window.__fishystuffOtel.emitError(...)`,
  `window.__fishystuffOtel.withSpanAsync(...)`, and
  `window.__fishystuffOtel.getMeter(...).createCounter(...).add(...)`.

## Performance workflow
- Do not make performance claims without running the native profiling harness or the relevant benchmark target.
- Prefer measured improvements over speculative optimization.

## Secrets
- Repo-level secret requirements live in `/home/carp/code/fishystuff/secretspec.toml`.
- Use SecretSpec to load secrets at runtime:
  - `secretspec check --profile api`
  - `secretspec check --profile cdn`
  - `secretspec check --profile bot`
  - `secretspec run --profile api -- ...`
  - `secretspec run --profile cdn -- ...`
  - `secretspec run --profile bot -- ...`
- Do not add new dotenv-based secret loading to the repo.
- Prefer runtime loading with `secretspec run` over exporting secrets into the whole shell.

## Data policy
- Keep committed documentation under `data/spec/`.
- Small tracked landmark/reference CSVs may live under `data/landmarks/`.
- Treat `data/` as local developer input/output state, not a serving root.
- Stage CDN publish payloads under `data/cdn/`.

## Source-of-truth policy
- Prefer original game/source files over derived, external, or legacy intermediates whenever original files are available.
- The target end state is that the repo can bootstrap most, and ideally all, derived state from available original files.
- Do not add or preserve legacy-support code paths when the original-file path is available and sufficient.
- When replacing an external or legacy input, remove the old dependency instead of keeping dual-path support unless a user explicitly asks for a temporary migration path.

## Generated artifact policy
- Hand-edited map host source lives under:
  - `site/assets/map/loader.js`
  - `site/assets/map/map-host.js`
  - `site/assets/map/map-host.test.mjs`
- Hand-edited site icon generation source lives under:
  - `site/scripts/build-icon-sprite.mjs`
- The copied Bevy UI stylesheet lives under:
  - `site/assets/map/ui/fishystuff.css`
- Generated map runtime bundle outputs live under:
  - `data/cdn/public/map/runtime-manifest.json`
  - `data/cdn/public/map/fishystuff_ui_bevy.<hash>.js`
  - `data/cdn/public/map/fishystuff_ui_bevy_bg.<hash>.wasm`
- Generated site icon sprite lives under:
  - `site/assets/img/icons.svg`
- Generated site runtime config lives under:
  - `site/.out/runtime-config.js`
- Runtime-served image, tile, terrain, GeoJSON, and icon assets live under `data/cdn/public/`.
- Treat the contents of `data/cdn/public/` as local CDN payload state; keep only `.gitkeep` placeholders tracked there.
- `site/` should reference CDN-served runtime assets rather than owning a second copy under `site/assets/`.
- `api/` should return normalized relative asset paths and should not resolve CDN base URLs itself.
- Keep raw imagery, terrain inputs, and scratch outputs under `data/`, not under `site/assets/`.
- Do not hand-edit generated bundle outputs.
- Do not commit unrelated generated build outputs.

## Frontend references
- The site UI uses DaisyUI for frontend styling. For framework-oriented guidance and component conventions, use the daisyui-blueprint MCP server and optionally refer to <https://daisyui.com/llms.txt>.
- The site is static. SVG icons must be static at runtime.
- Do not add browser-side Iconify usage such as `iconify-icon`, runtime SVG generation, client-side icon fetches, or MutationObserver-based icon patching.
- When adding or replacing site icons, generate static SVGs at build time with `@iconify/utils` via `site/scripts/build-icon-sprite.mjs`, update `site/assets/img/icons.svg`, and reference icons from templates/JS with `<svg><use href="/img/icons.svg#..."></use></svg>`.
