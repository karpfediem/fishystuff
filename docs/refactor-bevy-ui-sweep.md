# Refactor Bevy UI Sweep

## Scope

This note is specific to `map/fishystuff_ui_bevy`. It records the main structural pain points that motivated this sweep, the target shape for incremental cleanup, and the concrete refactors that have already landed. It is not a redesign document.

## Current Subsystem Inventory

### App bootstrap

- Files:
  - `src/lib.rs`
  - `src/app.rs`
  - `src/main.rs`
  - `src/plugins/mod.rs`
- Role:
  - WASM entrypoint
  - Bevy default plugin setup
  - plugin ordering

### Browser bridge / JS interop

- Files:
  - `src/bridge/host/{mod,emission/{mod,state,view,diagnostic},persistence/{mod,patches,layers,view}}.rs`
  - `src/bridge/host/input/{mod,queue,commands/{mod,view,fish,selection}}.rs`
  - `src/bridge/host/input/state/{mod,filters,layers,theme}.rs`
  - `src/bridge/host/snapshot/{mod,filters/{mod,capabilities,layers,state},state/{mod,ui,interaction},view}.rs`
  - `src/bridge/contract/{mod,input,snapshot,events,normalize}.rs`
  - `src/bridge/theme.rs`
- Role:
  - browser <-> WASM state patch contract
  - emitted lifecycle/view/hover/diagnostic events
  - restore/apply commands
  - theme color parsing

### Shared map state / registry

- Files:
  - `src/map/layers/{mod,registry,runtime}.rs`
  - `src/map/ui_layers/{mod,panel/{mod,view_rows,layer_rows,debug_row},controls/{mod,view,layers,debug},diagnostics}.rs`
  - `src/plugins/api/{mod,requests,fish,state/{mod,bootstrap,filters,interaction,pending,catalog}}.rs`
- Role:
  - layer descriptor registry
  - runtime layer visibility/opacity/status state
  - map metadata/bootstrap loading
  - UI-facing state used by both HTML bridge and remaining in-engine UI

### Coordinate spaces / transforms

- Files:
  - `src/map/spaces/*.rs`
  - consumers across `plugins/*`, `map/vector/*`, `map/terrain/*`
- Role:
  - map/world points and rects
  - affine transforms
  - layer-to-map/world transforms

### 2D camera / input

- Files:
  - `src/map/camera/map2d.rs`
  - `src/map/camera/mode.rs`
  - `src/plugins/camera.rs`
  - `src/plugins/input.rs`
- Role:
  - camera spawning
  - initial fit logic
  - 2D pan/zoom controls
  - shared camera activation and mode transition logic

### 3D terrain camera / terrain runtime

- Files:
  - `src/map/camera/terrain3d.rs`
  - `src/map/terrain/{mode,runtime}.rs`
  - `src/map/terrain/runtime/{manifest,chunks,camera/{mod,startup,controls,estimate,tests},drape/{mod,chunk_aligned,raster,mesh},diagnostics}.rs`
  - `src/map/terrain/{chunks,drape,height_tiles,materials,mesh}.rs`
- Role:
  - 3D camera state and control math
  - terrain mode activation and camera/lighting application
  - terrain manifest/chunk runtime
  - drape rendering
  - height tile sampling
  - terrain diagnostics

### Raster layer streaming

- Files:
  - `src/plugins/raster.rs`
  - `src/map/raster/manifest.rs`
  - `src/map/raster/policy/{mod,bounds,residency,requests}.rs`
  - `src/map/raster/cache/{mod,render/{mod,geometry,loaded,visibility},filters/{mod,compose,clip_mask/{mod,sample,revision}}}.rs`
  - `src/map/raster/runtime.rs`
  - `src/map/streaming.rs`
- Role:
  - manifest loading
  - visible set computation
  - request scheduling
  - cache eviction
  - raster render entity updates
  - clip-mask sampling helpers

### Vector layers

- Files:
  - `src/plugins/vector_layers.rs`
  - `src/map/vector/*.rs`
- Role:
  - GeoJSON loading
  - incremental build/triangulation
  - vector mesh cache
  - render entity sync

### Events snapshot / local filtering

- Files:
  - `src/map/events/{snapshot,index,cluster}.rs`
  - `src/plugins/points/{mod,loading,query/{mod,state,refresh,evidence},render}.rs`
- Role:
  - snapshot polling
  - local spatial indexing
  - viewport/time/fish filtering
  - point clustering and render state

### Diagnostics / overlays

