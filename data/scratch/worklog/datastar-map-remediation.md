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
