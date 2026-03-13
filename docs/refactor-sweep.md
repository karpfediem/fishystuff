# Refactor Sweep

## Scope

This note covers the repo-root Rust workspace and the checked-in map/site bridge artifacts that interact with it. It is intentionally brief and should track the current codebase, not an idealized redesign.

## Workspace Inventory

| Crate | Role | Category | Notes |
| --- | --- | --- | --- |
| `fishystuff_api` | Shared request/response DTOs, ids, API versioning, error envelope | Shared contract | Stable boundary for server and browser/WASM code. |
| `fishystuff_client` | Thin API client wrapper used by WASM/browser-side code | Shared runtime adapter | Should stay transport-focused and rendering-agnostic. |
| `fishystuff_core` | Shared math, masks, terrain primitives, coordinate/tile helpers | Shared domain core | Correct home for canonical map geometry and transform math. |
| `fishystuff_config` | Lightweight config file parser for shared CLI/server settings | Shared support | Small, but still carries some legacy path fields. |
| `fishystuff_store` | SQLite-backed event/water tile storage | Tooling/offline data | Runtime server no longer uses it; still used by analytics + ingest tooling. |
| `fishystuff_zones_meta` | CSV/Dolt loaders for zone metadata | Tooling/shared data access | Shared by offline analytics and ingest. |
| `fishystuff_analytics` | Offline SQLite analytics and JSON helpers for zone stats/effort | Tooling/offline analytics | Overlaps conceptually with server-side MySQL analytics logic. |
| `fishystuff_server` | Axum/Tower HTTP server, state, routes, MySQL/Dolt SQL store | Runtime server/API | Main runtime boundary for data access. |
| `fishystuff_ingest` | Ranking import, mask indexing, region-group import, offline diagnostics | Tooling/offline pipeline | Large CLI entrypoint mixes many unrelated commands. |
| `fishystuff_tilegen` | Tile and terrain pyramid generation binaries | Tooling/offline pipeline | Clear tooling role, but terrain binary is oversized. |
| `fishystuff_dolt_import` | XLSX -> Dolt import pipeline | Tooling/offline pipeline | Single large binary with parsing + import orchestration together. |
| `fishystuff_ui_bevy` | Bevy WASM map app, browser bridge, rendering/runtime systems | Runtime rendering | Largest concentration of module-size and state-boundary issues. |

Generated-artifact-adjacent paths outside crates:

- `site/assets/map/loader.js`
- `site/assets/map/map-host.js`
- `site/assets/map/ui/fishystuff.css`
- `data/cdn/public/map/*`

`site/assets/map/loader.js` and `site/assets/map/map-host.js` are browser-host source files. `site/assets/map/ui/fishystuff.css` is a copied UI stylesheet, and the hashed wasm/js runtime bundle now lives under `data/cdn/public/map/`.

## Boundary Analysis

### `fishystuff_api`

- Should own: stable DTOs, ids, small enums, wire-format errors.
- Currently owns: mostly that.
- Blur: low. Keep runtime policy and transform logic out.

### `fishystuff_client`

- Should own: transport, path building, error decoding.
- Currently owns: that, plus browser default-base-URL policy.
- Blur: small. The browser default URL policy is acceptable, but this crate should not accrete page or Bevy state logic.

### `fishystuff_core`

- Should own: canonical map geometry, tile math, terrain chunk math, shared pure transforms.
- Currently owns: core math, masks, terrain, map-to-water transforms, coordinate helpers.
- Blur: canonical map/world geometry is duplicated in `fishystuff_ui_bevy::map_space::world`; some transform formulas are still reimplemented in tooling.

### `fishystuff_server`

- Should own: server config, app state, thin routes, store-backed queries, runtime-only diagnostics.
- Currently owns: that, but `store/dolt_mysql.rs` also owns layer parsing, fish catalog normalization, synthetic revision hashing, Gaussian blur/statistics helpers, RNG, schema-error mapping, and response-shaping details.
- Blur: DB access and analytics/transformation helpers are collapsed into one store file, which makes profiling and reasoning about query cost harder.

### `fishystuff_ui_bevy`