- Files:
  - `src/plugins/diagnostics.rs`
  - `src/map/ui_layers/{diagnostics,controls}.rs`
  - diagnostics portions of `src/map/raster/{cache,policy,runtime}.rs`
  - `src/map/terrain/runtime/diagnostics.rs`
- Role:
  - textual overlays
  - per-layer and terrain counters
  - debug-only visibility controls

### Remaining in-engine UI

- Files:
  - `src/plugins/ui/{mod,scroll,search,toggles,panel}.rs`
  - `src/plugins/ui/patches/{mod,selection,dropdown,slider}.rs`
  - `src/plugins/ui/setup/{mod,panel_shell,patch_shell,search_shell,styles}.rs`
  - `src/map/ui_layers/{mod,panel/{mod,view_rows,layer_rows,debug_row},controls/{mod,view,layers,debug},diagnostics}.rs`
- Role:
  - legacy patch/fish/search panel UI
  - layer/debug overlay controls
  - pointer capture and Bevy-native debug/menu surfaces

## File And Module Pain Points

### Largest current files

- `src/map/terrain/runtime/camera/{mod,startup,controls,estimate,tests}.rs` split to ~16 / ~24 / ~129 / ~28 / ~267 LOC
- `src/map/terrain/runtime.rs` ~372 LOC
- `src/map/terrain/runtime/manifest.rs` ~354 LOC
- `src/map/terrain/runtime/drape/{mod,chunk_aligned,raster,mesh}.rs` split to ~83 / ~166 / ~138 / ~107 LOC
- `src/map/terrain/runtime/chunks.rs` ~348 LOC
- `src/map/terrain/mode.rs` ~367 LOC
- `src/plugins/api/requests/{mod,ensure,poll,apply,spawn,util}.rs` split to ~101 / ~59 / ~118 / ~102 / ~93 / ~136 LOC
- `src/plugins/api/state/{mod,bootstrap,filters,interaction,pending,catalog}.rs` split to ~18 / ~36 / ~70 / ~57 / ~18 / ~51 LOC
- `src/plugins/ui/patches/{mod,selection,dropdown,slider}.rs` split to ~15 / ~157 / ~411 / ~162 LOC
- `src/plugins/ui/setup/{mod,panel_shell,patch_shell,search_shell,styles}.rs` split to ~61 / ~142 / ~245 / ~226 / ~16 LOC
- `src/map/raster/policy/{mod,bounds,residency,requests}.rs` split to ~410 / ~279 / ~417 / ~369 LOC
- `src/map/raster/cache/{mod,render/{mod,geometry,loaded,visibility},filters/{mod,compose,clip_mask/{mod,sample,revision}}}.rs` split to ~363 / ~3 / ~110 / ~171 / ~116 / ~165 / ~99 / ~5 / ~245 / ~65 LOC
- `src/bridge/host/input/{mod,queue,commands/{mod,view,fish,selection}}.rs` split to ~17 / ~15 / ~55 / ~44 / ~19 / ~31 LOC
- `src/bridge/host/input/state/{mod,filters,layers,theme}.rs` split to ~40 / ~53 / ~30 / ~20 LOC
- `src/bridge/host/snapshot/{mod,filters/{mod,capabilities,layers,state},state/{mod,ui,interaction},view}.rs` split to ~96 / ~8 / ~15 / ~88 / ~51 / ~8 / ~18 / ~99 / ~31 LOC
- `src/bridge/host/emission/{mod,state,view,diagnostic}.rs` split to ~9 / ~65 / ~37 / ~92 LOC
- `src/bridge/host/persistence/{mod,patches,layers,view}.rs` split to ~14 / ~76 / ~96 / ~58 LOC
- `src/bridge/contract/{mod,input,snapshot,events,normalize}.rs` split to ~431 / ~322 / ~202 / ~39 / ~63 LOC
- `src/map/events/{snapshot,index,cluster}.rs` split to ~414 / ~189 / ~211 LOC
- `src/plugins/points/{query/{mod,state,refresh,evidence},render,loading}.rs` split to ~158 / ~81 / ~187 / ~82 / ~356 / ~70 LOC
- `src/map/ui_layers/{panel/{mod,view_rows,layer_rows,debug_row},controls/{mod,view,layers,debug},diagnostics}.rs` split to ~113 / ~244 / ~108 / ~106 / ~15 / ~155 / ~83 / ~87 / ~261 LOC
- `src/map/layers/{mod,registry,runtime}.rs` split to ~230 / ~260 / ~204 LOC

