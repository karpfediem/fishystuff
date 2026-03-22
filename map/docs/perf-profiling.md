# Map Performance Profiling

Current execution priorities are tracked in [perf-workstream.md](./perf-workstream.md).
The single-threaded-web baseline and future threading constraints are recorded in [web-threading-readiness.md](./web-threading-readiness.md).

## Goals
- Measure the native Bevy map runtime with deterministic inputs.
- Separate end-to-end scenario timings from pure CPU hot-path microbenchmarks.
- Produce machine-readable artifacts that can be compared by scripts or Codex.

## Profiling layers
- Scenario harness: native `profile_harness` binary that runs scripted map scenarios and writes JSON summaries.
- Microbenchmarks: Criterion benches for pure CPU paths such as raster tile planning, terrain mesh generation, and event clustering.
- Manual traces: optional Chrome trace output from the harness for deeper inspection in Perfetto or Chrome tracing.

## Deterministic fixtures
- Harness fixtures live under `map/fishystuff_ui_bevy/tests/fixtures/profiling/`.
- The profiling workflow does not depend on the live API or developer-local data directories.
- Fixture sizes are intentionally small enough to keep runs repeatable while still exercising raster, vector, terrain, and event workloads.

## Native harness
- Canonical baseline scenario:
  ```bash
  PERF_WARMUP_FRAMES=0 tools/scripts/perf-run-scenario.sh load_map
  ```
- Run one scenario:
  ```bash
  tools/scripts/perf-run-scenario.sh raster_2d_pan_zoom
  ```
- Write to a specific report path:
  ```bash
  tools/scripts/perf-run-scenario.sh terrain3d_enter_and_orbit /tmp/terrain.json
  ```
- Emit a Chrome trace in addition to the JSON report:
  ```bash
  PERF_TRACE_CHROME_PATH=target/perf/terrain.trace.json \
    tools/scripts/perf-run-scenario.sh terrain3d_enter_and_orbit
  ```
- The wrapper uses local `cargo`/`xvfb-run` when they are already on `PATH` and otherwise falls back to `devenv shell`.
- `load_map` is the simplest scenario and is the recommended canonical baseline when you want to measure the plain startup/load path before comparing more behavior-heavy scenarios.
- `tools/scripts/perf-run-scenario.sh` also accepts optional environment overrides:
  - `PERF_FRAMES`
  - `PERF_SECONDS`
  - `PERF_WARMUP_FRAMES`
  - `PERF_FIXTURE_ROOT`
  - `PERF_WINDOW_WIDTH`
  - `PERF_WINDOW_HEIGHT`

## Report artifacts
- Default scenario reports land under `target/perf/<scenario>.json`.
- JSON reports contain frame-time quantiles, named span summaries, and counters/gauges.
- Top spans can be summarized with:
  ```bash
  tools/scripts/perf-top-spans.sh target/perf/raster_2d_pan_zoom.json
  ```

## Comparing runs
- Compare two JSON reports:
  ```bash
  tools/scripts/perf-compare.sh baseline.json candidate.json
  ```
- The comparison script prints frame-time deltas and the named spans with the largest total-time changes.
- The `perf-compare.sh` and `perf-top-spans.sh` helpers use local `jq` when available and otherwise fall back to `devenv shell`.

## Microbenchmarks
- Run the CPU-only benchmark suite:
  ```bash
  tools/scripts/perf-bench.sh
  ```
- Current benches cover:
  - raster visible tile computation
  - raster desired-set construction
  - raster eviction scoring
  - vector triangulation
  - terrain chunk mesh generation
  - event snapshot bbox query
  - event clustering

## Build profile
- Profiling runs use the Cargo `profiling` profile:
  - inherits from `release`
  - keeps line-table debug info via `debug = 1`
  - keeps symbols with `strip = "none"`
- For sampling tools that benefit from frame pointers, run with:
  ```bash
  RUSTFLAGS="-C force-frame-pointers=yes" tools/scripts/perf-run-scenario.sh raster_2d_pan_zoom
  ```

## Tooling
- The top-level `devenv` shell includes the minimum supported profiling toolchain:
  - `jq`
  - `hyperfine`
  - `valgrind`
  - `perf`
  - `chromium`
  - `xvfb-run`

## Browser smoke guard
- The single-threaded browser/WASM startup path has a committed smoke check:
  ```bash
  tools/scripts/map-browser-smoke.sh
  ```
- It launches headless Chromium against the local `/map` page, waits for
  `FishyMapBridge` to report `ready` with a usable fish catalog, and writes a
  JSON result to `target/smoke/map-browser.json`.
- Use it after `devenv up` to catch browser startup stalls or renderer startup
  regressions before making performance claims from native harness results.

## Integrated browser profiling
- The browser-integrated profiler measures the JS host, wasm bridge, and Bevy runtime together from the real `/map` page.
- Run the canonical browser startup scenario:
  ```bash
  tools/scripts/map-browser-profile.sh load_map
  ```
- Run the vector-layer enable scenario that exercises the integrated browser path:
  ```bash
  tools/scripts/map-browser-profile.sh vector_region_groups_enable
  ```
- Run the real page-shell DOM toggle scenario that exercises `loader.js` plus the bridge:
  ```bash
  tools/scripts/map-browser-profile.sh vector_region_groups_dom_toggle
  ```
- Browser profiling reports default to `target/perf/browser/<scenario>.json`.
- The browser report keeps the same top-level shape as the native harness report:
  - `scenario`
  - `frames`
  - `warmup_frames`
  - `frame_time_ms`
  - `named_spans`
  - `counters`
- That means the existing helpers also work on browser reports:
  ```bash
  tools/scripts/perf-top-spans.sh target/perf/browser/vector_region_groups_enable.json
  tools/scripts/perf-compare.sh baseline-browser.json candidate-browser.json
  ```
- Browser reports also include nested `host` and `wasm` sections:
  - `host.*` spans/counters cover `map-host.js` work such as patch flush, bootstrap polling, wasm state pulls, and event handling.
  - `bridge.*` spans/counters cover wasm bridge work such as patch ingest, state apply, snapshot sync, and event emission.
- For long or degraded browser scenarios, `browser_action` records how much of the requested capture window actually completed:
  - `capture_frames_target`
  - `completed_frames`
  - `frame_wait_timed_out`
- `frame_wait_timed_out=true` means the integrated browser run did not advance the requested number of animation frames in time. Treat that as a strong regression signal rather than a normal successful run.

## Stability expectations
- Named span totals and counter deltas are usually more stable than wall-clock frame time.
- First-run shader compilation and dependency builds are noisy; compare warmed runs.
- Scenario reports are intended for before/after comparisons on the same machine class and shell configuration.

## Hotspot notes
- The first measured hotspot ranking for the current browser/native setup is in [perf-hotspots-initial.md](./perf-hotspots-initial.md).
