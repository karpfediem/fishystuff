# Current Map Performance Workstream

Last updated: 2026-03-22

This note keeps the latest direction visible without rereading the full task history.

## Current baseline

- Treat single-threaded Wasm as the production baseline.
- Do not assume future Bevy web multithreading will rescue the current data path.
- Keep the pipeline backend-neutral so later worker/thread backends can slot in without changing the measurement interface.

## Latest pivot

- The zone-mask pyramid / retry branch was rolled back.
- Exact zone semantics are now separated from visual raster residency.
- The map no longer uses raster pick-probe requests for hover or click on the exact zone mask.
- A dedicated exact-lookup asset is now built from the canonical zone mask PNG and loaded directly into Wasm memory.
- The direct full-image `zone_mask` experiment was removed.
  - It caused WebGL upload failures and was not viable on the browser path.
- The visual `zone_mask` now uses a fixed display-tileset:
  - `/images/tiles/zone_mask_visual/v1/tileset.json`
  - tile size: `2048`
  - single level: `z=0`
  - exact hover/click and exact clip semantics still come from the exact-lookup asset, not from the visual tiles.
- The earlier custom GPU hover-highlight experiment was unstable and was backed out.
  - It caused a browser-side wgpu/WebGL panic and blank map output in the integrated shell.
- The current hover path is now hybrid:
  - exact lookup still determines which zone is hovered
  - the filter path is no longer called every 2D frame while idle
  - small hover transitions still use the targeted CPU span-delta path because it remains the fastest local-case path
  - larger hover transitions now switch to a per-tile shader overlay instead of rewriting tile textures
  - the shader overlay uses the layer's actual depth plus a small bias, so variable display order still works
  - clip-mask and evidence-filter cases still fall back to the compose path until the overlay path can support them directly

## Current diagnosis

- The browser slowdown is not just "Bevy is slow". The hotter architectural problem is coupling exact semantics to the visual raster tile cache.
- Exact hover/click work should not depend on whether the display tiles for that pixel happen to be resident.
- Exact/static assets must resolve through the configured public CDN base just like tiles and GeoJSON.
  - Site-root relative URLs (`/images/...`) are wrong in the integrated site shell and break both display tiles and exact-lookup hover.
- Visual transport format and semantic lookup format must be treated as separate concerns.
- The first stable target is:
  - exact lookup is cheap, bounded, and independent
  - visual raster work is measured and bounded separately
  - bridge work stays coarse and batched
- Current measured result of the current hybrid path:
  - browser smoke passes again and the map is visible
  - previous `zone_mask_hover_sweep` baseline was `9.653 ms` avg with `46.0 ms` p95
  - first hover optimization reduced `zone_mask_hover_sweep` to `4.635 ms` avg with `9.6 ms` p95
  - previous targeted hover path reduced `zone_mask_hover_sweep` to `3.026 ms` avg with `6.2 ms` p95
  - current hybrid hover path reduces `zone_mask_hover_sweep` further to `2.684 ms` avg with `4.5 ms` p95
  - new `zone_mask_hover_far_jumps` browser scenario is `2.905 ms` avg with `5.0 ms` p95
  - `raster.sync_visual_filters` dropped from `353.5 ms` total to `17.2 ms`, then to `13.8 ms`
  - `raster.update_tiles` is now `22.0 ms` total on `zone_mask_hover_sweep`
  - `raster.sync_visual_filters` is now `2.2 ms` total on `zone_mask_hover_sweep`
  - top hover spans are now `raster.update_tiles`, `raster.visible_tile_computation`, and `raster.desired_tile_set_build`
  - the shader path is stable in the integrated browser shell, but only engaged when the hover transition fans out beyond a tile threshold
- Current integrated vector activation result on the same zone-mask path:
  - latest `vector_region_groups_enable` frame avg is `7.835 ms`
  - top spans are `vector.layer_update`, `vector.geojson_parse`, then host/bridge patch ingest
- The browser bridge is measurable but not the current dominant cost in these runs.

## Current module split

Backend-neutral stages:

1. `tile_visibility`
   - pure visible-set logic
2. `tile_scheduler`
   - request prioritization and cancellation
3. `visual_tile_source`
   - display-oriented fetch/decode path
