# Datastar Map Remediation

Last updated: 2026-03-31

This document isolates the map-specific Datastar remediation plan from the broader
`datastar-frp-focus` worklog.

The goal is not to "polish" `site/assets/map/loader.js`. The goal is to replace most of its
current responsibilities with:

- declarative Datastar bindings in the map shell
- signal-owned page state
- small reusable UI components/modules
- a thin JS <-> Bevy bridge adapter

This document should be sufficient context for a fresh Codex session without relying on prior
conversation history.

## Progress log

### 2026-03-31: Slice 1 landed

- Created a new pure signal-contract module in:
  - `site/assets/map/map-signal-contract.js`
- Moved map defaults and signal normalizers out of the imperative loader.
- Added:
  - `site/assets/map/map-signal-contract.test.mjs`

This established the clean-slate destination for subsequent migrations.

### 2026-03-31: Slice 2 landed

- Moved the first bridge-owned fields off transitional `_map_controls` and onto `_map_bridged`:
  - `ui.diagnosticsOpen`
  - `ui.viewMode`
  - `filters.fromPatchId`
  - `filters.toPatchId`
- Updated page persistence/restore so those fields are now sourced from `_map_bridged`.
- Kept the stored JSON shape stable under `inputUi` / `inputFilters` for backwards compatibility.

### 2026-03-31: Slice 3 landed

- Extracted bridge projection rules from `loader.js` into:
  - `site/assets/map/map-bridge-projection.js`
- This moved the bridge whitelist/projection logic into a smaller pure module.

### 2026-03-31: Slice 4 landed

- Moved the global point display toggles into `_map_bridged.ui`:
  - `showPoints`
  - `showPointIcons`
  - `pointIconScale`
- Updated persistence/restore to match that ownership.

### 2026-03-31: Slice 5 landed

- Moved the durable layer-override cluster to `_map_bridged.filters` for persistence/restore:
  - `layerIdsVisible`
  - `layerIdsOrdered`
  - `layerOpacities`
  - `layerClipMasks`
  - `layerWaypointConnectionsVisible`
  - `layerWaypointLabelsVisible`
  - `layerPointIconsVisible`
  - `layerPointIconScales`
- Cleaned the initial signal shape so `_map_controls` is now page-only again for:
  - `filters.searchText`
  - `filters.patchId`
  - `ui.legendOpen`
  - `ui.leftPanelOpen`
- Expanded `_map_bridged.filters` defaults in the new signal-contract module and shell bootstrap to reflect the new ownership.
- Kept storage backwards-compatible by still reading legacy layer overrides from `_map_controls.filters` when restoring older snapshots.
- This slice only changes durable ownership at rest; live layer interaction handlers still patch `_map_controls` and will be migrated next.

### 2026-03-31: Slice 6 landed

- Moved live layer interaction patches to `_map_bridged`:
  - visibility toggles
  - waypoint connection/label toggles
  - point-icon toggles
  - point-icon scales
  - layer opacity sliders
  - drag-drop reordering / clip mask attachment
- Removed the layer-override fields from the transitional `_map_controls -> _map_bridged` projection in:
  - `site/assets/map/map-bridge-projection.js`
- Tightened `normalizeMapControlSignalState(...)` so the transitional control branch now only normalizes page-owned control fields plus still-transitional fish/zone/semantic filter inputs.

Net effect:

- layer stack and layer override state is now bridged-owned both:
  - at rest
  - during live interaction
- `_map_controls` no longer owns or derives layer override state
- the transitional bridge projection is materially smaller again

### 2026-03-31: Slice 7 landed

- Extracted the pure layer-state helpers from `loader.js` into:
  - `site/assets/map/map-layer-state.js`
- Migrated into that module:
  - layer ordering resolution
  - visible-layer derivation
  - opacity helpers
  - point-icon scale helpers
  - drag-drop ordering helpers
  - clip-mask normalization helpers
  - per-layer toggle patch builders
- `loader.js` now imports those helpers instead of defining them inline.
- Follow-up stabilization:
  - kept `flattenLayerClipMasks(...)` and the point-scale constants flowing through the new module boundary so the layer panel markup stays self-consistent after extraction.

Why this matters:

- this is a clean-slate migration out of the imperative monolith, not another ownership tweak inside it
- it removes a coherent responsibility cluster from `loader.js`
- it gives future map work a smaller pure module as the place to continue moving layer behavior

### 2026-03-31: Slice 8 landed

- Added a new live layer panel controller in:
  - `site/assets/map/map-layer-panel-live.js`
- The live map app now boots that controller from:
  - `site/assets/map/map-app-live.js`
- This is the first live shell controller that renders and mutates a substantial map UI subtree
  without going through `loader.js`.

What the controller owns:

- rendering the live layer stack via the already-extracted `map-layer-panel.js`
- settings expansion state via `_map_ui.layers.expandedLayerIds`
- visibility toggles via `_map_bridged.filters.layerIdsVisible`
- per-layer waypoint/icon toggles
- per-layer opacity/icon-scale sliders
- drag/drop reordering and clip-mask attachment

Why this matters:

- the live Layers window is no longer a placeholder while `loader.js` still exists in the repo
- it proves the clean-slate path can own a real UI window end-to-end
- it materially reduces the reason to keep any layer UI behavior in the legacy loader

Validation for this slice:

- `node --check site/assets/map/map-layer-panel-live.js`
- `node --check site/assets/map/map-app-live.js`
- `node --check site/assets/js/pages/map-page.js`
- `node --test site/assets/map/map-layer-panel-live.test.mjs site/assets/map/map-app.test.mjs site/assets/js/pages/map-page.test.mjs`
- rebuilt site output
- verified served:
  - `/map/map-layer-panel-live.js`
  - `/map/map-app-live.js`
  - `/js/pages/map-page.js`
  match `site/.out`
- live DevTools checks confirmed:
  - clicking a layer settings button dispatches `_map_ui.layers.expandedLayerIds`
  - expanded fish-evidence controls become visible
  - fish-evidence icon toggle writes `_map_bridged.filters.layerPointIconsVisible`

### 2026-03-31: Supporting signal-store fix landed

The live layer panel exposed an important page-store issue:

- `window.__fishystuffMap.signalObject()` still returned a disconnected cloned snapshot
- deep merge alone could not clear canonical object-map branches like:
  - `layerPointIconsVisible`
  - `layerWaypointLabelsVisible`
  - `layerClipMasks`

What changed in `site/assets/js/pages/map-page.js`:

- `signalObject()` now returns the live Datastar-backed object instead of a stale snapshot clone
- exact branch replacement is now applied for a small explicit whitelist of canonical branches

Why this matters:

- clean-slate controllers can now read/write the live shell state directly
- canonical object-map clears back to `{}` work correctly
- this avoids having new modules accidentally depend on loader-style mirrored state

### 2026-03-31: Partial bridge-event regression fixed

The first live clean-slate controllers exposed a bridge-event assumption bug in
`site/assets/map/map-app-live.js`.

Problem:

- the live app treated every incoming bridge event as if `event.detail.state` were a full runtime
  snapshot
- `fishymap:view-changed` only carries a partial payload:
  - `state.view`
- that partial payload replaced `_map_runtime` on the page side and cleared:
  - `ready`
  - `catalog`
  - `statuses`

Visible symptom:

- dragging the map could make the Layers window fall back to:
  - `Layer registry is loading…`
- Settings could fall back to:
  - `Loading`
- clicking a point recovered the UI because `selection-changed` emits a full snapshot again

Fix:

- added `resolveBridgeSnapshot(...)` in `site/assets/map/map-app-live.js`
- live bridge events are now resolved against the bridge's current full snapshot before
  projecting into `_map_runtime` / `_map_session`

Validation:

- `node --check site/assets/map/map-app-live.js`
- `node --test site/assets/map/map-app-live.test.mjs site/assets/map/map-app.test.mjs site/assets/js/pages/map-page.test.mjs`
- rebuilt site output and reloaded the served `/map/`
- live DevTools check:
  - dispatched a synthetic `fishymap:view-changed` carrying only `state.view`
  - `_map_runtime.ready` stayed `true`
  - `_map_runtime.catalog.layers.length` stayed `7`

### 2026-03-31: Live search moved onto Datastar state

Added a new clean-slate search path:

- `site/assets/map/map-search-state.js`
- `site/assets/map/map-search-panel-live.js`

This moves the live search window off the legacy loader path.

What the new search path owns:

- deriving a search-state bundle directly from:
  - `_map_ui.search`
  - `_map_bridged.filters`
  - `_shared_fish`
  - `_map_runtime.catalog`
- building live matches for:
  - fish
  - fish filter terms
  - semantic terms
- rendering:
  - active search chips
  - live result rows
- mutating only the canonical signal branches:
  - `_map_ui.search`
  - `_map_bridged.filters`

Important note:

- this clean-slate slice does **not** reintroduce the old loader-owned external zone-catalog path
- fish/filter/semantic search now works live from the runtime-owned catalog
- dedicated zone-catalog search can be reintroduced later from a clean source if still wanted

Why this matters:

- the search window is now another real live shell subsystem that no longer depends on
  `loader.js`
- it continues the pattern:
  - pure state derivation module
  - small live controller
  - Datastar-owned page signals
  - no intermediate mirrored control branch

Validation:

- `node --check site/assets/map/map-search-state.js`
- `node --check site/assets/map/map-search-panel-live.js`
- `node --check site/assets/map/map-app-live.js`
- `node --test site/assets/map/map-search-state.test.mjs site/assets/map/map-app-live.test.mjs site/assets/map/map-app.test.mjs site/assets/js/pages/map-page.test.mjs`
- rebuilt site output
- verified served search modules are present in the live map page
- live DevTools checks confirmed:
  - typing into the search box populates live results
  - selecting a semantic result patches `_map_bridged.filters.semanticFieldIdsByLayer`
  - selecting and removing a fish-filter chip patches `_map_bridged.filters.fishFilterTerms`

