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
- The visual `zone_mask` no longer streams hundreds of raster tiles on the browser hot path.
  - It now renders directly from the canonical `/images/zones_mask_v1.png` image as one async image asset.
  - Hover/click and exact clip semantics still come from the exact-lookup asset, not from the visual image.

## Current diagnosis

- The browser slowdown is not just "Bevy is slow". The hotter architectural problem is coupling exact semantics to the visual raster tile cache.
- Exact hover/click work should not depend on whether the display tiles for that pixel happen to be resident.
- Exact/static assets must resolve through the configured public CDN base just like tiles and GeoJSON.
  - Site-root relative URLs (`/images/...`) are wrong in the integrated site shell and break both direct-image display and exact-lookup hover.
- Visual transport format and semantic lookup format must be treated as separate concerns.
- The first stable target is:
  - exact lookup is cheap, bounded, and independent
  - visual raster work is measured and bounded separately
  - bridge work stays coarse and batched
- Current measured result of the direct `zone_mask` image path:
  - `vector_region_groups_enable` browser frame avg improved from `16.165 ms` to `6.206 ms`
  - frame p95 improved from `24.8 ms` to `13.5 ms`
  - pre-capture busy raster backlog improved from `422` tiles across `zone_mask` + `minimap` to `0`
  - `raster.update_tiles total_ms` improved from `768.4` to `17.3`

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
- `zone_mask` visual rendering now uses a direct static image path instead of the tile streamer
  - runtime: `map/raster/static_image.rs`
  - asset path: `/images/zones_mask_v1.png`
  - build staging keeps the canonical source image available under the CDN root
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
2. Keep `zone_mask` on the direct image path.
   - do not move it back into the raster tile streamer
   - keep direct-image and exact-lookup asset URLs on the configured CDN/public base path
   - if future devices hit texture-size constraints, split it deliberately rather than reviving tile churn
3. Reduce the remaining visual raster working set.
   - the next raster candidate is `minimap`, which is still tile-streamed
   - continue measuring busy-layer counts before and after each change
4. Reduce browser vector activation cost now that raster no longer dominates.
   - focus on `vector.layer_update`
   - treat `vector.geojson_parse` as a separate one-shot activation cost
5. Keep the profiling surface stable across backends.
   - measure the same named stages whether decode runs on the main thread, in JS workers, or in future wasm threads
6. Record future web-threading constraints, but do not block current optimization work on them.

## Current plan

1. Land the exact-lookup split and keep it measured.
2. Keep `zone_mask` on the direct image path and re-measure the integrated browser setup.
3. Simplify the remaining visual raster sources without bringing back tile-pyramid complexity.
4. Add explicit stage measurements for:
   - exact lookup load
   - exact lookup sample
   - visual tile fetch/decode
   - visual tile upload
   - bridge event flush
5. Attack the next measured hotspot after `zone_mask`.
   - current top steady-state browser hotspot: `vector.layer_update`
   - current remaining raster source: `minimap`
6. Only after the single-threaded pipeline is clean, revisit worker/thread options.

## Non-goals for this phase

- Do not wait on future Bevy web multithreading.
- Do not reintroduce more tile-pyramid work.
- Do not let exact hover/click depend on display-tile residency again.
- Do not make performance claims without a browser or native profiling run.
