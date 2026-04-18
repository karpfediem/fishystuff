# Current Map Performance Workstream

Last updated: 2026-04-18

This note keeps the latest direction visible without rereading the full task history.

## Current baseline

- Treat single-threaded Wasm as the production baseline.
- Do not assume future Bevy web multithreading will rescue the current data path.
- Keep the pipeline backend-neutral so later worker/thread backends can slot in without changing the measurement interface.

## Latest pivot

- The zone-mask pyramid / retry branch was rolled back.
- The map no longer uses raster pick-probe requests for hover or click on the exact zone mask.
- The canonical zone-mask field asset is now built from the source PNG and loaded directly into Wasm memory.
- `zone_mask`, `region_groups`, and `regions` now use the same field-backed runtime shape.
- There is no separate zone-mask raster tileset anymore.
- The direct full-image `zone_mask` experiment was removed.
  - It caused WebGL upload failures and was not viable on the browser path.
- The visual `minimap` now uses a map-space display pyramid:
  - `/images/tiles/minimap_visual/v1/tileset.json`
  - logical tile size: `512`
  - levels: `z=0..2`
  - only the finest display level keeps source-equivalent minimap detail
  - finest-level output textures are now about `1542px` wide instead of `3855px`, so highest zoom keeps detail without giant per-tile browser decodes
  - every minimap level is now sampled directly from the raw minimap source tiles in canonical map space
  - parent LODs are no longer built by stitching child PNG quadrants together
  - that removes the partial-edge distortion/misalignment bug from the previous parent-composition path
  - parent levels now shrink by 2x per level (`1542 -> 771 -> 386`) to keep browser memory bounded
  - the old mushy final `1x1`, `2x2`, and `3x3` minimap levels were removed
  - this is a visual-quality tradeoff, not a pure perf win: the browser now never falls back to those very coarse minimap views
  - runtime LOD is now overridden specifically for minimap startup:
    - `target_tiles=16`
    - `hysteresis_hi=24`
    - `hysteresis_lo=8`
    - `margin_tiles=1`
    - no coarse pinning, no refine
    - `warm_margin_tiles=1`
    - `protected_margin_tiles=1`
    - `max_resident_tiles=128`
  - this keeps startup on coarse minimap levels, preserves a small edge ring while panning, and only reaches finest detail when zoom actually requires it
  - the old 128px source-space `minimap` pyramid is no longer the runtime visual path
  - the raw `rader_*` source tiles are remapped offline into canonical map-space display tiles during `build_map.sh`
  - the minimap rebuild guard now tracks both `tile_size_px` and maximum generated level so stale pyramids do not survive config changes
- The earlier custom GPU hover-highlight experiment was unstable and was backed out.
  - It caused a browser-side wgpu/WebGL panic and blank map output in the integrated shell.
- The current hover path is now correctness-first and field-backed:
  - the zone-mask field still determines which zone is hovered
  - the filter path is no longer called every 2D frame while idle
  - all hover transitions currently use the targeted CPU path
  - the hover highlight color is now bright green
  - clip-mask and evidence-filter cases still use the compose path where needed
- The 2D zoom-in clamp has been loosened again.
  - the current minimum zoom factor is `0.0025 * fit_scale`
  - this restores deeper zooming without changing the initial fit-to-world behavior

## Current diagnosis

- The browser slowdown is not just "Bevy is slow". A major past problem was coupling exact semantics to the visual raster tile cache.
- A concrete cache bug also existed in the raster runtime:
  - eviction only ran on LOD changes
  - panning at a fixed zoom could keep accumulating decoded raster tiles without ever trimming the cache
  - that pattern matches the recent minimap OOMs after exploration better than startup alone
- Exact hover/click work should not depend on whether any unrelated raster tiles happen to be resident.
- Static assets must resolve through the configured public CDN base just like tiles and GeoJSON.
- Visual transport format and semantic lookup format must be treated as separate concerns.
- For `zone_mask`, that split has now been removed at the transport level:
  - the canonical field asset is both the semantic source and the 2D visual source
  - the old separate visual tileset path is gone
- The same rule now applies to minimap transport:
  - the browser should not pay tens of thousands of tiny PNG decodes for a static visual layer
  - the right runtime shape is a map-space display pyramid with far fewer, larger tiles than the raw source set
  - high-zoom detail is a hard requirement, so minimap work must preserve source-equivalent finest-level density
- The first stable target is:
  - field-backed semantic lookup is cheap, bounded, and independent
  - raster work is measured and bounded separately where raster transport is still required
  - bridge work stays coarse and batched
