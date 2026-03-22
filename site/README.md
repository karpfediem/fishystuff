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

Runtime image, terrain, icon, and tile assets are CDN-served from `data/cdn/public/` locally and `https://cdn.fishystuff.fish/` in production. The site build no longer copies a runtime image tree into `.out`.

For local map development, either run the pieces manually:

- repo root: `just cdn-serve`
- `site/`: `just watch`

`just cdn-serve` now uses a guarded launcher that reclaims a stale local
`serve_cdn.py` listener on `127.0.0.1:4040` instead of failing immediately on an
address-in-use error. The root `devenv up` stack also runs the same cleanup
before starting the CDN server and again when the managed CDN process exits.
The same guarded pattern now applies to the local API server on
`127.0.0.1:8080`.

Or start the full local stack from the repo root:

- `devenv up`

The repo-level `devenv` stack now uses the native process graph with explicit
readiness ordering:

- `site-tailwind -> site-build`
- `map-build -> cdn-stage -> cdn`
- `db -> api`
- `site` waits for `site-build`, `cdn`, and `api`

That means the local site server only starts once the generated site output
exists and the local API/CDN endpoints referenced by `.out/runtime-config.js`
are already reachable.

## Browser smoke check

Once the local stack is up, run:

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