### Mixed responsibilities

- `bridge/host/input/{mod,queue,commands/{mod,view,fish,selection}}.rs`, `bridge/host/input/state/{mod,filters,layers,theme}.rs`, `bridge/host/snapshot/{mod,filters/{mod,capabilities,layers,state},state/{mod,ui,interaction},view}.rs`, `bridge/host/persistence/{mod,patches,layers,view}.rs`, and `bridge/host/emission/{mod,state,view,diagnostic}.rs`
  - the browser bridge is now split by responsibility, and snapshot, persistence, and emission helpers are separated into narrower domains, but the bridge still applies browser state directly into `PatchFilterState`, `FishFilterState`, `MapDisplayState`, `LayerRuntime`, and camera/view resources instead of going through a thinner bridge-translation layer.
- `map/raster/policy/{mod,bounds,residency,requests}.rs` and `map/raster/cache/{mod,render/{mod,geometry,loaded,visibility},filters/{mod,compose,clip_mask/{mod,sample,revision}}}.rs`
  - raster policy and cache are now split by responsibility; load completion, visibility sync, tile geometry, clip-mask sampling, and clip-mask revision tracking have separate homes, but the raster diagnostics/state boundary is still broad and `clip_mask/sample.rs` remains a dense hot path for later profiling.
- `map/terrain/{runtime,mode}.rs`
  - terrain mode activation and camera/lifecycle logic are now split into `mode.rs`, `runtime/camera/{mod,startup,controls,estimate,tests}.rs`, `runtime/manifest.rs`, `runtime/chunks.rs`, `runtime/drape/{mod,chunk_aligned,raster,mesh}.rs`, and `runtime/diagnostics.rs`; the remaining terrain complexity is now the runtime boundary between height-tile/debug helpers and the plugin shell rather than one monolithic file.
- `plugins/api/{mod,requests/{mod,ensure,poll,apply,spawn,util},fish,state/{mod,bootstrap,filters,interaction,pending,catalog}}.rs`
  - API/bootstrap state is now split into bootstrap, filter/display, interaction, pending-request, and fish-catalog modules, and request orchestration is split into scheduling, polling, response application, spawn helpers, and defaults/URL helpers, but bridge input and snapshot code still translate directly into those ECS resources instead of through a narrower bridge-focused state layer.
- `plugins/ui/{scroll,search,panel}.rs`, `plugins/ui/patches/{mod,selection,dropdown,slider}.rs`, `plugins/ui/setup/{mod,panel_shell,patch_shell,search_shell,styles}.rs`, and `map/ui_layers/{panel/{mod,view_rows,layer_rows,debug_row},controls/{mod,view,layers,debug},diagnostics}.rs`
  - the legacy panel code and the layer/debug menu are both split now, but there is still overlap between the remaining in-engine control surface and the separate debug/layer panel.

### Duplicated or spread-out state ownership

- 2D view state now lives in `map/camera/map2d.rs`, 3D view state in `map/camera/terrain3d.rs`, mode state in `map/camera/mode.rs`, and terrain camera/lighting activation in `map/terrain/mode.rs`, but camera spawn/fit logic still lives in `plugins/camera.rs`.
- bridge state now mutates `PatchFilterState`, `FishFilterState`, `MapDisplayState`, `LayerRuntime`, view mode, and view resources directly while bootstrap/meta data lives separately in `ApiBootstrapState`; there is still no bridge-focused translation layer separating browser contract state from ECS runtime state.
- diagnostics live in terrain runtime, raster runtime, and overlay/UI code with no clear diagnostics-only home.

### Mode-dependent branches spread across unrelated systems

- `ViewMode` gating appears in:
  - `bridge/host/input/commands/{mod,view}.rs`
  - `bridge/host/snapshot/{mod,filters/{mod,capabilities,layers,state},state/{mod,ui,interaction},view}.rs`
  - `plugins/input.rs`
  - `map/raster/runtime.rs`
- `plugins/points/{query/{mod,state,refresh,evidence},render}.rs`
  - `plugins/mask.rs`
  - `map/ui_layers/{controls/{mod,view,layers,debug},diagnostics}.rs`
  - `map/terrain/mode.rs`
  - `map/terrain/runtime.rs`
- This makes 2D/3D lifecycle tracing difficult and hides which systems are inactive versus merely invisible.

### Resources that currently own too much

- `PatchFilterState`
  - patch selection and normalized patch windows
- `FishFilterState`
  - fish filters and selected-fish display identity