### 2026-03-31: Live bookmarks moved onto Datastar state

Added a new clean-slate bookmark path:

- `site/assets/map/map-bookmark-state.js`
- `site/assets/map/map-bookmark-panel-live.js`

This replaces another loader-owned UI island with explicit page-owned signals.

What the new bookmark path owns:

- deriving bookmark panel state directly from:
  - `_map_bookmarks.entries`
  - `_map_ui.bookmarks`
  - `_map_runtime.view`
  - `_map_runtime.selection`
- placing a bookmark from the next runtime selection while `placing = true`
- bookmark-local management:
  - select / clear selection
  - delete selected
  - rename
  - reorder
- map-facing bookmark inspect intent via explicit action signals:
  - `_map_actions.focusWorldPointToken`
  - `_map_actions.focusWorldPoint`

Important note:

- bookmark manager UI state remains page-owned
- only minimal bookmark geometry and selected bookmark ids still cross the bridge
- legacy copy/export/import affordances are intentionally disabled in this slice instead of
  remaining visible-but-dead while the format migration is still unresolved

Bridge/app follow-up found during this slice:

- clean-slate controllers were listening to Datastar patch events only
- bridge-originated `fishymap-signals-patch` updates mutate the live shell through
  `window.__fishystuffMap.applyPatch(...)`, but they do not naturally fan out to those
  controllers
- `map-app-live.js` now explicitly schedules:
  - window manager
  - bookmarks
  - layers
  - search
  after each bridge-to-shell signal patch

Why this matters:

- bookmarks are now another real live subsystem running off canonical signals instead of loader
  locals
- bookmark inspect is now represented as an explicit FRP action token rather than bookmark glue
  hidden in the bridge
- the app-level controller fan-out makes the live clean-slate modules react consistently to
  bridge-driven state changes

Validation:

- `node --check site/assets/map/map-bookmark-state.js`
- `node --check site/assets/map/map-bookmark-panel-live.js`
- `node --check site/assets/map/map-app-live.js`
- `node --check site/assets/map/map-runtime-adapter.js`
- `node --test site/assets/map/map-bookmark-state.test.mjs site/assets/map/map-runtime-adapter.test.mjs site/assets/map/map-app.test.mjs site/assets/js/pages/map-page.test.mjs`
- rebuilt site output
- served asset checks matched `.out`
- live DevTools checks confirmed:
  - bookmark inspect patches `_map_actions.focusWorldPoint*`
  - bookmark toolbar controls now render from live Datastar state
  - copy/export/import buttons are explicitly disabled instead of remaining inert

## Why this exists

The map is now the biggest remaining area where we drift from Datastar's intended design.

The current architecture has improved since earlier revisions:

- there is now an explicit `_map_bridged` bridge contract
- runtime mirroring was narrowed compared with earlier broad state sync
- page persistence boundaries are cleaner than before

But the core problem remains:

- `site/assets/map/loader.js` is still about 9k lines
- it still acts as:
  - page UI state reconciler
  - renderer for multiple windows/panels
  - bookmark manager
  - search UI controller
  - layers UI controller
  - Datastar patch event router
  - JS <-> Bevy bridge adapter

That is not the end state Datastar is designed to support.

## Datastar guidance that matters here

Reference docs:

- <https://data-star.dev/guide/getting_started>
- <https://data-star.dev/guide/reactive_signals>
- <https://data-star.dev/guide/datastar_expressions>
- <https://data-star.dev/guide/backend_requests>
- <https://data-star.dev/guide/the_tao_of_datastar>
- <https://data-star.dev/reference/attributes>
- <https://data-star.dev/reference/actions>
- <https://data-star.dev/reference/sse_events>
- <https://data-star.dev/examples/>

The key guidance to follow for the map:

1. State should live in the right place.

- Datastar's Tao explicitly says most state should live in the backend, and the backend should
  drive the frontend by patching elements and signals.
- For the map page, the practical analogue is:
  - durable page UI state belongs in the Datastar signal graph
  - Bevy runtime state should cross the boundary only when the page truly needs it
  - transient internal runtime state should stay inside the runtime

2. Signals should be sparse and intentional.

- Datastar supports nested signal graphs and underscore-prefixed local state.
- That does not imply "mirror everything into signals".
- For the map, this means:
  - page UI state must be explicit
  - Bevy-shared state must be whitelisted
  - hover and per-frame runtime churn must stay out of Datastar

3. Expressions and bindings should do most of the page-local work.

- Datastar's model favors declarative `data-*` attributes and expressions.
- `data-effect` exists for side effects.
- `data-on-signal-patch` exists for narrow reactive seams.
- This pushes us away from:
  - global helper calls from templates
  - document-wide custom patch orchestration as the default mechanism

4. External JavaScript should be minimal and boundary-focused.

- Some JS is still appropriate:
  - bridge adaptation
  - reusable UI components
  - persistence helpers where Datastar OSS lacks built-ins
- But JS should not become a second application framework layered on top of Datastar.

## Current diagnosis

### What is already good

- `_map_bridged` is now a real explicit shared branch.
- `_map_session` and `_map_actions` are clearer than earlier ad hoc bridge state.
- page-owned persistence for map UI/bookmarks/session is cleaner than before.
- hover is no longer mirrored broadly through Datastar, which was the right correction.

### What is still wrong

1. `loader.js` owns too many responsibilities.

Current major responsibilities still in `site/assets/map/loader.js`:

- current page state projection:
  - `currentMapUiSignalState()`
  - `currentMapControlSignalState()`
  - `currentMapSessionSignalState()`
  - `currentMapBookmarksSignalState()`
  - `currentMapActionSignalState()`
  - `currentMapBridgedSignalState()`
- page UI rendering:
  - `renderBookmarkManager(...)`
  - `renderSearchSelection(...)`
  - `renderSearchResults(...)`
  - `renderCurrentState(...)`
- managed-window behavior:
  - `applyManagedWindows()`
  - `toggleManagedWindowOpen(...)`
  - `toggleManagedWindowCollapsed(...)`
- bridge sync:
  - `syncMapBridgedSignalsFromPageState(...)`
  - `syncBridgeInputStateFromSignals()`
  - `syncBridgeSessionStateFromSignals()`
  - `syncMapActionsFromSignals()`
- Datastar event routing:
  - the document-level `datastar-signal-patch` listener

2. The schema still mixes concerns.

Current branches:

- `_map_ui`
- `_map_controls`
- `_map_bridged`
- `_map_session`
- `_map_bookmarks`
- `_shared_fish`

Problem:

- `_map_controls` still mixes:
  - page-only control state
  - bridge-relevant rendering state
- this causes duplication and reconciliation:
  - page state -> control state -> bridged state -> runtime

That is unnecessary churn.

3. Templates still depend on JS helper globals for ordinary state mutation.

Current examples in `site/layouts/map.shtml`:

- `window.__fishystuffDatastarState.toggleBooleanPath(...)`
- `window.__fishystuffDatastarState.setObjectPath(...)`

This is workable but not ideal.

The better Datastar-shaped end state is:

- direct signal expressions where possible
- helper JS only where the interaction is genuinely too complex for inline expressions

4. The bookmark manager is not yet sufficiently isolated from bridge concerns.

Design requirement:

- bookmark manager should be entirely independent from hover info
- only bookmark data that Bevy actually needs should cross the bridge

That means:

- page-side bookmark UI/edit/select state should not be part of the shared bridge contract
- only a minimal bookmark projection should cross if required:
  - id
  - label
  - world position
  - selected ids if the runtime truly needs selected bookmark context

5. The bridge boundary must remain coarse.

This is reinforced by local map perf guidance:

- `map/docs/web-threading-readiness.md`
  - "The JS↔Wasm boundary must stay coarse."
  - "Batched commands in, compact events out."
- `map/docs/perf-workstream.md`
  - bridge work should stay coarse and batched
  - exact hover/click must stay out of the visual transport path
  - host/bridge work is measurable, but not something to make noisier

So the remediation must not reintroduce:

- broad runtime mirroring into Datastar
- fine-grained per-frame signal churn
- hover-derived signal traffic

## Target architecture

The target is a strict separation of responsibilities.

### 1. Page-owned signal state

#### `_map_ui`

Page-only UI state.

Examples:

- window open/collapsed/x/y
- search panel open
- expanded layer cards
- bookmark placement/edit mode
- selected zone-info tab
- diagnostics panel open if page-only

#### `_map_bookmarks`

Page-owned canonical bookmark state.

Examples:

- full bookmark entries
- local ordering
- editor-local metadata if needed

#### `_map_session`

Durable/restorable coarse runtime session snapshot.

Examples:

- view mode
- camera snapshot
- stable selection snapshot

Not:

- per-frame camera drift
- hover

#### `_map_actions`

Explicit one-shot commands only.

Examples:

- `resetViewToken`
- `resetUiToken`

### 2. Shared Bevy-facing signal state

#### `_map_bridged`

This is the only intentional page -> Bevy shared input branch.

It should contain only the minimal runtime-relevant state.

Examples:

- fish/zone/semantic filters
- patch range
- layer visibility/order/opacity/clip toggles
- evidence/icon visibility toggles
- point icon scale
- view mode if it is runtime-owned
- minimal bookmark projection if Bevy needs it

### 3. Runtime -> page outputs

#### `_map_runtime`

Coarse runtime outputs only.

Examples:

- `ready`
- status/error fields
- stable layer catalog
- patch catalog
- stable selection/detail payload
- settled camera/view snapshot if needed for persistence

Not:

- hover spam
- full input mirrors
- broad internal runtime snapshots

## Proposed signal cleanup

### Remove `_map_controls`

