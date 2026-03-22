# Initial Map Hotspot Analysis

This note captures the first measured hotspot pass before further optimization work.

## Scope

The goal here is attribution, not fixes.

Reports were collected from:

- native deterministic harness scenarios
- integrated browser profiling against the real `/map` page

The browser reports are the source of truth for the user-visible vector-layer slowdown because the native fixture runs do not reproduce that cliff.

## Commands

Native:

```bash
tools/scripts/perf-run-scenario.sh raster_2d_pan_zoom /tmp/perf-raster-nav.json
tools/scripts/perf-run-scenario.sh vector_region_groups_enable /tmp/perf-vector-enable-native.json
tools/scripts/perf-run-scenario.sh terrain3d_enter_and_orbit /tmp/perf-terrain-orbit-native.json
```

Browser:

```bash
tools/scripts/map-browser-profile.sh load_map /tmp/map-browser-load-map.json
tools/scripts/map-browser-profile.sh vector_region_groups_enable /tmp/map-browser-vector.json
```

## Measured Findings

### Native Bevy is currently cheap on the committed fixtures

- `raster_2d_pan_zoom`
  - frame avg: `0.149 ms`
  - frame p95: `0.432 ms`
  - top span: `raster.update_tiles total_ms=0.160`
- `vector_region_groups_enable`
  - frame avg: `0.155 ms`
  - frame p95: `0.396 ms`
  - top span: `vector.layer_update total_ms=0.622`
  - counters: `vector.cache_hits=446`, `vector.cache_misses=4`
- `terrain3d_enter_and_orbit`
  - frame avg: `0.135 ms`
  - frame p95: `0.272 ms`
  - top spans:
    - `raster.update_tiles total_ms=4.406`
    - `raster.desired_tile_set_build total_ms=2.084`
    - `terrain.visible_chunk_computation total_ms=0.733`

Conclusion:

- The native harness does not explain a browser drop from high FPS to `10-20 FPS`.
- The current committed vector fixture is too small to make native vector processing expensive.

### Browser startup is dominated by one-time mount and initial raster work

Observed from `load_map`:

- frame avg: `20.220 ms`
- frame p95: `96.100 ms`
- top spans:
  - `host.mount total_ms=414.0`
  - `raster.update_tiles total_ms=41.4`
  - `raster.tile_entity_update total_ms=36.4`
  - `bridge.emit.dispatch total_ms=4.3`
  - `host.emit total_ms=4.1`

Interpretation:

- `host.mount` is startup-only and should not be treated as the steady-state navigation bottleneck.
- Initial raster setup is the main steady-state startup cost that remains after mount.
- Bridge costs are measurable but small relative to startup raster work.

### Browser vector enable does reproduce a meaningful degradation

Observed from `vector_region_groups_enable`:

- requested capture: `180` animation frames
- completed before timeout: `72`
- `browser_action.frame_wait_timed_out=true`
- frame avg: `12.297 ms`
- frame p95: `22.000 ms`

Top spans:

- `raster.update_tiles total_ms=568.9`
- `raster.tile_entity_update total_ms=538.3`
- `vector.layer_update total_ms=83.2`
- `vector.geojson_parse total_ms=26.5`
- `raster.desired_tile_set_build total_ms=9.1`
- `raster.visible_tile_computation total_ms=6.2`

Counters:

- `vector.cache_hits=102`
- `vector.cache_misses=40`
- `raster.requests_started=432`

Interpretation:

- The slowdown is real in the integrated browser path.
- The dominant measured CPU hotspot is not bridge work. It is raster churn, especially `raster.update_tiles` and its `raster.tile_entity_update` subpath.
- `vector.layer_update` is significant, but it is clearly behind raster work in this run.
- `vector.geojson_parse` is a one-shot activation cost, not the main steady-state per-frame cost.
- The vector enable path also correlates with substantial raster activity (`raster.requests_started=432`), which suggests the vector-layer activation is invalidating or amplifying raster work.

### Bridge and JS host costs are visible and currently not the primary bottleneck

For the browser vector run:

- top host spans:
  - `host.patch_flush total_ms=1.3`
  - `host.set_state total_ms=0.1`
- top bridge spans:
  - `bridge.snapshot_sync total_ms=1.9`
  - `bridge.patch_json_parse total_ms=1.3`
  - `bridge.patch_ingest total_ms=0.8`
  - `bridge.state_apply total_ms=0.4`

Conclusion:

- The integrated setup is now transparent enough to measure bridge and host work.
- Current evidence does **not** support treating the bridge as the first optimization target.

## Important Caveat

The browser vector report completed only `72 / 180` requested animation frames before timeout even though the measured Bevy frame average was `12.297 ms`.

That implies some meaningful wall-clock loss is happening outside the instrumented CPU spans. In the local Chromium run, stderr also reported repeated GPU stalls:

- `GPU stall due to ReadPixels`

So the current browser slowdown likely has two parts:

1. real CPU-side raster and vector work inside the map runtime
2. browser/WebGL/GPU stall time outside the currently instrumented Bevy spans

This means the span totals are actionable, but they are not the whole story.

## Prioritized Optimization Order

1. `raster.update_tiles`
   - especially the `raster.tile_entity_update` subpath under vector-layer activation
2. Raster invalidation/request churn triggered when vector layers are enabled
   - investigate why enabling `region_groups` causes `raster.requests_started=432`
3. `vector.layer_update`
   - reduce steady-state vector work after the initial activation
4. `vector.geojson_parse`
   - only after steady-state raster churn is reduced, because parse is mostly one-shot
5. Browser/WebGL stall investigation
   - likely needs a Chrome trace / Perfetto pass focused on GPU/compositor behavior

## Not First-Priority Right Now

- Bridge patch ingest / state apply
- JS host event dispatch
- Terrain chunk generation
- Native microbenchmark-driven vector algorithm rewrites

Those may matter later, but the current integrated browser data does not justify starting there.