- `MapDisplayState`
  - view/debug feature toggles
  - point icon scale
  - zone-mask presentation and hover RGB state
- `ApiBootstrapState`
  - bootstrap metadata
  - map version and layer retry state
  - zone name catalog
- `TerrainRuntime`
  - terrain manifests, now mostly driven through `runtime/manifest.rs`
  - chunk runtime, now mostly driven through `runtime/chunks.rs`
  - drape manifests and drape entities, now mostly driven through `runtime/drape/{chunk_aligned,raster}.rs`
  - height tiles
  - multiple cache/queue/counter groups
- `BrowserBridgeState`
  - currently small, but the surrounding module still owns too many bridge-specific translation details globally.

### Behavior that is hard to trace

- A browser patch changing view mode touches bridge code, terrain runtime mode resources, camera state, and later render-system gating.
- Raster clip-mask behavior spans `map/layers/{registry,runtime}.rs`, `map/raster/cache/filters/clip_mask/{sample,revision}.rs`, `plugins/mask.rs`, and vector runtime state.
- Event snapshot loading is now split into `map/events/snapshot.rs`, `map/events/index.rs`, and `map/events/cluster.rs`, while render-facing point query state, clustered point refresh, evidence-zone filtering, and marker sync now live in `plugins/points/{query/{mod,state,refresh,evidence},render}.rs`; the remaining complexity is the combined `PatchFilterState`/`FishFilterState` dependency and the coupling to raster evidence masks.

### Stale or half-migrated UI ownership

- `plugins/ui/{scroll,search,panel}.rs`, `plugins/ui/patches/{mod,selection,dropdown,slider}.rs`, and `plugins/ui/setup/{mod,panel_shell,patch_shell,search_shell,styles}.rs` are still a legacy in-engine control surface, but now with explicit subdomains.
- `map/ui_layers/{panel/{mod,view_rows,layer_rows,debug_row},controls/{mod,view,layers,debug},diagnostics}.rs` is now split, but still acts as a separate debug/layer menu system.
- The HTML/DaisyUI host already owns much of the user-facing control surface, so the remaining Bevy UI should be narrowed to debug/native-only surfaces rather than continuing to share general app control ownership.

## Proposed Target Shape

This sweep should move toward the following structure, incrementally:

```text
src/
  lib.rs
  app.rs
  config.rs
  prelude.rs

  bridge/
    mod.rs
    contract/
      mod.rs
      input.rs
      snapshot.rs
      events.rs
      normalize.rs
    host/
      mod.rs
      emission/
        mod.rs
        state.rs
        view.rs
        diagnostic.rs
      persistence/
        mod.rs
        patches.rs
        layers.rs
        view.rs
      input/
        mod.rs
        queue.rs
        commands/
          mod.rs
          view.rs
          fish.rs
          selection.rs
        state/
          mod.rs
          filters.rs
          layers.rs
          theme.rs
      snapshot/
        mod.rs
        filters/
          mod.rs
          capabilities.rs
          layers.rs
          state.rs
        state/
          mod.rs
          ui.rs
          interaction.rs
        view.rs
    theme.rs

  map/
    mod.rs
    spaces/
      mod.rs
      affine.rs
      world.rs
      layer_transform.rs
    layers/
      mod.rs
      registry.rs
      runtime.rs
      ui_state.rs
    raster/
      mod.rs
      manifest.rs
      scheduler.rs
      cache/
        mod.rs
        render/
          mod.rs
          geometry.rs
          loaded.rs
          visibility.rs
        filters.rs
      policy/
        mod.rs
        bounds.rs
        residency.rs
        requests.rs
    vector/
      mod.rs
      geojson.rs
      triangulate.rs
      build.rs
      cache.rs
      render.rs
      style.rs
    terrain/
      mod.rs
      camera3d.rs
      mode.rs
      chunks.rs
      drape.rs
      height_tiles.rs
      materials.rs
      mesh.rs
      runtime.rs
    events/
      mod.rs
      snapshot.rs
      index.rs
      cluster.rs
    diagnostics/
      mod.rs
      overlay.rs
      metrics.rs

  plugins/
    mod.rs
    bridge.rs
    cameras.rs
    raster.rs
    vector.rs
    terrain.rs
    events.rs
    diagnostics.rs
```

### Concrete ownership rules for this sweep

- `bridge/`
  - owns WASM entrypoints, contract translation, emitted browser events, and theme parsing.
- `map/spaces/`
  - owns canonical Bevy-facing map/layer/world transform types; it should be the only Bevy home for those formulas.