Long-term, `_map_controls` should disappear.

Reason:

- it is an intermediate staging branch that duplicates ownership
- it mixes page-local and bridge-shared concerns
- it forces loader reconciliation logic

Replacement:

- page-only controls move under `_map_ui`
- bridge-relevant controls move directly under `_map_bridged`

This is the most important schema simplification.

### Rename and clarify shared state

This repo already moved toward `_map_bridged`.

Keep pushing that naming discipline:

- `_map_ui` = page-only UI
- `_map_bookmarks` = page-owned canonical bookmarks
- `_map_bridged` = only values Bevy actually consumes
- `_map_runtime` = only values the page actually consumes from Bevy

## Explicit whitelist for bridge crossing

This should be treated as the authoritative design target.

### Page -> Bevy

Should cross:

- fish ids filter
- zone RGB filter
- semantic field filter map
- patch id / fromPatchId / toPatchId
- layerIdsVisible
- layerIdsOrdered
- layerOpacities
- layerClipMasks
- layerWaypointConnectionsVisible
- layerWaypointLabelsVisible
- layerPointIconsVisible
- layerPointIconScales
- diagnostics visibility only if runtime rendering actually depends on it
- showPoints
- showPointIcons
- pointIconScale
- viewMode
- minimal bookmark projection:
  - bookmark id
  - label
  - world coordinates
  - selected bookmark ids only if runtime needs them
- coarse session restore data:
  - camera/view
  - stable selection

Should not cross:

- search input open/closed state
- search box text unless search execution really happens in Bevy
- window positions/open states
- expanded layer cards
- bookmark placement/edit UI state
- legend open/closed
- left panel open/closed
- hover tooltip state
- generic loading indicators
- shared fish progress unless runtime rendering actually depends on it

### Bevy -> page

Should cross:

- `ready`
- bridge/runtime errors
- stable layer registry/catalog
- patch catalog
- stable selection payload
- zone/fish detail payloads needed for page rendering
- settled camera/view snapshot if used for persistence

Should not cross:

- hover
- per-frame camera changes
- broad internal runtime snapshots
- full echoed input state

## Loader replacement strategy

The objective is to replace most of `loader.js`, not merely subdivide it.

### Loader's future role

`site/assets/map/loader.js` should become:

- bootstrap
- bridge mount/unmount
- bridge input projection from `_map_bridged` / `_map_session` / `_map_actions`
- runtime output projection into `_map_runtime`
- nothing else

### What should move out of loader

1. Managed windows

Move out:

- open/collapse/position logic
- toolbar state rendering
- titlebar interactions

Target:

- Datastar-owned `_map_ui.windowUi`
- reusable managed-window component/module

2. Bookmark manager

Move out:

- bookmark list rendering
- selection logic
- import/export UI
- drag/drop reorder UI
- rename/delete/copy UI

Target:

- page-owned `_map_bookmarks`
- page-owned bookmark UI state under `_map_ui`
- bridge only receives minimal bookmark projection

3. Layer panel

Move out:

- layer list rendering
- drag/drop reorder UI
- per-layer toggles/sliders/expanded-state rendering

Target:

- page-owned UI under `_map_ui`
- bridge-relevant controls under `_map_bridged`
- reusable layer panel module/component

4. Search panel

Move out:

- search result rendering
- search selection shell rendering
- local open/close behavior

Target:

- page-owned `_map_ui.search`
- page-owned search state
- bridge only receives actual runtime-relevant search-derived filters if necessary

5. Zone-info tab UI

Move out:

- selected tab state
- tab rendering and switching logic

Target:

- `_map_ui.zoneInfo.tab`
- page-side rendering using coarse `_map_runtime.selection`

## Best-practice phased plan

### Phase 1: Freeze the signal contract

Goal:

- define the final map signal schema before more implementation work

Tasks:

- formally deprecate `_map_controls`
- document exact field ownership under:
  - `_map_ui`
  - `_map_bookmarks`
  - `_map_bridged`
  - `_map_session`
  - `_map_runtime`
  - `_map_actions`

Acceptance:

- no ambiguity about whether a given field is page-only, bridge-shared, or runtime output

### Phase 2: Remove template dependence on helper globals

Goal:

- replace routine `window.__fishystuffDatastarState.*` usage with direct Datastar expressions

Tasks:

- migrate toolbar toggle buttons
- migrate search-open expressions
- migrate reset-action token increments
- migrate diagnostics toggle expressions

Acceptance:

- map shell mostly uses direct Datastar expressions for ordinary state mutation

### Phase 3: Evict `_map_controls`

Goal:

- stop duplicating control state between page and bridge branches

Tasks:

- move page-only controls into `_map_ui`
- move bridge-relevant controls directly into `_map_bridged`
- remove reconciliation paths that only exist to keep `_map_controls` in sync

Acceptance:

- loader no longer needs "controls -> bridged" projection logic

### Phase 4: Extract bookmark manager

Goal:

- make bookmarks page-owned and independent from hover/runtime churn

Tasks:

- move bookmark UI rendering out of loader
- move bookmark UI interactions into a page component/module
- keep only minimal bookmark projection in `_map_bridged`

Acceptance:

- bookmark manager works without loader rendering the list UI

### Phase 5: Extract layer panel

Goal:

- make the layer UI declarative/page-owned

Tasks:

- move layer card rendering and local UI state out of loader
- keep only runtime-relevant layer values in `_map_bridged`

Acceptance:

- loader no longer renders the layer list or owns layer UI interactions

### Phase 6: Extract search and zone-info UI

Goal:

- make search and detail UI page-side

Tasks:

- move search results rendering out of loader
- move selected zone-info tab state/rendering out of loader
- keep only stable selection/detail payloads from runtime in `_map_runtime`

Acceptance:

- loader no longer renders page panels

### Phase 7: Collapse loader into a thin bridge adapter

Goal:

- leave only bridge code in `loader.js`

Tasks:

- keep:
  - mount/unmount
  - `_map_bridged` -> Bevy projection
  - `_map_session` -> Bevy restore projection
  - `_map_actions` -> Bevy command dispatch
  - Bevy events -> `_map_runtime` projection
- remove:
  - page UI renderers
  - local page UI reconcilers
  - panel/window/business logic not strictly required for bridge operation

Acceptance:

- `loader.js` becomes a small bridge-oriented module instead of a monolith

## Anti-goals

These are explicitly not the target:

- mirroring broad runtime state into Datastar
- sending hover through Datastar
- per-frame signal patch churn
- replacing one monolith with several tightly coupled helper monoliths
- using Datastar as a generic event bus for everything

## Performance constraints

This remediation must obey existing map perf guidance.

Important local constraints:

- `map/docs/web-threading-readiness.md`
  - production performance must be acceptable on single-threaded Wasm
  - JS↔Wasm boundary must stay coarse
- `map/docs/perf-workstream.md`
  - bridge work should stay coarse and batched
  - exact hover/click must not be coupled to display transport

So every phase must preserve:

- no hover-through-Datastar regression
- no broad runtime mirroring regression
- no increase in bridge chatter from ordinary UI interactions

## Validation criteria

Every phase should be checked against:

1. Correctness

- map loads
- 2D/3D toggles work
- filters work
- layers work
- bookmarks work
- reset flows work
- selection/detail panels work

2. Contract discipline

- page-only state remains page-only
- `_map_bridged` contains only the explicit whitelist
- `_map_runtime` remains coarse

3. Performance

- no new hover FPS collapse
- no new bridge call amplification
- no unnecessary runtime reloads from page-only UI changes

4. Served asset correctness

- compare served files against `site/.out` before trusting browser validation

## Recommended first implementation slice

The first best slice is:

- formally remove `_map_controls` from the schema

Reason:

- it is the highest-leverage structural simplification
- it will reduce both template helper usage and loader reconciliation code
- it clarifies what is page-only vs bridge-shared before extracting larger UI modules

Concrete immediate next steps:

1. write the final field-by-field schema mapping
2. rebind shell controls from `_map_controls` to `_map_ui` or `_map_bridged`
3. delete the control-to-bridged projection paths that become obsolete

Once that lands, bookmark and layer extraction become much cleaner.

## First implementation slice landed

The first clean-slate extraction started with a new pure contract module:

- `site/assets/map/map-signal-contract.js`

What moved into it:

- map signal defaults
- bridge/shared branch constants
- page UI signal defaults
- bridged signal defaults
- session/action defaults
- pure normalization helpers for:
  - window UI state
  - page UI state
  - transitional `_map_controls`
  - `_map_bridged`

Why this is the right first cut:

- it creates a new clean functional core outside `loader.js`
- it removes some schema/default/normalization ownership from `loader.js`
- it gives later migration slices a stable place to move logic into without continuing to grow
  the imperative monolith

What did **not** happen yet:

- `_map_controls` still exists
- loader still owns major UI/render responsibilities
- the bridge adapter is not yet isolated

Validation for this slice:

- `node --check site/assets/map/map-signal-contract.js`
- `node --check site/assets/map/loader.js`
- `node --test site/assets/map/map-signal-contract.test.mjs site/assets/map/loader.test.mjs site/assets/map/map-host.test.mjs site/assets/js/pages/map-page.test.mjs`
- rebuilt site output
- verified served `loader.js` and served `map-signal-contract.js` both match `site/.out`

## Second implementation slice landed

The next migration step moved the first live control subset away from the transitional
`_map_controls` branch and into `_map_bridged`:

- `_map_bridged.ui.diagnosticsOpen`
- `_map_bridged.ui.viewMode`
- `_map_bridged.filters.fromPatchId`
- `_map_bridged.filters.toPatchId`

What changed:

- the map template now binds those controls directly to `_map_bridged`
- map-page persistence/restore now treats those fields as bridged-owned while keeping the
  existing storage JSON shape stable
