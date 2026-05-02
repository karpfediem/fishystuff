# site

Zine static site and deployable browser-host assets.

This component should own:

- page layouts and content
- the `/map` page shell
- published static assets under `site/assets/`

Hand-edited browser-host source stays under `site/assets/map/`. The generated wasm/js map runtime bundle is emitted into `data/cdn/public/map/` with hashed filenames and loaded from the CDN, while the copied Bevy UI stylesheet remains at `site/assets/map/ui/fishystuff.css`.

Each site build also emits `.out/runtime-config.js`, which is the single browser
runtime source of truth for the resolved site/API/CDN base URLs in local
development.
For static deploys, the same generator can derive the public API/CDN/telemetry
origins from one site base like `https://fishystuff.fish` or
`https://beta.fishystuff.fish` via `FISHYSTUFF_PUBLIC_SITE_BASE_URL`, with
explicit per-service overrides available through
`FISHYSTUFF_PUBLIC_API_BASE_URL`, `FISHYSTUFF_PUBLIC_CDN_BASE_URL`,
`FISHYSTUFF_PUBLIC_TELEMETRY_BASE_URL`, and
`FISHYSTUFF_PUBLIC_TELEMETRY_TRACES_ENDPOINT`.
The legacy `FISHYSTUFF_PUBLIC_OTEL_*` names are still accepted as compatibility
aliases.
The repo-managed release build also rewrites `zine.ziggy` through
`site/scripts/run-zine-release.sh`, so canonical URLs, Open Graph URLs, RSS
links, `sitemap.xml`, `robots.txt`, and the runtime config all resolve from the
same public site base.

Runtime image, terrain, icon, and tile assets are CDN-served from `data/cdn/public/` locally and `https://cdn.fishystuff.fish/` in production. The site build no longer copies a runtime image tree into `.out`.

`site/scripts/finalize-assets.mjs` is the final browser asset hardening pass for
site output. It minifies referenced JS/CSS, writes content-hashed files and
public source maps, rewrites HTML with SRI, and injects a CSP meta tag. The CSP
uses the resolved deployment URLs: production and beta builds do not include
loopback script/connect sources, while local builds keep the localhost
allowances needed by the Caddy development ingress.

## Frontend tests

Run the frontend unit suite from `site/` with:

- `bun run test`

The site test runner is Bun only. Test files should import test APIs from
`bun:test`, not `node:test`; `node:*` modules such as `node:assert/strict`,
`node:fs`, and `node:vm` remain fine as utility modules.

Map tests that assert user-facing copy should install the shared test i18n
payload with `installMapTestI18n()` instead of relying on untranslated keys.
Component tests may use small fake DOM objects where they are enough to cover
signal projection, rendering decisions, and event dispatch; browser smoke and
profiling remain separate checks for the integrated `/map` page.

For local site and map development, the repo-root flow is:

- `just up`
  - services only
- `just watch`
  - services plus rebuild/restart watches
- `just build`
  - one-shot rebuild of the map runtime, staged CDN payload, and `site/.out`
- `just build-map`
  - one-shot rebuild of the map runtime plus CDN staging
- `just build-site`
  - one-shot rebuild of `site/.out`

`just up` and `just watch` both go through `devenv` and the same
`process-compose` process view. The difference is only whether the `watch`
profile is enabled.

## Browser smoke check

Once the local stack is serving current outputs, run:

- `tools/scripts/map-browser-smoke.sh`

The smoke check launches headless Chromium against `http://127.0.0.1:1990/map/`,
waits for `window.FishyMapBridge.getCurrentState()` to reach `ready` with a
non-empty fish catalog, and fails if startup stalls or the renderer error
overlay appears.

It writes a machine-readable result to `target/smoke/map-browser.json` by
default. To override the timeout or report path:

- `MAP_SMOKE_TIMEOUT_SECS=45 tools/scripts/map-browser-smoke.sh /tmp/map-browser.json`

## Browser profiling

For integrated browser profiling against the real `/map` page, run:

- `tools/scripts/map-browser-profile.sh load_map`
- `tools/scripts/map-browser-profile.sh vector_region_groups_enable`

These reports land under `target/perf/browser/` by default and include:

- JS host timings from `site/assets/map/map-host.js`
- wasm bridge timings from `map/fishystuff_ui_bevy/src/bridge/host/`
- Bevy runtime spans from the wasm map app itself

The output JSON intentionally matches the native profiling report shape at the
top level, so the existing helpers also work:

- `tools/scripts/perf-top-spans.sh target/perf/browser/load_map.json`
- `tools/scripts/perf-compare.sh baseline-browser.json candidate-browser.json`

For browser scenarios that fail to advance the requested number of frames in
time, inspect `browser_action.frame_wait_timed_out` in the report. That is a
signal that the integrated page became too slow or stalled during the scenario.

For continuous local visibility between deep profiling runs, the browser map
runtime also exports a small OTLP metrics surface through the repo's local
Caddy telemetry ingress and downstream Vector-first telemetry path. This is
intentional: the local browser path exercises the same edge-owned CORS contract
that public `telemetry.*` deployments are expected to expose, instead of
talking to raw Vector directly. Those live gauges land on the same Prometheus
target as the Jaeger spanmetrics after Vector aggregates them and are intended
for always-on map runtime dashboards, while the JSON report harnesses remain the
deeper investigation path. For live event inspection of the browser and Vector
pipeline itself, use
`tools/scripts/vector-tap.sh browser-logs`,
`tools/scripts/vector-tap.sh raw-traces`, or another repo preset.