- `map/layers/`
  - separates static layer descriptors from runtime mutable layer state.
- `map/raster/`
  - owns manifest/cache/policy/render helpers and should remain the only home for raster residency, cache, and clip-mask sampling logic.
- `map/events/`
  - owns loaded snapshot state, spatial indexing, and clustering; point rendering should consume this state rather than redefine it.
- `map/terrain/`
  - should separate 2D/3D mode state and camera-transition helpers from chunk/drape runtime work.
- `plugins/`
  - should become thin wiring, not primary homes for domain logic.

## Sweep Priorities

1. Move bridge code under an explicit `bridge/` module.
2. Rename `map_space/` into `map/spaces/` and make it the only Bevy transform home.
3. Split events snapshot/index/cluster logic out of `map/events_store.rs`.
4. Pull raster helper logic out of the old `plugins/tiles.rs` into `map/raster/`.
5. Split terrain mode/view state from terrain chunk/drape runtime enough that 2D/3D ownership is easier to trace.
6. Keep Bevy-native UI focused on debug/native overlays; avoid expanding the legacy control surface.

## What Changed In This Sweep

- Moved the browser-facing contract and host code into `src/bridge/`:
  - `src/bridge/contract/{mod,input,snapshot,events,normalize}.rs`
  - `src/bridge/host/{mod,emission/{mod,state,view,diagnostic},persistence/{mod,patches,layers,view}}.rs`
  - `src/bridge/host/input/{mod,queue,state,commands/{mod,view,fish,selection}}.rs`
  - `src/bridge/host/snapshot/{mod,filters/{mod,capabilities,layers,state},state/{mod,ui,interaction},view}.rs`
  - `src/bridge/theme.rs`
  - This removed the old top-level `bridge_contract.rs` / `browser_bridge.rs` split and isolated CSS color parsing from the host plugin.
- Moved Bevy transform formulas under `src/map/spaces/` and made `map::spaces` the internal import path for map/world/layer geometry.
- Split event loading/query logic into `src/map/events/`:
  - `snapshot.rs` owns network and loaded snapshot state
  - `index.rs` owns spatial indexing and tile-scope selection helpers
  - `cluster.rs` owns render-oriented clustering
- Split camera ownership out of terrain runtime:
  - `src/map/camera/map2d.rs` owns flat-map view state and camera application
  - `src/map/camera/mode.rs` owns `ViewMode` / `ViewModeState`
  - `src/map/camera/terrain3d.rs` owns terrain camera state and reset helpers
- Split raster runtime out of the old monolithic tile plugin:
  - `src/plugins/raster.rs` is now thin plugin wiring
  - `src/map/raster/manifest.rs` owns tileset manifests and map-version URL resolution
  - `src/map/raster/policy/mod.rs` now owns shared raster policy state and public re-exports
  - `src/map/raster/policy/bounds.rs` owns visible-set analysis, LOD hysteresis, and camera-motion-derived cache budget inputs
  - `src/map/raster/policy/residency.rs` owns raster residency planning, fallback ancestor coverage, and eviction weighting helpers
  - `src/map/raster/policy/requests.rs` owns request queue building, motion-aware request suppression, and tile request startup/logging
  - `src/map/raster/cache/mod.rs` now owns shared cache state, eviction, residency counters, and stable raster cache types
  - `src/map/raster/cache/render/mod.rs` now owns only the stable raster-render helper surface
  - `src/map/raster/cache/render/geometry.rs` owns affine-quad checks, tile quad meshes, world rect projection, and exact-pixel zone extraction
  - `src/map/raster/cache/render/loaded.rs` owns raster load completion, exact-pixel payload capture, and first-time tile entity creation
  - `src/map/raster/cache/render/visibility.rs` owns visibility linger, entity visibility/depth sync, and per-layer visible counts
  - `src/map/raster/cache/filters/mod.rs` owns per-tile filter-state coordination for ready raster tiles
  - `src/map/raster/cache/filters/compose.rs` owns exact-pixel filtering, hover highlighting, and in-place raster visual composition
  - `src/map/raster/cache/filters/clip_mask/mod.rs` now owns only the stable clip-mask helper surface
  - `src/map/raster/cache/filters/clip_mask/sample.rs` owns raster/vector clip-mask sampling and ready-tile ancestor sampling
  - `src/map/raster/cache/filters/clip_mask/revision.rs` owns clip-mask state revision hashing
  - `src/map/raster/runtime.rs` owns the frame update system that coordinates those pieces