4. `exact_lookup`
   - semantic zone lookup path
5. `tile_cache`
   - bounded visual residency
6. `upload_budgeter`
   - GPU admission / per-frame upload limits
7. `pick_lookup`
   - world/map px to semantic id
8. `bridge_events`
   - coarse outbound notifications only

## What is implemented now

- Shared exact lookup domain type in `lib/fishystuff_core`:
  - `ZoneLookupRows`
- Dedicated asset builder in `tools/fishystuff_tilegen`:
  - `zone_lookup`
- Build integration in `tools/scripts/build_map.sh`
- Map-side exact lookup cache in `map/exact_lookup.rs`
- Hover/click path in `plugins/mask.rs` now samples the exact lookup asset instead of queueing raster pick probes
- Old raster pick-probe request path was removed from `map/streaming.rs` and `map/raster/policy/requests.rs`
- `zone_mask` visual rendering now uses fixed display chunks instead of the old visual/semantic tile coupling
  - build output: `/images/tiles/zone_mask_visual/v1`
  - runtime override: `map/layers/registry.rs`
- Hover/click state updates in `plugins/mask.rs` are now deduplicated so unchanged hover samples do not churn the 2D raster path every frame
- Raster visual filtering in `map/raster/runtime.rs` now reruns on real state changes instead of every Map2D frame
- Zone-mask visual tiles now keep row-span lookup data so hover-only transitions can restore/apply just the affected zone runs
- Hover-only visual updates now use a zone-to-tile index so only tiles containing the old/new hovered zones are touched
- Larger hover fanout now uses a per-tile `Material2d` overlay driven by the loaded zone-mask texture instead of CPU texture rewrites
- Browser profiling now includes a `zone_mask_hover_far_jumps` scenario for large-distance hover transitions
- Browser profiling temp directories no longer cause false non-zero exits after successful runs

Current generated lookup asset:

- `/images/exact_lookup/zone_mask.v1.bin`
- size: `1,790,476` bytes
- dimensions: `11560x10540`
- row segments: `291,382`

## Current priorities

1. Keep exact semantics off the visual raster path.
   - no reintroduction of pick-probe tile fetches
   - exact hover/click should stay available even when visual raster is still converging
2. Reduce the remaining hover cost without regressing the local fast path.
  - exact lookup should remain the semantic source
  - keep the filter path change-driven, not per-frame
  - keep the CPU span/delta path for small hover transitions
  - use the shader overlay only where it is measurably better
  - next tuning knob is the tile-fanout threshold that switches between the two
3. Reduce the remaining visual raster working set.
   - the current pre-capture busy layers are still `zone_mask` and `minimap`
   - continue measuring busy-layer counts before and after each change
4. Reduce browser vector activation cost now that the blank-screen regression is gone.
   - focus on `vector.layer_update`
   - treat `vector.geojson_parse` as a separate one-shot activation cost
   - host/bridge patch ingest is now visible enough to profile alongside vector work
5. Keep the profiling surface stable across backends.
   - measure the same named stages whether decode runs on the main thread, in JS workers, or in future wasm threads
6. Record future web-threading constraints, but do not block current optimization work on them.

## Current plan

1. Keep the exact-lookup split and fixed display-tileset path measured.
2. Keep `zone_mask` on the fixed visual chunk path, not the full-image path and not the old pyramid branch.
3. Reduce residual raster startup/backlog for `zone_mask` + `minimap` without breaking exact semantics.
4. Add explicit stage measurements for:
   - exact lookup load
   - exact lookup sample
   - visual tile fetch/decode
   - visual tile upload
   - bridge event flush
5. Attack the next measured hotspot after the hover-path split.
  - current top activation hotspot: `vector.layer_update`
  - current top hover hotspots: `raster.update_tiles`, `raster.visible_tile_computation`, and `raster.desired_tile_set_build`
  - current remaining raster source: `minimap`
6. Only after the single-threaded pipeline is clean, revisit worker/thread options.

## Non-goals for this phase

- Do not wait on future Bevy web multithreading.
- Do not reintroduce more tile-pyramid work.
- Do not go back to the direct full-image `zone_mask` path.
- Do not let exact hover/click depend on display-tile residency again.
- Do not make performance claims without a browser or native profiling run.
