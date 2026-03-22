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

## Current diagnosis

- The browser slowdown is not just "Bevy is slow". The hotter architectural problem is coupling exact semantics to the visual raster tile cache.
- Exact hover/click work should not depend on whether the display tiles for that pixel happen to be resident.
- Visual transport format and semantic lookup format must be treated as separate concerns.
- The first stable target is:
  - exact lookup is cheap, bounded, and independent
  - visual raster work is measured and bounded separately
  - bridge work stays coarse and batched

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

Current generated lookup asset:

- `/images/exact_lookup/zone_mask.v1.bin`
- size: `1,790,476` bytes
- dimensions: `11560x10540`
- row segments: `291,382`

## Current priorities

1. Keep exact semantics off the visual raster path.
   - no reintroduction of pick-probe tile fetches
   - exact hover/click should stay available even when visual raster is still converging
2. Redesign visual zone-mask display separately.
   - evaluate a non-pick visual path such as a single-image overlay or a simpler bounded visual source
   - do not mix this back into semantic lookup
3. Bound the visual raster working set.
   - visible ring
   - prefetch ring
   - upload budget per frame
   - cancellation / reprioritization
4. Keep the profiling surface stable across backends.
   - measure the same named stages whether decode runs on the main thread, in JS workers, or in future wasm threads
5. Record future web-threading constraints, but do not block current optimization work on them.

## Current plan

1. Land the exact-lookup split and keep it measured.
2. Measure browser interaction again with exact lookup no longer coupled to raster residency.
3. Simplify the visual zone-mask path without bringing back tile-pyramid complexity.
4. Add explicit stage measurements for:
   - exact lookup load
   - exact lookup sample
   - visual tile fetch/decode
   - visual tile upload
   - bridge event flush
5. Only after the single-threaded pipeline is clean, revisit worker/thread options.

## Non-goals for this phase

- Do not wait on future Bevy web multithreading.
- Do not reintroduce more tile-pyramid work.
- Do not let exact hover/click depend on display-tile residency again.
- Do not make performance claims without a browser or native profiling run.