- Current integrated vector activation result on the same zone-mask path:
  - the browser vector scenarios now explicitly isolate a single vector layer instead of accidentally inheriting both default-visible vector layers
  - the first measured fix stopped forcing `regions` active when `region_groups` is enabled, and stopped `vector.layer_update` from running every frame just because a layer stayed `Ready`
  - the main remaining vector bottleneck was not raw triangle count; it was mesh count
  - before the render-path change:
    - `region_groups` alone built `240` meshes for `240` features
    - `regions` alone built `1244` meshes for `1252` features
  - the current vector render path now uses per-vertex color in shared chunk meshes instead of one mesh/material bucket per unique feature color
  - after that change:
    - `region_groups` alone builds `1` mesh for `40351` triangles / `41074` vertices
    - `regions` alone builds `1` mesh for `108595` triangles / `113343` vertices
  - latest isolated browser results:
    - `vector_region_groups_enable`: `7.498 ms` avg / `18.4 ms` p95 -> `4.249 ms` avg / `12.0 ms` p95
    - `vector_region_groups_dom_toggle`: `8.283 ms` avg / `15.9 ms` p95 -> `4.244 ms` avg / `12.1 ms` p95
    - `vector_regions_enable`: `8.777 ms` avg / `21.8 ms` p95 -> `5.002 ms` avg / `10.3 ms` p95
  - current top vector spans are now `vector.layer_update`, `vector.geojson_parse`, then light residual raster work from the rest of the scene
- Current integrated minimap results:
  - coarse single-level minimap cut previously reached `load_map` avg `44.513 ms`, p95 `128.5 ms`
  - first full-detail `1024` minimap pyramid reached `minimap_enable` avg `27.981 ms`, p95 `87.1 ms`
  - full-detail-on-all-levels `1280` minimap pyramid reached `minimap_enable` avg `21.439 ms`, p95 `84.4 ms`
  - that full-detail-on-all-levels `1280` variant later hit a browser `rust_oom` during real map convergence and was backed out
  - current browser-safe `1280` pyramid with shrinking parent levels reaches `minimap_enable` avg `23.547 ms`, p95 `124.3 ms`
  - adding the minimap-specific startup LOD override drops `minimap_enable` further to `12.086 ms` avg, `108.6 ms` p95
  - that was `11.461 ms` faster than the parent-shrunk-only variant on `minimap_enable`, but it still left very large finest-level decodes
  - the first `512px` finest-level minimap pyramid plus real over-budget eviction reached `minimap_enable` avg `9.606 ms`, p95 `10.2 ms`
  - after replacing parent-quadrant composition with direct source sampling and removing the final `1x1` level, `minimap_enable` reached `8.645 ms` avg, `40.7 ms` p95
  - `minimap_pan_zoom` reached `2.758 ms` avg, `5.7 ms` p95
  - after also removing the `2x2` and `3x3` coarse levels, the current `z=0..2` pyramid reaches `minimap_enable` at `17.619 ms` avg, `124.8 ms` p95
  - the current `z=0..2` pyramid reaches `minimap_pan_zoom` at `4.539 ms` avg, `10.6 ms` p95
  - that is slower than the `z=0..4` direct-sampled variant, but visually cleaner at the farthest zooms
  - `raster.update_tiles` is now `499.9 ms` on `minimap_enable`
  - `raster.tile_entity_update` is now `470.5 ms` on `minimap_enable`
  - a longer `load_map` browser run completed and captured `101` frames at `17.049 ms` avg without renderer failure
  - the old raw `minimap/v1` runtime visual surface was `26,777` PNGs / about `815.8 MiB` on disk
  - the current `minimap_visual/v1` display pyramid is `665` PNGs / about `987 MiB` on disk
  - on-disk size is still large, but the finest-level decoded working set is now much safer because each top-level PNG is around `7–8 MiB` compressed instead of `30–43 MiB`
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
- `zone_mask`, `region_groups`, and `regions` now use field-backed runtime layers
  - build outputs live under `/fields/`
  - `zone_mask` no longer has a separate visual tileset path