- loader render input now merges `_map_bridged` over `_map_controls`, so migrated fields
  behave immediately in the page without waiting for bridge echo
- the `_map_controls -> _map_bridged` projection no longer overwrites those fields
- patch-range normalization now reads and writes `_map_bridged`

Why this slice matters:

- it removes dual ownership for a small, real, bridge-relevant subset
- it proves the transition strategy without needing a big-bang schema rewrite
- it keeps page-only fields like `searchText`, `legendOpen`, and `leftPanelOpen` out of the
  bridged ownership path

What still has not moved yet:

- most layer/filter controls still originate in `_map_controls`
- bookmark manager UI is still page-local but not yet extracted from loader rendering
- `_map_controls` still exists as a transitional branch

Validation for this slice:

- `node --check site/assets/js/pages/map-page.js`
- `node --check site/assets/map/loader.js`
- `node --test site/assets/js/pages/map-page.test.mjs site/assets/map/loader.test.mjs`
- rebuild site output
- verify served `/map/` assets match `site/.out`

## Third implementation slice landed

## 2026-03-31: Live-path cutover landed

The remediation pivot is now explicit:

- `site/layouts/map.shtml` no longer boots the live map through `site/assets/map/loader.js`
- the live page now loads:
  - `site/assets/map/map-app-live.js`

This is the first intentionally breaking step that changes the design center of the map runtime.

### What changed

- Added a new live bootstrap module:
  - `site/assets/map/map-app-live.js`
- That module mounts the Bevy bridge directly through the clean-slate app contract:
  - `site/assets/map/map-app.js`
  - `site/assets/map/map-runtime-adapter.js`
- The map shell now seeds a coarse `_map_runtime` signal branch up front so the new live app can
  project runtime snapshots without relying on loader-owned bootstrap behavior.

### Why this is important

This is the first slice that stops treating `loader.js` as the thing to improve.

Instead:

- `loader.js` is now legacy implementation baggage
- new work should target:
  - the signal contract
  - the runtime adapter
  - the clean-slate map app

That is a better match for Datastar's intended design:

- explicit ownership
- sparse shared signals
- thin side-effect seams
- bridge logic isolated from page UI logic

### Current state after the cutover

- the live page is no longer bootstrapped by `loader.js`
- `loader.js` still exists in the repo, but is no longer the live entrypoint
- page UI behavior is expected to regress temporarily while the clean-slate path replaces the old
  imperative renderer piece by piece

This tradeoff is intentional.

The objective is not "keep shrinking the old loader carefully". The objective is to stop adding
design weight to the old loader and migrate from a new, Datastar-aligned center.

### New rule going forward

For map remediation work:

- do not move responsibilities into `loader.js`
- do not keep extracting 1:1 legacy behavior merely to preserve the old architecture
- prefer implementing behavior in:
  - `map.shtml`
  - `map-page.js`
  - `map-signal-contract.js`
  - `map-runtime-adapter.js`
  - `map-app.js`
  - dedicated small panel/component modules

`loader.js` should only survive temporarily as dead/legacy code until the new path fully
supersedes it.

## 2026-03-31: New live bridge path no longer reads `_map_controls`

After the live cutover, the next clean-slate tightening step was to remove transitional
`_map_controls` fallback from the new runtime adapter path.

Updated:

- `site/assets/map/map-runtime-adapter.js`
- `site/assets/map/map-app-live.js`
- `site/assets/map/map-runtime-adapter.test.mjs`

What changed:

- `buildBridgeInputPatchFromSignals(...)` now reads bridge inputs only from:
  - `_map_bridged`
  - `_map_bookmarks`
  - `_map_ui.bookmarks.selectedIds`
  - `_shared_fish`
- `map-app-live.js` now only reacts to live signal patches from:
  - `_map_bridged`
  - `_map_actions`
  - `_map_bookmarks`
  - `_shared_fish`

What it no longer does:

- it does not inherit bridge inputs from `_map_controls`
- it does not treat `_map_controls` patches as Bevy-facing live updates

Why this matters:

- it makes the clean-slate path more honest
- it flushes out any features that still depend on the transitional branch
- it keeps the new live bootstrap aligned with the explicit shared-signal contract rather than
  silently preserving legacy ownership rules

## 2026-03-31: Map shell moved out of SuperHTML

The next major remediation step was to stop defining the interactive map shell inside
`site/layouts/map.shtml`.

New structure:

- `site/layouts/map.shtml`
  - now acts as a thin wrapper:
    - page chrome
    - styles
    - script tags
    - raw shell include
- `site/assets/map/map-shell.html`
  - now contains the interactive map shell markup
- `site/content/en-US/map.smd`
  - is back to frontmatter-only page metadata

The shell is now included with:

- `:html="$site.asset('map/map-shell.html').bytes()"`

Why this matters:

- SuperHTML was conflicting with literal Datastar `$...` expressions in the shell
- a plain HTML asset gives us a literal DOM source for the interactive shell
- the shell can now evolve toward direct Datastar expressions without fighting the template engine

This is closer to the intended architecture than keeping the shell embedded in a large `.shtml`
layout file.

## 2026-03-31: Raw shell now uses direct Datastar expressions

With the shell living in `site/assets/map/map-shell.html`, ordinary shell interactions no longer
need to route through `window.__fishystuffDatastarState.toggleBooleanPath(...)` and
`setObjectPath(...)`.

Updated in `site/assets/map/map-shell.html`:

- toolbar window toggles
- search-open-on-input / focus
- 2D/3D mode toggle
- reset action tokens
- diagnostics details toggle

These now use direct Datastar expressions such as:

- `$_map_ui.windowUi.search.open = !$_map_ui.windowUi.search.open`
- `$_map_actions.resetUiToken = ... + 1`
- `$_map_bridged.ui.diagnosticsOpen = event.currentTarget.open`

Why this matters:

- it is closer to Datastar's intended FRP style
- it reduces dependence on our custom helper globals for ordinary page mutations
- it validates that moving the shell out of SuperHTML unlocked the exact cleanup the old layout
  structure prevented

## 2026-03-31: Published clean-slate live module chain

After switching the live map bootstrap to `map-app-live.js`, the site initially still failed to
boot because the imported clean-slate modules were not being published as static assets.

Fixed in `site/zine.ziggy`:

- `map/map-app.js`
- `map/map-runtime-adapter.js`

Validation:

- `curl -sSI http://127.0.0.1:1990/map/map-app-live.js`
- `curl -sSI http://127.0.0.1:1990/map/map-app.js`
- `curl -sSI http://127.0.0.1:1990/map/map-runtime-adapter.js`
- live Chromium reload of `/map/` with no module-load errors in the console

The next clean-slate extraction moved the bridge projection logic into a dedicated module:

- `site/assets/map/map-bridge-projection.js`

What moved out of `loader.js`:

- the explicit `_map_controls -> _map_bridged` projection whitelist
- bookmark projection for bridged bookmark payloads
- the pure `projectBridgeSharedInputState(...)` function

Why this extraction matters:

- it gives the bridge-facing input contract its own home instead of burying it in the loader
- it makes later `_map_controls` removal work happen in a focused module instead of the monolith
- it keeps the projection rules explicit and testable

What still remains in `loader.js` after this slice:

- patch listening and dispatch
- bridge mount lifecycle
- large DOM rendering responsibilities
- remaining transitional control ownership

Validation for this slice:

- `node --check site/assets/map/map-bridge-projection.js`
- `node --check site/assets/map/loader.js`
- `node --test site/assets/map/loader.test.mjs site/assets/js/pages/map-page.test.mjs`
- rebuild site output
- verify served loader/module assets match `site/.out`

## Fourth implementation slice landed

The next ownership cleanup moved the remaining global display toggles to bridged ownership:

- `_map_bridged.ui.showPoints`
- `_map_bridged.ui.showPointIcons`
- `_map_bridged.ui.pointIconScale`

What changed:

- map-page persist/restore now reads and writes those values from `_map_bridged.ui`
  while keeping the existing persisted `inputUi` storage shape stable
- the bridge projection no longer derives those values from `_map_controls.ui`
- the `_map_controls` bridge-relevant patch whitelist no longer includes them

Why this slice matters:

- these fields are part of the Bevy contract today, so they should not stay in the
  transitional page-owned branch
- it shrinks `_map_controls.ui` further down toward truly page-local leftovers
- it removes one more source of dual ownership and silent fallback behavior

What still remains after this slice:

- `_map_controls.filters` still contains the majority of bridged layer/filter inputs
- loader still owns most DOM rendering and event wiring
- bookmark manager, layer panel, and search UI still need extraction from loader

Validation for this slice:

- `node --check site/assets/js/pages/map-page.js`
- `node --check site/assets/map/map-bridge-projection.js`
- `node --test site/assets/js/pages/map-page.test.mjs site/assets/map/loader.test.mjs`
- rebuild site output
- verify served assets match `site/.out`

## Fifth implementation slice landed

The next clean-slate extraction moved the layer panel renderer/markup out of the loader:

- `site/assets/map/map-layer-panel.js`

What moved out of `loader.js`:

- the full layer stack card markup renderer
- loading fallback rendering for the layer stack container
- layer-kind labeling for rendered layer cards
- layer settings / visibility / fish-evidence control markup generation

Why this extraction matters:

- it removes one of the largest DOM-rendering blocks from the loader monolith
- it keeps layer panel rendering as a pure function over a state bundle plus icon/loading callbacks
- it sets up the next step where layer panel event wiring can also be reduced or extracted without re-entangling the loader

What still remains after this slice:

- layer panel event delegation still lives in `loader.js`
- bookmark manager and search UI rendering are still loader-owned
- bridge mount/sync lifecycle is still loader-owned

Validation for this slice:

