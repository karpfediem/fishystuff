# Current Map Performance Workstream

Last updated: 2026-03-22

This note is the short-lived execution log for the current browser performance push.

Use it to keep the latest diagnosis, priorities, and next cuts visible without rereading the whole task history.

## Latest follow-up

- The coarse `zone_mask` pyramid cut was a real startup win, but it introduced an edge-coverage regression because parent tiles on partial east/south edges were being emitted as full-size `512x512` images.
- Those coarse edge parents now preserve their actual occupied dimensions before downsampling, which keeps the visible mask aligned with the canonical map bounds.
- The local rebuild and browser smoke/profile runs are clean again after regenerating the mask pyramid.
- Browser robustness bug: transient raster image EOF/load failures were previously cached as permanent `Failed` tiles, so the runtime would never retry them and could leave persistent holes until a full reload.
- Failed raster tiles are now retriable in the scheduler/residency path so temporary local CDN rebuild races can recover automatically.

## Current diagnosis

- The native Bevy harness is useful for subsystem attribution, but it does not reproduce the severe browser FPS collapse on its committed fixtures.
- The integrated browser profiler is the source of truth for user-visible regressions.
- Browser measurements show real raster/vector cost, and the first JS boundary cut removed routine host state pulls from the page-shell interaction path.
- The current blocker is that the browser profiler still cannot reach a raster-idle baseline before vector-layer capture. The hot path remains dominated by raster tile churn rather than bridge reads.
- The latest integrated runs show `zone_mask` and `minimap` staying busy for 10s+ before capture, which means vector regressions are still being measured on top of unresolved raster work.

## Current priorities

1. Keep the browser profiler honest.
   - Gate vector scenarios on pre-capture raster-idle checks.
   - Surface per-layer raster busy counts from the bridge snapshot so reports identify which layers are still active.
2. Investigate idle raster churn first.
   - Explain why `zone_mask` and `minimap` keep `pending`/`inflight` work alive in the steady state.
   - Prioritize request planning, desired-set, and residency churn before deeper vector optimization.
3. Keep the JS↔Wasm boundary cold on the hot path.
   - Preserve the `loader.js` no-pull-after-patch behavior.
   - Keep hot state ownership in Wasm and let the shell consume cached state plus semantic events.
4. Re-run browser reports and compare:
   - `host.wasm.state_reads`
   - `host.state_pull`
   - `host.handle_event`
   - `raster.update_tiles`
   - `raster.tile_entity_update`
   - per-layer `pendingCount` / `inflightCount`
   - `vector.layer_update`
5. Only after raster idle is under control, continue deeper vector optimization.

## Current plan

1. Land the profiling transparency batch:
   - snapshot layer runtime counts in the bridge contract
   - a cold-path `refreshCurrentStateNow()` host method for browser profiling only
   - browser profile scenarios that explicitly record whether raster was idle before capture
2. Use those measurements to identify why the default map view never settles.
   - start in `map/raster/runtime.rs`
   - inspect request planning in `map/raster/policy/requests.rs`
   - inspect default-view bounds/LOD behavior in `map/raster/policy/bounds.rs`
3. Optimize the raster steady state until the integrated vector scenarios start from an actually idle baseline.
   - first cut: add a coarse visible pyramid for `zone_mask`
   - keep exact hover/selection on `z=0` through the existing pick-probe path
4. Re-check whether bridge cost is still material once raster churn is reduced.
5. After the mask-pyramid cut, focus on the remaining `minimap` + residual `zone_mask` backlog until the browser scenarios can actually begin from idle.

## Latest measured result

First boundary cut: `loader.js` now projects local input-state patches and no longer forces a wasm state read after routine UI state changes.

Measured on the real browser DOM-toggle scenario (`vector_region_groups_dom_toggle`):

- Before:
  - `host.wasm.state_reads=2`
  - `host.state_pull total_ms=0.5`
  - `browser_action.completed_frames=45`