- Split API/bootstrap support out of the old `src/plugins/api.rs` monolith:
  - `src/plugins/api/mod.rs` is now thin plugin wiring and public re-exports
  - `src/plugins/api/state/mod.rs` now owns only the stable API-state re-export surface
  - `src/plugins/api/state/bootstrap.rs` owns bootstrap metadata, map-version state, and zone-name catalog state
  - `src/plugins/api/state/filters.rs` owns patch, fish, and display/filter resources
  - `src/plugins/api/state/interaction.rs` owns hover/selection interaction resources and info payloads
  - `src/plugins/api/state/pending.rs` owns async request-handle resources
  - `src/plugins/api/state/catalog.rs` owns fish catalog resources and fish-table normalization internals
  - `src/plugins/api/requests/mod.rs` now owns only the stable request helper surface
  - `src/plugins/api/requests/ensure.rs` owns request scheduling guards for meta, layers, zones, and fish catalog bootstrap
  - `src/plugins/api/requests/poll.rs` owns async response polling and response-to-resource application orchestration
  - `src/plugins/api/requests/apply.rs` owns meta/layer response application and zone-mask control sync
  - `src/plugins/api/requests/spawn.rs` owns async request spawning and client error normalization
  - `src/plugins/api/requests/util.rs` owns bootstrap defaults, zone-stats request building, and public asset URL resolution helpers
  - `src/plugins/api/fish.rs` owns fish catalog normalization and icon/url cleanup
- Split the legacy Bevy UI plugin out of the old `src/plugins/ui.rs` monolith:
  - `src/plugins/ui/mod.rs` now owns only shared marker/resource types plus plugin wiring
  - `src/plugins/ui/scroll.rs` owns wheel/scrollbar behavior for evidence, autocomplete, and patch dropdowns
  - `src/plugins/ui/search.rs` owns fish search state, text input, tags, and autocomplete selection
  - `src/plugins/ui/toggles.rs` owns legacy toggle buttons and zone-mask opacity controls
  - `src/plugins/ui/panel.rs` owns selected-zone text and evidence list rendering
  - `src/plugins/ui/setup/mod.rs` now owns only startup wiring plus shared style/font helpers
  - `src/plugins/ui/setup/panel_shell.rs` owns the zone/evidence panel shell construction
  - `src/plugins/ui/setup/patch_shell.rs` owns patch-range and point-icon shell construction
  - `src/plugins/ui/setup/search_shell.rs` owns search/autocomplete shell construction
  - `src/plugins/ui/setup/styles.rs` owns font loading and the shared text-style helper
  - `src/plugins/ui/patches/mod.rs` now owns only public re-exports for the patch UI subsystem
  - `src/plugins/ui/patches/selection.rs` owns patch-range normalization, selected-label sync, and shared patch ordering helpers
  - `src/plugins/ui/patches/dropdown.rs` owns dropdown visibility, entry spawning, and scrollbar interaction
  - `src/plugins/ui/patches/slider.rs` owns point-icon slider drag/sync behavior
  - `src/bridge/host/persistence/patches.rs` now reuses the shared patch timestamp helper instead of carrying its own copy
- Split the layer/debug Bevy UI panel out of `src/map/ui_layers.rs`:
  - `src/map/ui_layers/mod.rs` now owns plugin wiring plus shared UI marker/resource types
  - `src/map/ui_layers/panel/mod.rs` now owns the stable panel setup/rebuild entrypoints and root panel shell
  - `src/map/ui_layers/panel/view_rows.rs` owns view toggle, mode toggle, and terrain drape row construction
  - `src/map/ui_layers/panel/layer_rows.rs` owns per-layer row spawning and opacity button construction
  - `src/map/ui_layers/panel/debug_row.rs` owns debug toggle, eviction toggle, and diagnostics text construction
  - `src/map/ui_layers/controls/mod.rs` now owns only the stable layer-control helper surface
  - `src/map/ui_layers/controls/view.rs` owns view toggle, mode toggle, and terrain drape interaction/label sync
  - `src/map/ui_layers/controls/layers.rs` owns layer visibility/opacity interaction and label sync
  - `src/map/ui_layers/controls/debug.rs` owns debug and eviction interaction/label sync
  - `src/map/ui_layers/diagnostics.rs` owns debug text generation and layer/terrain diagnostic formatting
