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

For local site and map development, the repo-root flow is now explicit:

- `just dev-build`
- `just up`

Or, if you want `devenv` itself to own rebuild/restart behavior:

- `just watch`

Then add only the rebuild watchers you actually need:

- `just dev-watch-site`
  - rebuild `site/.out` when site sources change
- `just dev-watch-map`
  - rebuild the wasm runtime and refresh the staged CDN payload
- `just dev-watch-cdn`
  - restage CDN-owned browser host assets from `site/assets/map`
- `just dev-watch-builds`
  - run the map/CDN/site rebuild watchers together while `just up` keeps serving the outputs
- `just dev-watch-api`
  - restart the API on backend changes; use it with `just dev-up-no-api`

`just up` now serves the current outputs instead of owning the build graph.
If `site/.out` or `data/cdn/public/` is stale or missing, that state is visible
directly instead of being hidden behind nested watchers.

`just watch` is the opt-in alternative where `devenv` runs the initial
builds, watches source inputs, rebuilds outputs, and restarts the API on
changes.

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
