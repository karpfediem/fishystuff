# Web Threading Readiness

Last updated: 2026-03-22

This memo records what the map pipeline must fix regardless of future Bevy/Wasm threading, what threading would help later, and what should explicitly not wait for it.

## Current assumption

Production web performance must be acceptable on single-threaded Wasm.

Why:

- Bevy WebAssembly multithreading tracking issue: <https://github.com/bevyengine/bevy/issues/4078>
- Web task-model issue for `bevy_tasks`: <https://github.com/bevyengine/bevy/issues/20404>
- `bevy_tasks` docs: <https://docs.rs/bevy_tasks/latest/bevy_tasks/>
- Shared-memory browser requirements: <https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Global_Objects/SharedArrayBuffer>

## What must be fixed regardless of threading

1. Exact semantic lookup must not depend on visual raster residency.
   - Hover and click need a direct data path.
2. Visual transport format and semantic lookup format must be different concerns.
   - Display PNGs are not a good canonical semantic store.
3. Visual raster residency must be bounded.
   - Visible ring, prefetch ring, upload budget, cancellation.
4. The JS↔Wasm boundary must stay coarse.
   - Batched commands in, compact events out.
5. Each expensive stage must be measurable and swappable.
   - Visibility, scheduling, fetch, decode, upload, lookup, bridge.

## What threading would help later

If future worker/thread backends become viable, they should help with:

- image decode / transcode
- GeoJSON parse and triangulation
- clustering and index rebuilds
- terrain chunk decode / mesh generation
- bulk preprocessing for upload

Those are accelerators, not prerequisites.

## What should not wait for threading

- exact zone hover/click
- request budgeting
- cache residency policy
- upload throttling
- bridge simplification
- semantic / visual data-format split

## Current backend-neutral stage split

1. `tile_visibility`
2. `tile_scheduler`
3. `visual_tile_source`
4. `exact_lookup`
5. `tile_cache`
6. `upload_budgeter`
7. `pick_lookup`
8. `bridge_events`

These stages should stay valid whether execution remains single-threaded, moves to JS workers, or eventually gains proper wasm thread support.

## Current implementation status

Done:

- exact semantic lookup moved onto a dedicated asset path
- shared lookup representation lives in `lib/fishystuff_core`
- map hover/click no longer queue raster pick-probe requests

Not done yet:

- visual zone-mask display still uses the older raster path
- visual tile residency and upload work still need a cleaner bounded design
- browser-integrated measurements need a second pass after the exact-lookup split

## Future deployment constraints

If web threads are revisited later, validate:

- secure context
- cross-origin isolation
- `Cross-Origin-Embedder-Policy`
- `Cross-Origin-Opener-Policy`
- cross-origin asset compatibility
- popup / auth / payment flows that may be disrupted by isolation headers
