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
- Hover highlight no longer repaints zone-mask tile pixels on the CPU.
  - The highlight is now a material/shader effect on the zone-mask display tiles.
  - Exact lookup still determines which zone is hovered.

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
- Current measured result of the fixed display-tileset plus GPU hover path:
  - `zone_mask_hover_sweep` browser frame avg improved from the earlier `10.634 ms` baseline to `3.760 ms`
  - frame p95 improved from `45.0 ms` to `7.5 ms`
  - `raster.sync_visual_filters` dropped out of the top hover spans entirely
  - top hover spans are now `raster.update_tiles`, `raster.tile_entity_update`, and smaller bridge hover emission
- Current integrated vector activation result on the same zone-mask path:
  - `vector_region_groups_enable` frame avg is `7.629 ms`
  - top spans remain `vector.layer_update`, `vector.geojson_parse`, then raster update/render prep
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
  - hover highlight render path: `map/raster/cache/render/zone_mask_material.rs`
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
2. Keep hover highlight off the CPU raster compose path.
   - do not reintroduce per-hover tile image mutation for `zone_mask`
   - exact lookup should remain the semantic source; highlight should stay a render-time effect
3. Reduce the remaining visual raster working set.
   - the current pre-capture busy layers are still `zone_mask` and `minimap`
   - continue measuring busy-layer counts before and after each change
4. Reduce browser vector activation cost now that hover is no longer the main regression.
   - focus on `vector.layer_update`
   - treat `vector.geojson_parse` as a separate one-shot activation cost
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
5. Attack the next measured hotspot after hover stabilization.
   - current top activation hotspot: `vector.layer_update`
   - current remaining raster source: `minimap`
6. Only after the single-threaded pipeline is clean, revisit worker/thread options.

## Non-goals for this phase

- Do not wait on future Bevy web multithreading.
- Do not reintroduce more tile-pyramid work.
- Do not go back to the direct full-image `zone_mask` path.
- Do not let exact hover/click depend on display-tile residency again.
- Do not make performance claims without a browser or native profiling run.