- `node --check site/assets/map/map-layer-panel.js`
- `node --check site/assets/map/loader.js`
- `node --test site/assets/map/loader.test.mjs site/assets/js/pages/map-page.test.mjs site/assets/map/map-signal-contract.test.mjs`
- rebuild site output
- verify served `/map/loader.js` and `/map/map-layer-panel.js` match `site/.out`

## Sixth implementation slice landed

The next clean-slate extraction moved bookmark manager rendering out of the loader:

- `site/assets/map/map-bookmark-panel.js`

What moved out of `loader.js`:

- bookmark manager card list markup rendering
- bookmark manager control-label rendering
- bookmark empty-state rendering

Why this extraction matters:

- bookmark manager UI is page-side state and should not stay buried in the bridge adapter
- it removes another large DOM-rendering block from the loader monolith
- it keeps the bookmark view as a pure rendering module over bookmark/page state plus shared markup callbacks

What still remains after this slice:

- bookmark drag/click/change event wiring still lives in `loader.js`
- bookmark derived metadata helpers still live in `loader.js`
- search UI rendering is still loader-owned

Validation for this slice:

- `node --check site/assets/map/map-bookmark-panel.js`
- `node --check site/assets/map/loader.js`
- `node --test site/assets/map/loader.test.mjs site/assets/js/pages/map-page.test.mjs site/assets/map/map-signal-contract.test.mjs`
- rebuild site output
- verify served `/map/loader.js` and `/map/map-bookmark-panel.js` match `site/.out`

## Seventh implementation slice landed

The next clean-slate extraction moved the map search UI renderer out of the loader:

- `site/assets/map/map-search-panel.js`

What moved out of `loader.js`:

- selected-search-chip rendering
- search result list rendering
- search empty/show-hide rendering behavior

Why this extraction matters:

- search panel UI is page-side state and should not remain coupled to the bridge adapter
- it removes another substantial DOM-rendering block from the loader monolith
- it keeps the search panel renderer as a pure function over signal state plus identity/icon callbacks

What still remains after this slice:

- search matching/query logic still lives in `loader.js`
- search click/keyboard event wiring still lives in `loader.js`
- bookmark and zone-info event wiring still remain loader-owned

Validation for this slice:

- `node --check site/assets/map/map-search-panel.js`
- `node --check site/assets/map/loader.js`
- `node --test site/assets/map/loader.test.mjs site/assets/js/pages/map-page.test.mjs site/assets/map/map-signal-contract.test.mjs`
- rebuild site output
- verify served `/map/loader.js` and `/map/map-search-panel.js` match `site/.out`

## Architectural reset

We are explicitly changing approach here.

The goal is no longer "keep shrinking `loader.js`" or to continue a long series of
1:1 extractions from a legacy imperative file.

The goal is:

- treat `site/assets/map/loader.js` as legacy compatibility code
- define a replacement architecture from the Datastar signal graph outward
- keep the Bevy bridge on a strict explicit whitelist
- move toward a new clean-slate map app path that eventually makes the loader unnecessary

Why this pivot is necessary:

- the remaining bulk of `loader.js` is still imperative orchestration glue
- continuing to prettify or split that glue 1:1 would preserve the wrong design center
- Datastar best practices push us toward sparse explicit signal ownership and narrow side-effect seams, not toward a second client framework wrapped around the signal graph

Replacement direction from this point:

- `map-page.js`
  - restore/persist/bootstrap of durable page-owned signals
- `map-runtime-adapter.js`
  - pure bridge-facing Datastar contract
  - explicit bridge input projection
  - explicit one-shot command projection
  - explicit coarse runtime/session projection
- `map-app.js`
  - clean replacement entrypoint that wires Datastar state to the adapter and the bridge
  - no page UI rendering logic
- legacy `loader.js`
  - compatibility path only until the clean replacement is ready

## Eighth implementation slice landed

The first real clean-slate replacement module is now in place:

- `site/assets/map/map-runtime-adapter.js`
- `site/assets/map/map-app.js`

What this module establishes:

- the Bevy-facing input patch is built from an explicit Datastar signal subset
- bookmark sharing is minimal and independent from hover/runtime snapshots
- bridge commands are derived from action-token state only
- runtime snapshots project back only coarse `_map_runtime` state
- restorable state projects back only `_map_session.view` and `_map_session.selection`
- a clean replacement app entry surface now exists that composes those pure adapter pieces without depending on `loader.js`

Why this slice matters:

- it stops using `loader.js` as the design center for the replacement path
- it gives the future clean `map-app.js` a pure tested contract to build around
- it makes the intended shared-signal boundary explicit and testable

What still remains after this slice:

- the new adapter is not wired into the live map boot path yet
- `_map_controls` still exists as a transitional compatibility branch
- the future `map-app.js` entrypoint does not exist yet

Validation for this slice:

- `node --check site/assets/map/map-runtime-adapter.js`
- `node --check site/assets/map/map-app.js`
- `node --test site/assets/map/map-runtime-adapter.test.mjs`
- `node --test site/assets/map/map-app.test.mjs`

## Ninth implementation slice landed

The live shell and page persistence model are now aligned to the clean-slate signal schema.

What changed:

- `site/assets/map/map-shell.html`
  - removed `_map_controls` from live shell signals
  - search input now binds directly to `_map_ui.search.query`
- `site/assets/js/pages/map-page.js`
  - page persistence now stores only:
    - `_map_ui.windowUi`
    - `_map_ui.layers`
    - `_map_ui.search.query`
    - `_map_bridged.ui`
    - `_map_bridged.filters`
    - `_map_bookmarks.entries`
    - `_map_session`
  - restore now patches only `_map_ui` and `_map_bridged`
  - legacy `inputUi` / `inputFilters` are accepted only as read-time fallback for existing local storage
  - query-owned restore stripping now targets `_map_ui.search.query` and `_map_bridged.*`, not `_map_controls`
- `site/assets/js/pages/map-page.test.mjs`
  - updated to the canonical storage shape
  - still keeps a legacy-storage restore regression

Why this slice matters:

- it removes `_map_controls` from the live map shell entirely
- it makes page-owned persistence match the actual live signal graph
- it narrows the transitional compatibility surface to legacy local-storage reads only

What still remains after this slice:

- other map modules may still reference `_map_controls` as compatibility input
- the live map shell still uses imperative DOM/event behavior outside the bridge contract
- the clean-slate `map-app.js` path still needs to replace the remaining live loader-owned behavior

Validation for this slice:

- `node --check site/assets/js/pages/map-page.js`
- `node --test site/assets/js/pages/map-page.test.mjs`
- `node --test site/assets/map/map-app.test.mjs site/assets/map/map-runtime-adapter.test.mjs site/assets/map/map-signal-contract.test.mjs`
- rebuild site output
- compare served map shell and page module against `site/.out`

## Tenth implementation slice landed

Startup query parsing for page-owned/shared state now lives in a new clean-slate module:

- `site/assets/map/map-query-state.js`

What changed:

- `map-query-state.js` parses the URL into signal patches for:
  - `_map_ui.search.query`
  - `_map_bridged.filters.fishIds`
  - `_map_bridged.filters.fishFilterTerms`
  - `_map_bridged.filters.patchId`
  - `_map_bridged.filters.fromPatchId`
  - `_map_bridged.filters.toPatchId`
  - `_map_bridged.filters.layerIdsVisible`
  - `_map_bridged.ui.diagnosticsOpen`
  - `_map_bridged.ui.viewMode`
- `site/assets/map/map-app-live.js`
  - applies that query-derived signal patch before bridge mount
- `site/zine.ziggy`
  - now publishes `map/map-query-state.js`

Why this slice matters:

- it moves another ownership seam out of the bridge and into Datastar-owned page state
- it keeps query-driven page state aligned with the same signal graph as restore/persist
- it prepares the host contract for a later removal of page-owned query parsing

What still remains after this slice:

- `map-host.js` still parses some overlapping query params as legacy compatibility behavior
- selection/semantic/world-point query commands are still host-owned
- the live map shell still needs more clean-slate replacement of panel behavior beyond bootstrap

Validation for this slice:

- `node --check site/assets/map/map-query-state.js`
- `node --check site/assets/map/map-app-live.js`
- `node --test site/assets/map/map-query-state.test.mjs site/assets/map/map-app.test.mjs site/assets/map/map-runtime-adapter.test.mjs site/assets/js/pages/map-page.test.mjs`
- rebuild site output
- compare served `/map/`, `/map/map-app-live.js`, and `/map/map-query-state.js` against `site/.out`

## Eleventh implementation slice landed

The host-side query parser is now trimmed to true runtime command ownership only.

What changed:

- `site/assets/map/map-host.js`
  - `parseQueryState()` no longer reads page-owned/shared URL fields such as:
    - fish filters
    - patch range
    - search text
    - diagnostics
    - visible layers
    - view mode
  - it now keeps only command-style selection parsing:
    - `zone`
    - `semanticLayer` + `semanticField`
    - `worldX` / `worldZ`
- `site/assets/map/map-host.test.mjs`
  - updated to assert that page-owned/shared query params are ignored by the host
  - kept direct command-query coverage in place

Why this slice matters:

- it removes duplicated ownership between `map-app-live.js` page-side query patching and the host bootstrap path
- it narrows the bridge contract toward explicit runtime concerns
- it makes the startup flow more Datastar-aligned: page/shared URL state first, bridge commands second

What still remains after this slice:

- `map-host.js` still carries stale contract fields in its broader input-state model
- selection/world-point query commands are still host-owned
- the live shell still needs clean-slate replacements for more interactive panel behavior

Validation for this slice:

- `node --check site/assets/map/map-host.js`
- `node --test site/assets/map/map-host.test.mjs site/assets/map/map-query-state.test.mjs site/assets/map/map-app.test.mjs site/assets/map/map-runtime-adapter.test.mjs site/assets/js/pages/map-page.test.mjs`
- rebuild site output
- compare served `/map/`, `/map/map-app-live.js`, `/map/map-host.js`, and `/map/map-query-state.js` against `site/.out`

