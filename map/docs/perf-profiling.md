# Map Performance Profiling

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

## Stability expectations
- Named span totals and counter deltas are usually more stable than wall-clock frame time.
- First-run shader compilation and dependency builds are noisy; compare warmed runs.
- Scenario reports are intended for before/after comparisons on the same machine class and shell configuration.