- `minimap` visual rendering now uses a source-equivalent map-space display pyramid instead of the old 128px decode-heavy source pyramid
  - build output: `/images/tiles/minimap_visual/v1`
  - generator: `tools/fishystuff_tilegen/src/bin/minimap_display_tiles.rs`
  - runtime override: `map/layers/registry.rs`
  - startup LOD override: `target_tiles=16`, no refine, no coarse pinning, `margin_tiles=1`, `warm_margin_tiles=1`, `protected_margin_tiles=1`, `max_resident_tiles=128`
  - idle raster queue refresh now only runs when a layer still has blank visible tiles or failed entries to recover
  - minimap LOD selection now counts only the truly visible tiles; the one-tile request ring no longer pollutes hysteresis and freeze the minimap on a coarse level when zoomed in
  - the raster cache now evicts whenever it is actually over budget, not just when the chosen LOD changes
  - parent minimap levels are now direct source resamples, not stitched child quadrants
- Hover/click state updates in `plugins/mask.rs` are now deduplicated so unchanged hover samples do not churn the 2D raster path every frame
- Browser profiling now includes a `minimap_enable` scenario for minimap visibility regressions after startup
- Browser profiling now includes a `minimap_pan_zoom` scenario for exploration-time minimap regressions
- Browser profiling now includes a `zone_mask_hover_far_jumps` scenario for large-distance hover transitions
- Browser profiling now includes both `vector_region_groups_enable` and `vector_regions_enable` as isolated single-layer scenarios
- Browser profiling temp directories no longer cause false non-zero exits after successful runs

Current generated zone-mask field asset:

- `/fields/zone_mask.v1.bin`
- size: `1,790,476` bytes
- dimensions: `11560x10540`
- row segments: `291,382`

## Current priorities

1. Keep `zone_mask` on the canonical field-backed path.
   - no reintroduction of pick-probe tile fetches
   - no reintroduction of a separate visual tileset path
2. Restore a fast large-fanout hover path without regressing correctness.
  - the field asset should remain the semantic source
  - keep the filter path change-driven, not per-frame
  - the current CPU path is the correctness baseline for hover transitions
  - the next fast path must derive overlay coverage from exact zone semantics
3. Reduce the remaining visual raster working set.
   - the biggest remaining startup spans are still `raster.update_tiles` and `raster.tile_entity_update`
   - `minimap` is no longer using the pathological raw 128px decode surface, but it still participates in raster startup cost
   - the current browser-safe `512` finest-level pyramid keeps top-zoom detail while greatly reducing per-tile decode spikes
   - the minimap LOD override plus real over-budget eviction are now part of that guarantee; removing either one reintroduces browser-risky finest-tile accumulation
   - continue measuring busy-layer counts before and after each change
4. Reduce the remaining browser vector activation cost now that the draw-call cliff is fixed.
   - the per-feature mesh explosion is gone; keep `vector_mesh_count` low
   - current activation hotspots are `vector.layer_update` and `vector.geojson_parse`
   - the next vector work should focus on one-shot build/parse cost, not reintroducing per-feature draw buckets
   - host/bridge patch ingest is visible but still secondary in the current isolated vector scenarios
5. Keep the profiling surface stable across backends.
   - measure the same named stages whether decode runs on the main thread, in JS workers, or in future wasm threads
6. Record future web-threading constraints, but do not block current optimization work on them.

## Current plan

1. Keep the canonical field-backed `zone_mask` path measured and documented.
2. Keep `minimap` on the browser-safe `512` source-equivalent finest-level pyramid with `z=0..2` unless a replacement beats it with data.
3. Re-measure hover hotspots on the field-backed `zone_mask` path before optimizing further.
4. Reduce residual raster startup/backlog for `minimap` without breaking exact semantics elsewhere.
5. Add explicit stage measurements for:
   - field asset load
   - field sample
   - visual tile fetch/decode
   - visual tile upload
   - bridge event flush
6. Attack the next measured hotspot after the vector draw-call collapse.
  - current top activation hotspot: `vector.layer_update`
  - current top one-shot vector hotspot: `vector.geojson_parse`
  - current startup raster hotspot after the minimap cut: `raster.update_tiles` / `raster.tile_entity_update`
  - `minimap_enable` and `minimap_pan_zoom` are now the dedicated browser regression scenarios for that layer
7. Keep browser smoke/profile automation trustworthy.
  - `tools/scripts/map_browser_smoke.py` now ignores Chromium temp-profile cleanup races after successful runs
  - keep one-command local browser validation green while iterating
8. Only after the single-threaded pipeline is clean, revisit worker/thread options.

## Non-goals for this phase

- Do not wait on future Bevy web multithreading.
- Do not reintroduce the raw source-space 128px `minimap` pyramid.
- Do not reintroduce a second transport path for `zone_mask`.
- Do not let exact hover/click depend on raster-tile residency again.
- Do not make performance claims without a browser or native profiling run.