## Fourteenth implementation slice landed

The live bridge path no longer writes back through the page global.

What changed:

- `site/assets/map/map-signal-patch.js`
  - new dedicated shell patch module for the clean-slate map path
  - exports:
    - `FISHYMAP_SIGNAL_PATCH_EVENT`
    - `dispatchShellSignalPatch(...)`
    - `combineSignalPatches(...)`
- `site/assets/map/map-app-live.js`
  - query-state patches now dispatch directly to the shell
  - runtime/session projections from Bevy now dispatch directly to the shell
  - reset-UI signal handling now dispatches the reset patch directly to the shell
  - the live write path no longer depends on `window.__fishystuffMap.patchSignals(...)`
- `site/assets/map/map-signal-patch.test.mjs`
  - added coverage for shell patch event dispatch and patch combination

Why this slice matters:

- it reduces the `window.__fishystuffMap` global surface in the live map path
- it makes the shell, not the page-global helper, the one explicit write boundary for live signal updates
- it keeps the clean-slate architecture moving away from the old "global helper as app bus" pattern

What still remains after this slice:

- `window.__fishystuffMap` is still needed for restore/bootstrap and snapshot reads
- `map-app-live.js` still owns a fair amount of orchestration that could later move into smaller shell/runtime controller modules
- substantial panel/result markup is still rendered by imperative helper modules rather than fully declarative shell bindings

Validation for this slice:

- `node --check site/assets/map/map-app-live.js`
- `node --check site/assets/map/map-signal-patch.js`
- `node --test site/assets/map/map-signal-patch.test.mjs site/assets/map/map-app.test.mjs site/assets/js/pages/map-page.test.mjs site/assets/map/map-shell.test.mjs`
- rebuild site output
- live browser checks:
  - map boots without console errors
  - dispatching `fishymap-signals-patch` on `#map-page-shell` updates the live DOM
  - Bevy/runtime sync still flows through the shell after reload

## Fifteenth implementation slice landed

Managed window behavior now has its own clean-slate module instead of remaining an implicit loader-era gap.

What changed:

- `site/assets/map/map-window-manager.js`
  - new dedicated window manager for the live map shell
  - owns:
    - bringing windows to front
    - drag titlebar handling
    - persisting dragged `x/y` back into `_map_ui.windowUi`
    - collapse-on-tap for non-search windows
    - applying `_map_ui.windowUi` positions back onto the real shell elements
- `site/assets/map/map-window-manager.test.mjs`
  - added helper coverage for:
    - bounds clamping
    - normalized window-ui patch generation
- `site/assets/map/map-app-live.js`
  - now boots the new window manager
  - live shell no longer depends on loader-era window management behavior

Why this slice matters:

- it replaces one more concrete loader responsibility with a focused clean-slate module
- it makes `_map_ui.windowUi` meaningful again in the live shell
- it keeps managed-window behavior aligned with Datastar ownership:
  - shell state lives in signals
  - the window manager just interprets pointer intent and patches those signals

What still remains after this slice:

- bookmark, search-result, and zone-info result rendering still rely on imperative modules
- the new window manager is still JS-owned behavior rather than declarative shell expressions, though that is appropriate for drag interaction
- the page-global bootstrap surface can still be reduced further over time

Validation for this slice:

- `node --check site/assets/map/map-window-manager.js`
- `node --check site/assets/map/map-app-live.js`
- `node --test site/assets/map/map-window-manager.test.mjs site/assets/map/map-signal-patch.test.mjs site/assets/js/pages/map-page.test.mjs site/assets/map/map-shell.test.mjs`
- rebuild site output
- live browser checks:
  - shell patching `_map_ui.windowUi.settings.{x,y}` moves the Settings window after the next animation frame
  - shell patching `_map_ui.windowUi.layers.collapsed = true` hides the Layers body

## Thirteenth implementation slice landed

The map page no longer uses a disconnected JS-owned signal store as its source of truth.

What changed:

- `site/assets/js/pages/map-page.js`
  - removed the page-local Datastar signal store as the live owner
  - `restore($)` now connects to the actual shell signal graph and keeps only:
    - a cloned snapshot cache for JS consumers
    - restore logic
    - persistence logic
  - `patchSignals(...)` now prefers dispatching a shell-scoped custom event instead of mutating a fake store
  - `applyPatch($, patch)` now mutates the live Datastar signal object passed from the shell
- `site/assets/map/map-shell.html`
  - root shell now handles `fishymap-signals-patch` declaratively:
    - `data-on:fishymap-signals-patch="window.__fishystuffMap.applyPatch($, evt.detail)"`
- `site/assets/map/map-shell.test.mjs`
  - added shell-level assertions for the new live patch hook

Why this slice matters:

- it restores the shell as the real owner of live UI state
- it fixes the concrete regression where:
  - `window.__fishystuffMap.patchSignals(...)` changed JS state
  - but bound Datastar DOM did not update
- it replaces a major conceptual drift point with a cleaner Datastar-aligned seam:
  - page JS emits intent to the shell
  - the shell mutates the live signal graph
  - Datastar propagates DOM updates and signal-patch events from there

What still remains after this slice:

- `map-app-live.js` still depends on the legacy `window.__fishystuffMap` bootstrap surface
- the shell still contains substantial imperative subtrees rendered by legacy modules
- more shell behavior still needs to move from extracted legacy renderers toward truly Datastar-owned UI

Validation for this slice:

- `node --check site/assets/js/pages/map-page.js`
- `node --check site/assets/map/map-app-live.js`
- `node --test site/assets/js/pages/map-page.test.mjs site/assets/map/map-shell.test.mjs site/assets/map/map-app.test.mjs site/assets/map/map-runtime-adapter.test.mjs`
- rebuild site output
- live browser checks:
  - `window.__fishystuffMap.patchSignals({ _map_ui.windowUi.search.open = true })` now opens the real Search window
  - clicking the Search toolbar button updates both the Datastar snapshot and the DOM

## Twelfth implementation slice landed

The host input/snapshot model no longer carries stale page-only fields.

What changed:

- `site/assets/map/map-host.js`
  - removed `filters.searchText`
  - removed `ui.legendOpen`
  - removed `ui.leftPanelOpen`
  - cut those fields out of:
    - typedef documentation
    - `createEmptyInputState()`
    - `createEmptySnapshot()`
    - `normalizeStatePatch()`
    - `applyStatePatch()`
    - host diagnostic input-state summaries
- `site/assets/map/map-host.test.mjs`
  - updated assertions so those fields are now explicitly absent from bridge-owned input state

Why this slice matters:

- it removes phantom bridge state that no longer belongs to the explicit shared-signal whitelist
- it tightens the host contract beyond startup parsing and into the actual runtime model
- it makes the clean-slate map path less likely to regress by accidentally reusing page-only fields at the bridge layer

What still remains after this slice:

- `map-host.js` still owns some compatibility behaviors beyond the strict clean-slate target
- selection/world-point query commands are still host-owned
- more shell behavior still needs to move out of legacy imperative handling into clean Datastar-owned modules

Validation for this slice:

- `node --check site/assets/map/map-host.js`
- `node --test site/assets/map/map-host.test.mjs site/assets/map/map-query-state.test.mjs site/assets/map/map-app.test.mjs site/assets/map/map-runtime-adapter.test.mjs site/assets/js/pages/map-page.test.mjs`
- rebuild site output
- compare served `/map/`, `/map/map-app-live.js`, `/map/map-host.js`, and `/map/map-query-state.js` against `site/.out`

## Sixteenth implementation slice landed

Zone Info now runs through the clean-slate live map app instead of relying on the legacy loader-era rendering path.

What changed:

- `site/assets/map/map-zone-info-state.js`
  - added a pure zone-info view-model builder driven from:
    - `_map_runtime.selection`
    - `_map_runtime.catalog.layers`
    - `_map_ui.windowUi.zoneInfo.tab`
  - derives:
    - title / status text
    - active tab selection
    - a generic facts list for the active layer sample
  - ignores empty default runtime coordinates unless there is a meaningful `pointKind`
- `site/assets/map/map-zone-info-panel-live.js`
  - added a focused live controller for:
    - tab rendering
    - facts rendering
    - active-tab updates back into `_map_ui.windowUi.zoneInfo.tab`
- `site/assets/map/map-app-live.js`
  - now boots the zone-info controller
  - schedules it after bridge-originated shell patches alongside the other clean-slate live panels
- `site/zine.ziggy`
  - now publishes the zone-info live modules

Why this slice matters:

- it removes another visible shell responsibility from the legacy loader path
- it keeps zone inspection aligned with the Datastar ownership model:
  - Bevy publishes coarse selection/runtime state
  - the shell derives presentation from signals
  - tab choice is page-owned UI state
- it confirms the live shell is stable across selection updates:
  - current live build keeps the layer catalog present before and after point selection
  - the old “selection click repopulates Layers” regression was not present in the validated served build

What still remains after this slice:

- `map-page.js` still exposes a broader bootstrap/global surface than the final target
- the live path still depends on custom panel controllers instead of more direct Datastar-owned shell markup
- some shell behaviors still need to move from JS controllers toward direct signal expressions or smaller dedicated modules

Validation for this slice:

- `node --check site/assets/map/map-zone-info-state.js`
- `node --check site/assets/map/map-zone-info-panel-live.js`
- `node --check site/assets/map/map-app-live.js`
- `node --test site/assets/map/map-zone-info-state.test.mjs site/assets/map/map-bookmark-state.test.mjs site/assets/map/map-runtime-adapter.test.mjs site/assets/map/map-app.test.mjs site/assets/js/pages/map-page.test.mjs`
- rebuild site output
- compare served `/map/`, `/map/map-app-live.js`, `/map/map-zone-info-state.js`, and `/map/map-zone-info-panel-live.js` against `site/.out`