- Split the browser bridge host out of the old `src/bridge/host.rs` monolith:
  - `src/bridge/host/mod.rs` now owns wasm exports, thread-local bridge state, and plugin wiring
  - `src/bridge/host/emission/mod.rs` now owns only the stable bridge-emission helper surface
  - `src/bridge/host/emission/state.rs` owns ready, selection, and hover event emission
  - `src/bridge/host/emission/view.rs` owns throttled view-change emission
  - `src/bridge/host/emission/diagnostic.rs` owns diagnostic payload projection and deduped diagnostic event emission
  - `src/bridge/host/persistence/mod.rs` now owns only the stable persistence helper surface
  - `src/bridge/host/persistence/patches.rs` owns patch-window normalization and current patch-range projection
  - `src/bridge/host/persistence/layers.rs` owns layer order, opacity, and clip-mask override application
  - `src/bridge/host/persistence/view.rs` owns restored-view application and contract view-mode/RGB translation helpers
  - `src/bridge/host/input/mod.rs` now owns only the stable browser-input re-export surface
  - `src/bridge/host/input/queue.rs` owns draining pending JS patches into queued Bevy commands
  - `src/bridge/host/input/state/mod.rs` now owns only browser-input application orchestration
  - `src/bridge/host/input/state/filters.rs` owns UI/display flags plus patch/fish filter translation into ECS state
  - `src/bridge/host/input/state/layers.rs` owns layer visibility/order/opacity/clip-mask override translation
  - `src/bridge/host/input/state/theme.rs` owns theme-background translation into clear-color and camera clear settings
  - `src/bridge/host/input/commands/mod.rs` now owns only the stable browser-command application surface
  - `src/bridge/host/input/commands/view.rs` owns reset/restore-view and view-mode command application
  - `src/bridge/host/input/commands/fish.rs` owns focus-fish command application
  - `src/bridge/host/input/commands/selection.rs` owns zone-selection command application and zone-stats request startup
  - `src/bridge/host/snapshot/mod.rs` now owns snapshot orchestration and the stable helper re-export surface for the bridge host
  - `src/bridge/host/snapshot/filters/mod.rs` now owns only the stable snapshot-filter helper surface
  - `src/bridge/host/snapshot/filters/capabilities.rs` owns bridge capability catalog projection
  - `src/bridge/host/snapshot/filters/layers.rs` owns layer ordering, layer summaries, and layer opacity/clip-mask override projection
  - `src/bridge/host/snapshot/filters/state.rs` owns effective outbound filter projection
  - `src/bridge/host/snapshot/state/mod.rs` now owns only the stable snapshot-state helper surface
  - `src/bridge/host/snapshot/state/ui.rs` owns outbound UI snapshot projection
  - `src/bridge/host/snapshot/state/interaction.rs` owns selection/hover snapshot projection, hover-layer sample serialization, and zone-stats serialization
  - `src/bridge/host/snapshot/view.rs` owns camera/view snapshot projection
- Split the bridge contract out of the old `src/bridge/contract.rs` monolith:
  - `src/bridge/contract/mod.rs` now owns the stable public re-export surface and contract tests
  - `src/bridge/contract/input.rs` owns theme/filter/ui patch types, browser commands, and patch application
  - `src/bridge/contract/snapshot.rs` owns outbound state snapshot DTOs
  - `src/bridge/contract/events.rs` owns browser-facing event tags/payloads
  - `src/bridge/contract/normalize.rs` owns normalization helpers and nullable-string deserialization glue
- Split the points/events plugin out of the old `src/plugins/points.rs` monolith:
  - `src/plugins/points/mod.rs` now owns only plugin wiring and public re-exports
  - `src/plugins/points/loading.rs` owns snapshot polling and point-ring asset initialization
  - `src/plugins/points/query/mod.rs` now owns the stable query helper surface plus shared filter-normalization helpers and query tests
  - `src/plugins/points/query/state.rs` owns point query resources, render-point DTOs, and the internal query signature
  - `src/plugins/points/query/refresh.rs` owns viewport/signature derivation and clustered point refresh from the local snapshot
  - `src/plugins/points/query/evidence.rs` owns evidence-zone filter derivation from the snapshot
  - `src/plugins/points/render.rs` owns ring/icon marker entities, icon caching, viewport projection, and point sprite sync
- Split the old broad `UiState` resource into explicit API/bootstrap vs map-UI resources:
  - `src/plugins/api/state/bootstrap.rs` now owns `ApiBootstrapState` for meta/defaults/map-version/zones status
  - `src/plugins/api/state/filters.rs` now owns `PatchFilterState` for patch range, `FishFilterState` for fish filters/selection identity, and `MapDisplayState` for display toggles, point-icon scale, and zone-mask presentation state
  - `src/plugins/api/state/interaction.rs` now owns hover and selection state/resources
  - `src/plugins/api/requests/poll.rs` now updates bootstrap/meta state separately from filter state and display state
  - bridge host input/snapshot/emission, points, raster, mask, and remaining Bevy UI code now read the narrower resource they actually need instead of a single merged `UiState`
