# Current Map Performance Workstream

Last updated: 2026-03-22

This note is the short-lived execution log for the current browser performance push.

Use it to keep the latest diagnosis, priorities, and next cuts visible without rereading the whole task history.

## Current diagnosis

- The native Bevy harness is useful for subsystem attribution, but it does not reproduce the severe browser FPS collapse on its committed fixtures.
- The integrated browser profiler is the source of truth for user-visible regressions.
- Browser measurements show real raster/vector cost, but the JS shell and bridge still matter architecturally because the page shell has been treating the bridge like a synchronous query service.
- The main browser shell smell is `loader.js` pulling full bridge state after local UI patches instead of projecting local input state and waiting for semantic events.

## Current priorities

1. Slim the hot JS↔Wasm boundary in the page shell.
   - Stop pull-after-patch behavior in `site/assets/map/loader.js`.
   - Keep hot state ownership in Wasm and let the shell consume cached state plus semantic events.
2. Measure the real integrated shell path, not just direct `FishyMapBridge.setState()` calls.
   - Prefer DOM-driven browser scenarios for UI-triggered regressions.
3. Re-run browser reports and compare:
   - `host.wasm.state_reads`
   - `host.state_pull`
   - `host.handle_event`
   - `raster.update_tiles`
   - `raster.tile_entity_update`
   - `vector.layer_update`
4. Only after the boundary is quieter, continue deeper raster/vector optimization.

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
- JS should batch intent in, and mostly react to outbound semantic events rather than polling.
- If the browser still collapses after boundary cleanup, the next likely cause is render/raster/GPU churn rather than bridge shape.

## Explicit non-goals for this phase

- Do not rewrite the whole bridge in one pass.
- Do not chase cold-path serialization or mount-only work before hot interaction paths are cleaned up.
- Do not claim wins without updated browser measurements.