## Seventeenth implementation slice landed

The map page’s restore/persist contract now has its own clean-slate state module instead of living inline inside the large page bootstrap script.

What changed:

- `site/assets/map/map-page-state.js`
  - new pure page-state helper module for:
    - durable UI snapshot extraction
    - UI/session storage snapshot serialization
    - UI/session restore patch generation
    - stripping query-owned restore fields
  - now owns the map page’s default enabled layer fallback for persisted bridged filters
- `site/assets/js/pages/map-page.js`
  - no longer carries the restore/persist shape logic inline
  - now delegates those transforms to `window.__fishystuffMapPageState`
- `site/layouts/map.shtml`
  - loads the new page-state asset before `map-page.js`
- `site/assets/js/pages/map-page.test.mjs`
  - now loads the extracted helper before the bootstrap script
- `site/assets/map/map-page-state.test.mjs`
  - adds direct coverage for the new helper module

Why this slice matters:

- it continues the clean-slate remediation outside `loader.js`, not just around the live shell
- it reduces the amount of implicit page-global state logic hidden inside `map-page.js`
- it gives the map page a more explicit functional boundary:
  - shell signal graph stays live
  - restore/persist transforms live in a dedicated state helper

What still remains after this slice:

- `map-page.js` still owns patch filtering, restore sequencing, and the `window.__fishystuffMap` bootstrap surface
- the shell still depends on `window.__fishystuffMap.applyPatch(...)` for external shell-scoped patch events
- more of the bootstrap surface can still move into smaller dedicated modules over time

Validation for this slice:

- `node --check site/assets/map/map-page-state.js`
- `node --check site/assets/js/pages/map-page.js`
- `node --test site/assets/map/map-page-state.test.mjs site/assets/js/pages/map-page.test.mjs site/assets/map/map-app.test.mjs site/assets/map/map-runtime-adapter.test.mjs`
- rebuild site output
- compare served `/map/`, `/map/map-page-state.js`, and `/js/pages/map-page.js` against `site/.out`

## Eighteenth implementation slice landed

The map page bootstrap no longer depends directly on the shared Datastar helper for signal patch application and persistence filtering.

What changed:

- `site/assets/map/map-page-signals.js`
  - new clean-slate signal-ops helper for the map page bootstrap
  - now owns:
    - deep patch application into the live shell signal graph
    - exact-path replacements for array/object branches that must replace, not merge
    - persistence-filter matching for the map page’s durable signal subset
- `site/assets/js/pages/map-page.js`
  - now delegates signal patch application and persistence filtering to `window.__fishystuffMapPageSignals`
  - no longer reaches into `window.__fishystuffDatastarState.mergeObjectPatch(...)`
- `site/layouts/map.shtml`
  - loads the new signal-ops asset before `map-page.js`
- `site/assets/map/map-page-signals.test.mjs`
  - adds direct coverage for the new signal-ops helper
- `site/assets/js/pages/map-page.test.mjs`
  - now boots the extracted signal-ops helper alongside the extracted page-state helper

Why this slice matters:

- it removes one more piece of generic helper indirection from the live map bootstrap
- it makes the map page’s signal mutation rules explicit and local to the map remediation path
- it keeps the clean-slate work focused on the live map surface instead of relying on legacy shared helper behavior

What still remains after this slice:

- `map-page.js` still owns restore sequencing, persistence scheduling, and the `window.__fishystuffMap` bootstrap surface
- the shell still routes external shell-scoped patches through `window.__fishystuffMap.applyPatch(...)`
- more of the bootstrap surface can still move into smaller map-specific modules

Validation for this slice:

- `node --check site/assets/map/map-page-signals.js`
- `node --check site/assets/js/pages/map-page.js`
- `node --test site/assets/map/map-page-signals.test.mjs site/assets/map/map-page-state.test.mjs site/assets/js/pages/map-page.test.mjs site/assets/map/map-app.test.mjs site/assets/map/map-runtime-adapter.test.mjs`
- rebuild site output
- compare served `/map/`, `/map/map-page-signals.js`, and `/js/pages/map-page.js` against `site/.out`

## Nineteenth implementation slice landed

The live shell no longer routes custom shell patch events back through `window.__fishystuffMap.applyPatch(...)`.

What changed:

- `site/assets/map/map-shell.html`
  - now applies `fishymap-signals-patch` events directly through:
    - `window.__fishystuffMapPageSignals.applyPatchToSignals($, evt.detail)`
- `site/assets/js/pages/map-page.js`
  - no longer exposes `applyPatch` on `window.__fishystuffMap`
  - keeps the public bootstrap surface smaller:
    - `signalObject`
    - `patchSignals`
    - `restore`
    - `whenRestored`

Why this slice matters:

- it removes one more unnecessary callback through the page bootstrap global
- it makes the shell’s live patch handling use the dedicated map signal-ops helper directly
- it continues the Datastar-aligned direction:
  - shell event -> signal helper -> live signal graph
  - no extra page-global indirection in the middle

What still remains after this slice:

- `map-page.js` still owns restore sequencing, persistence scheduling, and the remaining bootstrap global
- `map-app-live.js` still depends on `window.__fishystuffMap` for restore readiness and signal access
- more of the bootstrap can still be split into smaller map-specific modules if needed

Validation for this slice:

- `node --check site/assets/js/pages/map-page.js`
- `node --test site/assets/map/map-page-signals.test.mjs site/assets/js/pages/map-page.test.mjs`
- rebuild site output
- compare served `/map/` and `/map/map-shell.html` against `site/.out`
- live Chromium reload showed:
  - no new console errors
  - map shell windows render normally
  - `Layers 7` and `Zone Info` are present immediately after boot

## Twentieth implementation slice landed

The live bookmark panel now owns copy/export/import again through a clean-slate module instead of
leaving those flows disabled while the old loader path is being retired.

What changed:

- `site/assets/map/map-bookmark-io.js`
  - new pure bookmark I/O helper for:
    - WorldmapBookMark XML serialization
    - XML / JSON import parsing
    - duplicate-aware bookmark merging
    - clipboard export
    - browser download export
    - file import reads
    - small user-facing status message builders
- `site/assets/map/map-bookmark-panel-live.js`
  - now wires the live Bookmark Manager to:
    - copy selected bookmarks as XML
    - copy a single bookmark as XML
    - export selected or all bookmarks as XML
    - import bookmark XML and merge it into the live Datastar bookmark state
  - newly imported bookmarks are selected immediately
  - import cancels placing mode so the panel stays internally consistent after merge
- `site/assets/map/map-bookmark-io.test.mjs`
  - adds direct coverage for the new bookmark I/O helper

Why this slice matters:

- bookmark manager behavior is now restored without routing back through `loader.js`
- bookmark copy/export/import are again owned by Datastar-backed page state
- the live panel can now replace more of the old imperative bookmark toolchain instead of only
  rendering cards

Validation for this slice:

- `node --check site/assets/map/map-bookmark-io.js`
- `node --check site/assets/map/map-bookmark-panel-live.js`
- `node --test site/assets/map/map-bookmark-io.test.mjs site/assets/map/map-bookmark-state.test.mjs site/assets/js/pages/map-page.test.mjs`
- rebuild site output
- compare served `/map/map-bookmark-io.js` and `/map/map-bookmark-panel-live.js` against `site/.out`
- live Chromium checks confirmed:
  - `Export` is enabled when bookmarks exist
  - `Import` is enabled
  - per-bookmark `Copy bookmark XML` is enabled
  - top-level `Copy` remains correctly disabled until something is selected

## Twenty-first implementation slice landed

This approach was reverted in the next slice. Treating runtime view mode as authoritative by
writing it back into `_map_bridged.ui.viewMode` was conceptually wrong for the clean-slate
contract because `_map_bridged` is input-owned and should not be mutated from runtime output.

What changed:

- `site/assets/map/map-runtime-adapter.js`
  - runtime view mode was temporarily mirrored back into `_map_bridged.ui.viewMode`
- `site/assets/map/map-runtime-adapter.test.mjs`
  - temporarily asserted that runtime snapshot projection kept `_map_bridged.ui.viewMode`
    aligned with the mounted runtime

Why this slice matters:

- it exposed that input-vs-output ownership was still muddy around map view mode
- that directly informed the next corrective slice, which restores the explicit Datastar contract:
  - `_map_bridged` stays input-owned
  - `_map_runtime` and `_map_session` stay output/session-owned

Validation for this slice:

- `node --check site/assets/map/map-runtime-adapter.js`
- `node --test site/assets/map/map-runtime-adapter.test.mjs site/assets/js/pages/map-page.test.mjs`
- rebuild site output
- compare served `/map/map-runtime-adapter.js` against `site/.out`
- live Chromium probe with intentionally conflicting persisted state confirmed that after reload:
  - `_map_bridged.ui.viewMode`
  - `_map_runtime.view.viewMode`
  - `_map_session.view.viewMode`
  all converge on the actual runtime-mounted view mode

## Twenty-second implementation slice landed

The clean-slate map app now resets safely and keeps runtime view state aligned without mutating
the explicit input-owned `_map_bridged` branch from runtime snapshots.

What changed:

- `site/assets/map/map-app-live.js`
  - added an explicit internal signal-patch guard so `Reset UI` can apply a full reset patch
    without recursively re-entering the document-level Datastar signal listener
  - reset now applies the clean-slate signal patch directly, then performs one bridge sync from the
    resulting signal graph
- `site/assets/map/map-runtime-adapter.js`
  - `projectRuntimeSnapshotToSignals(...)` no longer writes runtime view mode into
    `_map_bridged.ui.viewMode`
  - runtime output stays in `_map_runtime`, and restorable runtime state stays in `_map_session`