- Split the remaining broad filter state by domain:
  - bridge host input now applies patch-range overrides into `PatchFilterState` and fish selection into `FishFilterState`
  - patch dropdown/persistence code now depends only on `PatchFilterState`
  - fish search, selection snapshots, point filtering, and zone-evidence UI now depend only on `FishFilterState`
- Split `src/map/layers.rs` into explicit descriptor and runtime state modules:
  - `src/map/layers/mod.rs` now owns shared layer ids/spec enums and stable re-exports
  - `src/map/layers/registry.rs` owns DTO-to-spec parsing, registry indexing, and transform updates
  - `src/map/layers/runtime.rs` owns dynamic layer visibility/opacity/clip-mask/vector-progress state
- Split terrain mode lifecycle out of `src/map/terrain/runtime.rs`:
  - `src/map/terrain/mode.rs` now owns camera activation state, control-mutation flags, mode-specific camera/light application, projection helpers, and debug mode/render assertions
  - `src/map/terrain/runtime.rs` now focuses more narrowly on manifest loading, chunk runtime, drape updates, height sampling, and diagnostics
- Split terrain drape and diagnostics helpers out of `src/map/terrain/runtime.rs`:
  - `src/map/terrain/runtime/drape/mod.rs` now owns only drape gating and the stable drape helper surface
  - `src/map/terrain/runtime/drape/chunk_aligned.rs` owns chunk-aligned drape visibility, texture fetch/build progression, and stale-entry cleanup
  - `src/map/terrain/runtime/drape/raster.rs` owns raster-tile drape visibility, build-budget gating, and stale raster-drape cleanup
  - `src/map/terrain/runtime/drape/mesh.rs` owns raster drape mesh construction and terrain-height sampling for drape projection
  - `src/map/terrain/runtime/diagnostics.rs` now owns `TerrainDiagnostics` projection from runtime/view state
  - `src/map/terrain/runtime.rs` now focuses more narrowly on manifest requests, chunk queues/builds, height-tile runtime, and remaining terrain debug checks
- Split terrain chunk scheduling/build/cache flow out of `src/map/terrain/runtime.rs`:
  - `src/map/terrain/runtime/chunks.rs` now owns visible-chunk planning, fallback ancestor selection, incremental chunk request/build progression, resident-count sync, and terrain chunk cache eviction
  - `src/map/terrain/runtime.rs` now focuses more narrowly on manifest requests, manifest decode/network helpers, height-tile runtime, and remaining terrain debug checks
- Split terrain manifest lifecycle and fetch/decode helpers out of `src/map/terrain/runtime.rs`:
  - `src/map/terrain/runtime/manifest.rs` now owns config-driven invalidation, terrain/drape manifest request startup, manifest/chunk response polling, manifest decode/root rebasing, and async fetch helpers
  - at that stage, `src/map/terrain/runtime.rs` was reduced to camera controls, view estimation, height-tile runtime, and remaining terrain debug checks before the later camera split
- Split terrain camera controls and view estimation out of `src/map/terrain/runtime.rs`:
  - `src/map/terrain/runtime/camera/mod.rs` now owns the stable terrain-camera helper surface plus the `TerrainViewEstimate` re-export
  - `src/map/terrain/runtime/camera/startup.rs` owns default terrain-mode boot and terrain-light spawn
  - `src/map/terrain/runtime/camera/controls.rs` owns orbit/pan/dolly input state and 3D camera control application
  - `src/map/terrain/runtime/camera/estimate.rs` owns terrain view estimation
  - `src/map/terrain/runtime/camera/tests.rs` owns the camera/mode regression tests
  - `src/map/terrain/runtime.rs` now focuses on terrain runtime resources, height tiles, chunk/drape bookkeeping, and the plugin shell
- Wired the dormant chunk-aligned drape branch into the explicit drape-mode gate inside `map/terrain/runtime.rs`, which removed dead-code warnings and made the drape path easier to profile later.
- Remaining high-value follow-ups are now the remaining direct bridge-to-ECS translation path across `bridge/host/input/state/{filters,layers}.rs` and `bridge/host/snapshot/state/interaction.rs`.