- After:
  - `host.wasm.state_reads=0`
  - `host.state_pull` absent
  - `browser_action.completed_frames=71`

Interpretation:

- The page shell is materially less chatty across the JS↔Wasm boundary for this interaction.
- The dominant browser cost is still raster/vector work, so this boundary cut is necessary but not sufficient.

Latest profiler hardening result:

- Direct bridge-enable scenario (`vector_region_groups_enable`):
  - `frame_avg_ms=20.26`
  - `pre_capture_raster_idle_timed_out=true`
  - busy raster layers: `zone_mask`, `minimap`
  - busy raster tiles before capture: `685`
- Real page-shell DOM-toggle scenario (`vector_region_groups_dom_toggle`):
  - `frame_avg_ms=18.34`
  - `pre_capture_raster_idle_timed_out=true`
  - busy raster layers: `zone_mask`, `minimap`
  - busy raster tiles before capture: `435`
- Dominant spans in both runs remained:
  - `raster.update_tiles`
  - `raster.tile_entity_update`
  - `vector.layer_update`

Interpretation:

- The current vector browser slowdown is still confounded by unresolved raster streaming or residency churn that exists before the vector action.
- The next optimization target is not generic bridge work; it is the raster steady-state path that should already be idle before the vector action.
- The most likely first win is reducing whole-world `zone_mask` rendering cost at the default full-map view, because hover already has an explicit full-resolution pick path.

Latest optimization result:

- Added a coarse visible pyramid for `zone_mask` and allowed exact-pixel pick layers to use coarser manifest levels for visible coverage only.
- The build pipeline now regenerates `zone_mask` level-0 tiles from `zones_mask_v1.png` when needed and stages the finished mask pyramid atomically.
- Measured on integrated browser scenarios:
  - `vector_region_groups_dom_toggle`
    - `frame_avg_ms: 18.34 -> 6.96`
    - `frame_p95_ms: 29.7 -> 16.4`
    - `pre_capture_busy_raster_tiles: 435 -> 118`
    - `raster.update_tiles total_ms: 746.1 -> 141.9`
    - `raster.tile_entity_update total_ms: 712.1 -> 115.5`
  - `vector_region_groups_enable`
    - `frame_avg_ms: 20.26 -> 6.45`
    - `frame_p95_ms: 34.8 -> 13.5`
    - `pre_capture_busy_raster_tiles: 685 -> 125`
    - `raster.update_tiles total_ms: 869.2 -> 146.1`
    - `raster.tile_entity_update total_ms: 827.4 -> 116.0`

Interpretation:

- The mask pyramid was a real hotspot fix, not noise.
- The browser path is still not fully idle before vector capture, but the residual raster backlog is now much smaller.
- With raster startup pressure reduced, vector build/update cost is now easier to see and measure honestly.

## Canonical browser scenarios right now

- Startup:
  - `tools/scripts/map-browser-profile.sh load_map`
- Direct bridge vector enable:
  - `tools/scripts/map-browser-profile.sh vector_region_groups_enable`
- Real page-shell DOM toggle:
  - `tools/scripts/map-browser-profile.sh vector_region_groups_dom_toggle`

## Expected direction of travel

- `host.wasm.state_reads` should trend toward cold-path-only usage.
- `host.state_pull` should disappear from routine UI interactions.
- Vector browser scenarios should begin from `pre_capture_raster_idle_timed_out=false` or at least from much smaller busy raster counts.
- JS should batch intent in, and mostly react to outbound semantic events rather than polling.
- If the browser still collapses after boundary cleanup, the next likely cause is render/raster/GPU churn rather than bridge shape.

## Explicit non-goals for this phase

- Do not rewrite the whole bridge in one pass.
- Do not chase cold-path serialization or mount-only work before hot interaction paths are cleaned up.
- Do not claim wins without updated browser measurements.