- `site/assets/map/map-shell.html`
  - `Reset view` / `Reset UI` action-token expressions now use direct Datastar field increments
- `site/assets/map/map-shell.test.mjs`
  - updated for the new direct signal helper path and reset token expressions
- `site/assets/map/map-runtime-adapter.test.mjs`
  - updated to assert the corrected runtime projection contract
- `site/assets/map/map-signal-contract.js`
  - page-local search state now keeps its `query` field in the normalized default shape
- `site/assets/map/map-signal-contract.test.mjs`
  - updated to assert that normalized map UI state preserves `search.query`

Why this slice matters:

- it fixes the live recursion flood (`Maximum call stack size exceeded`) on map boot
- it restores the intended Datastar ownership model:
  - `_map_bridged` = page/user intent into Bevy
  - `_map_runtime` = coarse current runtime snapshot out of Bevy
  - `_map_session` = coarse restorable runtime snapshot
- it fixes the previously broken reset flow where:
  - the shell reset to 2D
  - but the runtime could stay stuck in 3D
- it removes a major source of stale loading state drift in the live map shell

Validation for this slice:

- `node --test site/assets/map/map-runtime-adapter.test.mjs site/assets/map/map-app-live.test.mjs site/assets/map/map-shell.test.mjs site/assets/js/pages/map-page.test.mjs`
- rebuild site output
- live Chromium checks confirmed:
  - no recursion errors on reload
  - `Layers 7` and `Settings Ready` stay present after boot
  - 2D/3D toggle is acknowledged by the runtime
  - `Reset UI` now returns:
    - `_map_runtime.view.viewMode`
    - `_map_session.view.viewMode`
    - the visible shell state
    back to `2d`
  - canvas drag no longer kicked the Layers or Settings panes back into indefinite loading

## Twenty-third implementation slice landed

The clean-slate live app now mounts the canonical global bridge singleton again, so the rest of the
repo and the browser profiling harness can see the real active runtime state.

What changed:

- `site/assets/map/map-app-live.js`
  - now uses the default exported `FishyMapBridge` singleton from `map-host.js` instead of
    creating a private bridge instance with `createFishyMapBridge()`

Why this slice matters:

- the live map shell and the external tooling now agree on the active runtime instance
- this restores the documented bridge contract used by:
  - `site/README.md`
  - `tools/scripts/map_browser_smoke.py`
  - `tools/scripts/map_browser_profile.py`
- it keeps the clean-slate app explicit:
  - one mounted bridge
  - one observable runtime state
  - no hidden second bridge instance behind the shell

Validation for this slice:

- `node --test site/assets/map/map-app-live.test.mjs site/assets/map/map-runtime-adapter.test.mjs site/assets/map/map-shell.test.mjs site/assets/js/pages/map-page.test.mjs`
- rebuild site output
- live Chromium checks confirmed that:
  - `window.FishyMapBridge.getCurrentState()` now reflects the mounted runtime
  - search selection, bookmark placing, layer toggles, window state, and reset still work
- browser profiling harness:
  - `python3 tools/scripts/map_browser_profile.py load_map --output-json /tmp/map-load.json`
    - `PASS`
    - `frame_avg_ms=107.646`
    - `p95_ms=120.700`
  - `python3 tools/scripts/map_browser_profile.py zone_mask_hover_sweep --timeout-seconds 90 --output-json /tmp/map-hover.json`
    - `PASS`
    - `frame_avg_ms=6.561`
    - `p95_ms=10.700`

Next recommended tasks from here:

- keep replacing remaining page-global bootstrap seams in `map-page.js` with smaller map-specific
  modules
- continue deleting legacy loader-era assumptions from tests/docs now that the live page no longer
  depends on `loader.js`

## Twenty-fourth implementation slice landed

The shell no longer owns custom `fishymap-signals-patch` application through a global template
hook. That responsibility now lives in `map-page.js`, where the page-owned signal graph already
handles restore/persist logic.

What changed:

- `site/assets/js/pages/map-page.js`
  - now binds a `fishymap-signals-patch` listener on `#map-page-shell`
  - shell-dispatched patches are applied into the live Datastar signal graph there, instead of
    through a template-level global expression
- `site/assets/map/map-shell.html`
  - removed:
    - `data-on:fishymap-signals-patch="window.__fishystuffMapPageSignals.applyPatchToSignals($, evt.detail)"`
- `site/assets/js/pages/map-page.test.mjs`
  - now covers shell-dispatched map patches flowing into the live signal graph
- `site/assets/map/map-shell.test.mjs`
  - now asserts that the shell no longer carries that global patch-application attribute

Why this slice matters:

- it removes one more imperative/global hook from the raw shell HTML
- it keeps signal mutation inside the page-owned module instead of the template
- it makes the shell more purely declarative:
  - `data-signals`
  - direct Datastar expressions
  - no custom signal-application global in markup

Validation for this slice:

- `node --test site/assets/js/pages/map-page.test.mjs site/assets/map/map-shell.test.mjs site/assets/map/map-app-live.test.mjs site/assets/map/map-runtime-adapter.test.mjs`
- rebuild site output
- live Chromium reload confirmed:
  - no new console errors
  - `window.FishyMapBridge.getCurrentState().ready === true`
  - `Layers 7` and `Settings Ready` still render immediately
  - persisted shell state still restores correctly after reload

## Twenty-fifth implementation slice landed

The map page bootstrap global is smaller again. Shell-dispatched signal patches still flow into the
live Datastar graph, but the public `window.__fishystuffMap` surface no longer exports a direct
`patchSignals(...)` mutator.

What changed:

- `site/assets/js/pages/map-page.js`
  - kept `patchSignals(...)` as an internal helper only
  - narrowed `window.__fishystuffMap` back to:
    - `signalObject`
    - `restore`
    - `whenRestored`
- `site/assets/js/pages/map-page.test.mjs`
  - now dispatches `fishymap-signals-patch` directly on the shell instead of calling the page
    global

Why this slice matters:

- the live shell now depends on the actual Datastar/event contract instead of a wider page-global
  escape hatch
- tests exercise the same signal-patch path the live shell uses
- this keeps the clean-slate map app aligned with the goal of deleting imperative loader-era glue
  instead of preserving it behind page globals

Validation for this slice:

- `node --test site/assets/js/pages/map-page.test.mjs site/assets/map/map-shell.test.mjs site/assets/map/map-app-live.test.mjs site/assets/map/map-runtime-adapter.test.mjs`
- `node --check site/assets/js/pages/map-page.js`
- rebuild site output
- spot-check served `/js/pages/map-page.js` against `site/.out`

## Twenty-sixth implementation slice landed

The clean-slate live map app now waits explicitly for the page bootstrap contract instead of
assuming the head module runs after the deferred page bootstrap scripts.

What changed:

- `site/assets/map/map-app-live.js`
  - added `waitForMapPageBootstrap(...)`
  - `start()` now waits for:
    - `window.__fishystuffMap.whenRestored`
    - `window.__fishystuffMapPageSignals.applyPatchToSignals`
  - this removes another piece of accidental script-order coupling between the head module and the
    Datastar/page bootstrap scripts
- `site/assets/map/map-app-live.test.mjs`
  - now covers delayed appearance of the page bootstrap globals

Why this slice matters:

- the clean-slate app should not rely on classic-defer vs module execution timing luck
- this makes the live page more robust after moving map scripts into the head
- it narrows the remaining fresh-boot failures to the runtime/headless path rather than the page
  bootstrap path

Validation for this slice:

- `node --test site/assets/map/map-app-live.test.mjs site/assets/js/pages/map-page.test.mjs site/assets/map/map-shell.test.mjs site/assets/map/map-runtime-adapter.test.mjs`
- `node --check site/assets/map/map-app-live.js`
- rebuild site output
- live Chromium reload still reached:
  - `window.FishyMapBridge.getCurrentState().ready === true`
  - `window.__fishystuffMap` keys limited to:
    - `signalObject`
    - `restore`
    - `whenRestored`

Open follow-up after this slice:

- headless `map-browser-smoke` / `load_map` still time out with:
  - `ready=false`
  - layer/meta/fish catalogs pending
  - `points: snapshot loading`
- because this remained unchanged after the bootstrap wait fix, the remaining issue is now more
  likely inside the headless runtime mount / WebGL path than in the page-global boot order

## Twenty-seventh implementation slice landed

The live map app no longer reaches through `window.__fishystuffMapPageSignals` to mutate the shell.
Internal live-map patches now go through the same shell signal-patch event contract as every other
live map patch.

What changed:

- `site/assets/map/map-app-live.js`
  - removed the live dependency on `window.__fishystuffMapPageSignals`
  - `applyInternalSignalPatch(...)` now dispatches `fishymap-signals-patch` on the shell instead of
    calling the helper global directly
  - `waitForMapPageBootstrap(...)` now only waits for `window.__fishystuffMap.whenRestored`
- `site/assets/map/map-app-live.test.mjs`
  - updated to reflect the smaller bootstrap dependency

Why this slice matters:

- the live app now uses one consistent shell patch path:
  - shell event
  - page-owned signal application
  - Datastar signal graph update
- this removes another loader-era escape hatch from the clean-slate path
- the remaining helper global is now page-internal rather than part of the live map app contract

Validation for this slice:

- `node --test site/assets/map/map-app-live.test.mjs site/assets/js/pages/map-page.test.mjs site/assets/map/map-shell.test.mjs site/assets/map/map-runtime-adapter.test.mjs`
- `node --check site/assets/map/map-app-live.js`
- rebuild site output
- live Chromium reload confirmed:
  - `window.FishyMapBridge.getCurrentState().ready === true`
  - layer catalog length remained `7`
  - the live shell still booted with no new console errors