- Should own: Bevy app bootstrap, rendering, camera/spatial state, runtime caches, stable browser bridge ingestion/emission.
- Currently owns: that, but with very large files that mix setup, state models, input handling, rendering policy, request scheduling, and browser contract translation.
- Blur:
  - `bridge/host/{mod,emission/{mod,state,view,diagnostic},persistence/{mod,patches,layers,view}}.rs`, `bridge/host/input/{mod,queue,commands/{mod,view,fish,selection}}.rs`, `bridge/host/input/state/{mod,filters,layers,theme}.rs`, and `bridge/host/snapshot/{mod,filters/{mod,capabilities,layers,state},state/{mod,ui,interaction},view}.rs` now split the browser bridge boundary by responsibility, `plugins/api/state/{mod,bootstrap,filters,interaction,pending,catalog}.rs` now separates bootstrap, filter/display, interaction, pending-request, and fish-catalog state, and `plugins/api/requests/{mod,ensure,poll,apply,spawn,util}.rs` now separates request scheduling, response polling, response application, spawn helpers, and defaults/URL helpers, but the bridge still mutates runtime resources directly across those resources.
  - `map/layers/{mod,registry,runtime}.rs` now separate static layer descriptors from dynamic per-layer runtime state, but clip-mask and UI-facing layer state still span raster, mask, and bridge consumers.
  - `map/raster/policy/{mod,bounds,residency,requests}.rs`, `map/raster/cache/{mod,render/{mod,geometry,loaded,visibility},filters/{mod,compose,clip_mask/{mod,sample,revision}}}.rs`, and `map/raster/runtime.rs` now isolate raster ownership; load completion, visibility sync, tile geometry, clip-mask sampling, and clip-mask revision tracking have separate homes, but the raster diagnostics/state boundary is still broader than it should be.
  - `map/terrain/{mode,runtime}.rs` now separate terrain mode activation from chunk/drape runtime, and `map/terrain/runtime/{camera/{mod,startup,controls,estimate,tests},manifest,chunks,drape/{mod,chunk_aligned,raster,mesh},diagnostics}.rs` now isolate terrain camera boot, control math, view estimation, regression tests, manifest lifecycle, chunk scheduling/builds, drape gating, chunk-aligned drapes, raster drapes, terrain-height mesh sampling, and terrain metrics, but the remaining terrain runtime still spreads across height-tile/debug helpers and the plugin shell.
  - `plugins/points/{loading,query/{mod,state,refresh,evidence},render}.rs` now split event snapshot polling, point-query state, local filtering, evidence-zone derivation, and marker rendering, but the subsystem still depends on patch/fish filter resources and drives raster evidence masking indirectly.
  - `plugins/ui/{scroll,search,panel}.rs`, `plugins/ui/setup/{mod,panel_shell,patch_shell,search_shell,styles}.rs`, and `plugins/ui/patches/{mod,selection,dropdown,slider}.rs` now split the legacy UI surface by concern, but it remains a broad in-engine control surface next to `map/ui_layers/{panel/{mod,view_rows,layer_rows,debug_row},controls/{mod,view,layers,debug},diagnostics}.rs`.
  - `map/ui_layers/{panel/{mod,view_rows,layer_rows,debug_row},controls/{mod,view,layers,debug},diagnostics}.rs` now isolates layer/debug panel shell construction, view rows, layer rows, view/layer/debug interactions, and diagnostics, but it still represents a separate in-engine UI surface that overlaps the site-hosted controls.

### `fishystuff_ingest`

- Should own: offline ingestion/indexing/import commands and their local data stores.
- Currently owns: that, but almost everything is in `main.rs`, including watermap fitting/debug helpers, zone stats plumbing, DB resolution, and command dispatch.
- Blur: command-specific logic is not separated, which makes it harder to test or profile individual ingestion stages.

### `fishystuff_analytics` and `fishystuff_store`

- Should own: offline SQLite analytics path only.
- Currently owns: exactly that, but they sit next to the runtime server and can look like an alternative runtime path unless documented clearly.
- Blur: server now computes similar analytics against MySQL/Dolt SQL independently, so shared intent is weaker than shared implementation.

### `fishystuff_tilegen` and `fishystuff_dolt_import`

- Should own: offline asset/data generation only.
- Currently owns: that, but large single binaries combine parsing, orchestration, and format-specific helpers.
- Blur: low runtime bleed, medium maintainability risk from god files.

## Risk Hotspots

Largest Rust files by line count during this audit:

- `api/fishystuff_server/src/store/dolt_mysql.rs` (~2966)
- `map/fishystuff_ui_bevy/src/map/terrain/runtime/camera/{mod,startup,controls,estimate,tests}.rs` (~16 / ~24 / ~129 / ~28 / ~267)
- `tools/fishystuff_ingest/src/main.rs` (~1935)
- `lib/fishystuff_analytics/src/lib.rs` (~1396)
- `tools/fishystuff_dolt_import/src/main.rs` (~1148)
- `tools/fishystuff_tilegen/src/bin/terrain_pyramid.rs` (~1065)
- `map/fishystuff_ui_bevy/src/plugins/ui/patches/{mod,selection,dropdown,slider}.rs` split the old ~723 LOC patch UI file into ~15 / ~157 / ~411 / ~162 LOC modules
- `map/fishystuff_ui_bevy/src/plugins/ui/setup/{mod,panel_shell,patch_shell,search_shell,styles}.rs` split the old ~773 LOC setup file into ~61 / ~142 / ~245 / ~226 / ~16 LOC modules
- `map/fishystuff_ui_bevy/src/map/raster/policy/{mod,bounds,residency,requests}.rs` split the old ~1433 LOC raster policy file into ~410 / ~279 / ~417 / ~369 LOC modules
- `map/fishystuff_ui_bevy/src/map/raster/cache/{mod,render/{mod,geometry,loaded,visibility},filters/{mod,compose,clip_mask/{mod,sample,revision}}}.rs` split the old ~1265 LOC raster cache file into ~363 / ~3 / ~110 / ~171 / ~116 / ~165 / ~99 / ~5 / ~245 / ~65 LOC modules
- `map/fishystuff_ui_bevy/src/map/terrain/{runtime,mode}.rs` split the old ~2434 LOC terrain runtime file into ~818 / ~367 LOC modules, with manifest, chunk, drape, and diagnostics helpers now moved into `map/fishystuff_ui_bevy/src/map/terrain/runtime/{manifest,chunks,drape/{mod,chunk_aligned,raster,mesh},diagnostics}.rs` (~354 / ~348 / ~83 / ~166 / ~138 / ~107 / ~59)
- `map/fishystuff_ui_bevy/src/bridge/host/snapshot/{mod,filters/{mod,capabilities,layers,state},state/{mod,ui,interaction},view}.rs` split the old ~378 LOC bridge snapshot file into ~96 / ~8 / ~15 / ~88 / ~51 / ~8 / ~18 / ~99 / ~31 LOC modules
- `map/fishystuff_ui_bevy/src/bridge/host/emission/{mod,state,view,diagnostic}.rs` split the old ~190 LOC emission file into ~9 / ~65 / ~37 / ~92 LOC modules
- `map/fishystuff_ui_bevy/src/bridge/host/persistence/{mod,patches,layers,view}.rs` split the old ~228 LOC persistence file into ~14 / ~76 / ~96 / ~58 LOC modules
- `map/fishystuff_ui_bevy/src/plugins/api/requests/{mod,ensure,poll,apply,spawn,util}.rs` split the old ~478 LOC requests file into ~101 / ~59 / ~118 / ~102 / ~93 / ~136 LOC modules
- `map/fishystuff_ui_bevy/src/plugins/api/state/{mod,bootstrap,filters,interaction,pending,catalog}.rs` split the old ~217 LOC API state file into ~18 / ~36 / ~70 / ~57 / ~18 / ~51 LOC modules
- `map/fishystuff_ui_bevy/src/bridge/contract/{mod,input,snapshot,events,normalize}.rs` split the old ~1018 LOC contract file into ~431 / ~322 / ~202 / ~39 / ~63 LOC modules
- `map/fishystuff_ui_bevy/src/map/ui_layers/{panel/{mod,view_rows,layer_rows,debug_row},controls/{mod,view,layers,debug},diagnostics}.rs` split the old ~501 LOC layer panel file into ~113 / ~244 / ~108 / ~106 LOC construction modules alongside ~15 / ~155 / ~83 / ~87 LOC control modules and the existing ~261 LOC diagnostics module
- `map/fishystuff_ui_bevy/src/plugins/points/{query/{mod,state,refresh,evidence},render,loading}.rs` split the old ~479 LOC points query file into ~158 / ~81 / ~187 / ~82 LOC query modules alongside the existing ~356 / ~70 LOC render and loading modules

Concrete hotspots:

- Duplicated transform/config/state logic
  - server-side query/analytics helpers in `fishystuff_server` overlap conceptually with offline analytics logic in `fishystuff_analytics`.
- Stale compatibility / legacy paths
  - `fishystuff_config::Paths::dolt_repo` remains as a legacy field for tooling; this is fine for ingest, but it should stay clearly tooling-only.
  - `fishystuff_store` + SQLite analytics remain valid offline tooling, but they should not read as a server runtime fallback.
- Dead/warning-producing code
  - `fishystuff_client` currently emits an `unused_mut` warning in `FishyClient::new`.
- Hard-to-reason-about state flow
  - Bevy tile residency/request scheduling now have explicit raster policy modules, and cache ownership is split into shared state, render sync, and filter/clip-mask modules, but clip-mask sampling and raster diagnostics still span several modules and remain a profiling target.
  - browser bridge schema is split, snapshot filter projection is separated into capabilities, filter-state projection, and layer-summary helpers, persistence helpers are split into patch, layer, and view domains, emission helpers are split into state, view, and diagnostic domains, API state is split into bootstrap, filter/display, interaction, pending-request, and fish-catalog modules, and API request logic is split into scheduling, polling, application, spawn helpers, and defaults/URL helpers, but host-side patch application and outbound snapshots still mutate and project broad runtime state directly.
  - server `AppState` is simple, but `DoltMySqlStore` obscures which cost comes from DB I/O vs local post-processing.
- Generated/source boundary blur
  - `site/assets/map/` now mixes browser-host source files with generated wasm outputs, so the source-vs-generated rule needs to stay explicit in AGENTS/build notes.

## Proposed Target Shape

### Crate Responsibilities

- Keep `fishystuff_api` contract-only.
- Keep `fishystuff_client` as a thin transport wrapper.
- Move canonical map geometry/transform formulas to `fishystuff_core` and have Bevy/tooling call into it.
- Keep `fishystuff_server` runtime-only:
  - routes thin
  - state explicit
  - store split by DB access vs pure post-processing helpers
- Keep `fishystuff_ui_bevy` focused on rendering/runtime:
  - bridge translation isolated from rendering systems
  - tile scheduling/cache policy isolated from tile mesh/render code
  - terrain runtime isolated from camera-mode state transitions where possible
- Keep SQLite analytics/tooling crates explicit as offline-only.

### Module Responsibilities

- `fishystuff_server::store`
  - `dolt_mysql.rs` should mainly hold store methods and DB-facing orchestration.
  - Pure helpers should move into focused child modules such as layer parsing, fish-catalog normalization, and stats utilities.
- `fishystuff_ui_bevy`
  - `browser_bridge`: split contract translation, emitted events, and color/theme parsing.
  - `plugins/tiles`: split manifest/cache state, residency planning, request scheduling, and render/clip-mask helpers.
  - `plugins/ui`: split patch controls, search/autocomplete, evidence panel, and shared widget math/state.
- `fishystuff_ingest`
  - split command modules by responsibility: import, indexing, watermap fitting/debug, zone-stats CLI glue.

### Shared vs Local

Share only when the logic is canonical and stable:

- Shared in `fishystuff_core`
  - map dimensions
  - world/map transform constants and pure formulas
  - tile math
  - terrain chunk math
- Shared in `fishystuff_api`
  - DTOs and ids only

Keep local:

- server cache state, request timeout policy, DB schema error mapping
- browser bridge patch/snapshot glue
- Bevy rendering/runtime cache internals
- ingest-only CLI argument resolution and import bookkeeping

## Refactor Priorities For This Sweep

1. Document the current shape and artifact boundaries.
2. Remove obvious stale/warning-producing code and keep runtime/server paths aligned to MySQL/Dolt SQL only.
3. Centralize canonical map geometry in `fishystuff_core`.
4. Split pure helper logic out of `fishystuff_server::store::dolt_mysql`.
5. Continue with the worst oversized Bevy/tooling files in follow-up focused passes rather than one risky rewrite.

## What Changed In This Sweep

- Removed `zonegen/map/` as a second map/static tree.
- Promoted `site/assets/map/loader.js` and `site/assets/map/map-host.js` to the canonical browser-host source files.
- Removed the server UI/static fallback so `fishystuff_server` stays API-focused instead of acting as a second site host.
- Dropped stale server config fields tied only to the removed static host path (`ui_dir`, `tiles_dir`, `base_map`, `zone_mask`).
