# Datastar Map Remediation

Last updated: 2026-04-01

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

### 2026-03-31: Clean-slate live map restored to product parity

The live `/map/` page is now running on the clean-slate Datastar path instead of relying on the
legacy `loader.js` page bootstrap.

Current live shape:

- shell:
  - `site/assets/map/map-shell.html`
- page bootstrap:
  - `site/assets/map/map-page-live.js`
- live app orchestration:
  - `site/assets/map/map-app-live.js`
- live controllers:
  - `site/assets/map/map-window-manager.js`
  - `site/assets/map/map-layer-panel-live.js`
  - `site/assets/map/map-search-panel-live.js`
  - `site/assets/map/map-bookmark-panel-live.js`
  - `site/assets/map/map-zone-info-panel-live.js`

Important restoration fixes that landed during this phase:

- shell-originated signal patches are re-emitted as `datastar-signal-patch` so live controllers
  stay in sync with ordinary Datastar patch flow
- bookmark placement now consumes coarse runtime selection changes without reintroducing
  hover-driven shared state
- bridge events that only carry partial state, such as `fishymap:view-changed`, are resolved
  against the current full bridge snapshot before projecting back into `_map_runtime`

Validation checkpoint:

- served assets matched `site/.out` for the active map runtime files:
  - `/map/map-app-live.js`
  - `/map/map-page-live.js`
  - `/map/map.css`
- live DevTools checks confirmed:
  - map boots to `Ready`
  - layer catalog reaches `7`
  - fish catalog reaches `496`
  - toolbar window visibility state is reflected immediately
  - layer toggles and per-layer settings update runtime state immediately
  - fish-evidence icon size updates the fish-evidence runtime path without mutating waypoint icon
    rendering
  - search results render live and selecting a result updates `_map_bridged.filters`
  - zone info updates from runtime selection on the live shell
  - bookmark placement and bookmark focus work again
  - `Reset UI` returns to `Ready`
- sequential browser scenarios passed:
  - `bash tools/scripts/map-browser-smoke.sh`
  - `python3 tools/scripts/map_browser_profile.py load_map --output-json /tmp/map-load.current.json`
  - `python3 tools/scripts/map_browser_profile.py zone_mask_hover_sweep --timeout-seconds 90 --output-json /tmp/map-hover.current.json`
  - `python3 tools/scripts/map_browser_profile.py zone_mask_hover_far_jumps --output-json /tmp/map-hover-far-jumps.current.json`
  - `python3 tools/scripts/map_browser_profile.py minimap_enable --output-json /tmp/map-minimap-enable.current.json`
  - `python3 tools/scripts/map_browser_profile.py vector_regions_enable --output-json /tmp/map-vector-regions-enable.current.json`
  - `python3 tools/scripts/map_browser_profile.py vector_region_groups_enable --output-json /tmp/map-vector-region-groups-enable.current.json`
  - `python3 tools/scripts/map_browser_profile.py vector_region_groups_dom_toggle --output-json /tmp/map-vector-region-groups-dom-toggle.current.json`

Profiling note:

- `minimap_pan_zoom` passed with a larger timeout:
  - `python3 tools/scripts/map_browser_profile.py minimap_pan_zoom --timeout-seconds 90 --output-json /tmp/map-minimap-pan-zoom.long.json`
- default-timeout failures here look like harness timing rather than a live map regression
- concurrent profile runs can still exhaust headless Chromium shared-image resources, so the
  restoration sweep should stay sequential

Current conclusion:

- map functionality is restored on the clean-slate Datastar path
- the next work is no longer parity restoration
- the next work is simplifying the remaining bootstrap/orchestration seams without widening the
  bridge contract again

### 2026-03-31: Live map bootstrap moved off the broader page-global surface

The clean-slate live map no longer depends on the broader `window.__fishystuffMap` bootstrap
global.

What changed:

- `site/assets/map/map-shell.html`
  - the Datastar init hook now calls:
    - `window.__fishystuffMapLiveRestore($)`
- `site/assets/map/map-page-live.js`
  - the restore hook now attaches a shell-scoped signal API on `#map-page-shell`:
    - `shell.__fishystuffMapPage.signalObject()`
    - `shell.__fishystuffMapPage.whenRestored()`
  - removed the old broader page-global live API surface
- `site/assets/map/map-app-live.js`
  - now waits for the shell-scoped signal API instead of polling `window.__fishystuffMap`

Why this matters:

- it keeps the live Datastar signal graph owned by the shell subtree instead of the page global
- it narrows the clean-slate bootstrap surface to the one hook still needed for Datastar init
- it makes the runtime app depend on the shell contract directly, which is closer to the intended
  FRP shape than a page-global helper object

Validation:

- `node --check site/assets/map/map-page-live.js`

### 2026-03-31: Restored page-owned hover facts with per-layer toggles

The clean-slate live map now restores hover facts without pushing transient hover data through the
Datastar signal graph.

What changed:

- added:
  - `site/assets/map/map-hover-facts.js`
  - `site/assets/map/map-hover-tooltip-live.js`
- `site/assets/map/map-shell.html`
  - `_map_ui.layers` now includes:
    - `hoverFactsVisibleByLayer`
- `site/assets/map/map-page-live.js`
  - persists/restores `_map_ui.layers.hoverFactsVisibleByLayer`
- `site/assets/map/map-layer-panel.js`
  - layer settings now render a `Hover facts` table per layer when that layer can contribute facts
- `site/assets/map/map-layer-panel-live.js`
  - toggle interactions now patch `_map_ui.layers.hoverFactsVisibleByLayer`
- `site/assets/map/map-app-live.js`
  - boots the new hover tooltip controller
  - wires zone-catalog data into both the layer settings preview and hover tooltip controller

Signal/bridge contract decisions:

- hover remains transient and **does not** enter `_map_runtime`
- fact visibility is page-owned UI state and **does not** cross `_map_bridged`
- the runtime/host still emits coarse `fishymap:hover-changed` events
- the shell hover controller consumes that coarse event directly and renders the tooltip locally

Current supported fact toggles:

- `zone_mask`
  - `Zone Name`
  - `RGB`
- `region_groups`
  - `Resources`
- `regions`
  - `Origin`

Design notes:

- fact toggles are intentionally per-layer and per-fact so more layer-specific facts can be added
  later without changing the bridge contract
- hover rows are ordered by layer stack using the same layer ordering helper as the rest of the
  clean-slate map, with the lowest layer rendered first
- the settings preview is derived from current selection/runtime sample data rather than hover so
  it stays deterministic and avoids reintroducing hover-driven bridge churn

Validation:

- `node --check site/assets/map/map-hover-facts.js`
- `node --check site/assets/map/map-hover-tooltip-live.js`
- `node --check site/assets/map/map-layer-panel-live.js`
- `node --check site/assets/map/map-layer-panel.js`
- `node --check site/assets/map/map-app-live.js`
- `node --check site/assets/map/map-page-live.js`
- `node --test site/assets/map/map-hover-facts.test.mjs site/assets/map/map-layer-panel-live.test.mjs site/assets/map/map-page-live.test.mjs site/assets/map/map-shell.test.mjs site/assets/map/map-app-live.test.mjs`
- rebuilt site output
- restored tracked font artifacts after rebuild
- verified served:
  - `/map/map-app-live.js`
  - `/map/map-hover-facts.js`
  - `/map/map-hover-tooltip-live.js`
  - `/map/map-layer-panel-live.js`
  - `/map/map.css`
  match `site/.out`
- `bash tools/scripts/map-browser-smoke.sh`
  - `PASS`

Remaining follow-up from this slice:

- verify the live hover/settings behavior end-to-end in a browser probe once the current DevTools
  transport/socket issue is clear again
- if needed, add more layer-specific facts from the runtime detail sections without widening the
  bridged contract

### 2026-03-31: Removed the obsolete pre-live map page bootstrap stack

Deleted the dead map page implementation that was no longer on the published/live path:

- `site/assets/js/pages/map-page.js`
- `site/assets/js/pages/map-page.test.mjs`
- `site/assets/map/map-page-state.js`
- `site/assets/map/map-page-state.test.mjs`
- `site/assets/map/map-page-signals.js`
- `site/assets/map/map-page-signals.test.mjs`

Why this matters:

- the live map now boots only through:
  - `site/assets/map/map-page-live.js`
  - `site/assets/map/map-app-live.js`
  - `site/assets/map/map-shell.html`
- keeping the old page bootstrap/state/signal layer around made the remediation harder to reason
  about because the repo still looked like it had two supported map page architectures
- deleting the dead stack makes the clean-slate path the only page-side map implementation left in
  the tree

Follow-up note:

- `site/assets/map/loader.js` still exists for now as a non-live historical implementation
- the next meaningful cleanup target after this is the remaining loader-only support surface, not
  resurrecting a second page bootstrap path

### 2026-03-31: Deleted the legacy loader stack from the repo

Removed the old non-live loader path entirely:

- `site/assets/map/loader.js`
- `site/assets/map/loader.test.mjs`
- `site/assets/map/map-bridge-projection.js`

Also updated stale tooling that still assumed `/map/loader.js` existed:

- `tools/scripts/stage_cdn_assets.sh`
- `tools/scripts/check_cdn_map_runtime_assets.sh`
- `site/zine.ziggy`

Why this matters:

- the repo now matches the architectural reality:
  - the live map does not boot through `loader.js`
  - the repo no longer keeps a 10k-line dead implementation around as if it were still relevant
- it removes the biggest remaining source of conceptual drift for future map work
- staging/check scripts no longer pretend the deleted `/map/loader.js` asset is part of the
  supported runtime contract

Current clean-slate runtime contract after this deletion:

- page shell:
  - `site/assets/map/map-shell.html`
- page bootstrap:
  - `site/assets/map/map-page-live.js`
- app orchestration:
  - `site/assets/map/map-app-live.js`
- bridge/runtime adapter:
  - `site/assets/map/map-app.js`
  - `site/assets/map/map-runtime-adapter.js`
  - `site/assets/map/map-host.js`
- live shell controllers:
  - window manager
  - search
  - layers
  - hover tooltip
  - bookmarks
  - zone info

Next tasks from here:

- keep removing stale legacy assumptions from tooling/worklogs as needed
- continue restoring or tightening any remaining map features only on the clean-slate path
- keep the bridge contract explicit and narrow:
  - `_map_bridged`
  - `_map_actions`
  - `_map_session`
  - `_map_runtime`

### 2026-03-31: Replaced the last map-specific restore global with a shell-local init event

The clean-slate map no longer needs `window.__fishystuffMapLiveRestore`.

What changed:

- `site/assets/map/map-shell.html`
  - the hidden Datastar init node now dispatches a shell-local custom event:
    - `fishymap-live-init`
  - the event payload is the live Datastar signal object from `$`
- `site/assets/map/map-page-live.js`
  - binds a shell-local listener for `fishymap-live-init`
  - restores/persists exactly as before, but without exposing a project-specific global restore
    function on `window`
- updated:
  - `site/assets/map/map-page-live.test.mjs`
  - `site/assets/map/map-shell.test.mjs`

Why this matters:

- it is closer to Datastar’s intended model than a page-global imperative bootstrap hook
- the shell now owns its own init handshake through DOM events instead of a custom project global
- it shrinks the remaining global map bootstrap surface to the generic runtime hooks that still
  make sense

Validation:

- `node --test site/assets/map/map-app-live.test.mjs site/assets/map/map-page-live.test.mjs site/assets/map/map-shell.test.mjs site/assets/map/map-hover-facts.test.mjs site/assets/map/map-layer-panel-live.test.mjs site/assets/map/map-bookmark-panel-live.test.mjs site/assets/map/map-runtime-adapter.test.mjs site/assets/map/map-host.test.mjs`
- rebuilt site output
- served `/map/` now contains:
  - `/map/map-page-live.js`
  - `fishymap-live-init`
- served `/map/` no longer contains:
  - `__fishystuffMapLiveRestore`

### 2026-03-31: Removed the map-side shared fish global dependency

The clean-slate map no longer reaches through `window.__fishystuffSharedFishState`.

What changed:

- `site/assets/map/map-page-live.js`
  - now restores `_shared_fish` directly from local/session storage using its own normalization
    logic
  - corrupted shared-fish storage is cleared directly during restore
- `site/assets/map/map-host.js`
  - now normalizes shared fish id inputs locally instead of consulting the site-global helper
- `site/assets/map/map-page-live.test.mjs`
  - added regression coverage proving shared-fish restore works without the site-global helper

Why this matters:

- the map clean-slate path is now more self-contained
- it removes another unnecessary global dependency from the live map runtime path
- it keeps shared fish state as plain data crossing into the map, not as a global service that map
  modules need to discover at runtime

Validation:

- `node --test site/assets/map/map-page-live.test.mjs site/assets/map/map-host.test.mjs site/assets/map/map-app-live.test.mjs site/assets/map/map-runtime-adapter.test.mjs`
- rebuilt site output
- served `/map/map-page-live.js` contains:
  - `fishymap-live-init`
- served `/map/map-page-live.js` no longer contains:
  - `__fishystuffSharedFishState`
- `node --check site/assets/map/map-app-live.js`
- `node --test site/assets/map/map-page-live.test.mjs site/assets/map/map-app-live.test.mjs site/assets/map/map-shell.test.mjs`
- rebuilt site output
- served checks confirmed:
  - `/map/map-page-live.js`
  - `/map/map-app-live.js`
  - `/map/`
  all match `site/.out`
- live DevTools checks confirmed:
  - `window.__fishystuffMap === undefined`
  - `window.__fishystuffMapLiveRestore` exists
  - `document.getElementById('map-page-shell').__fishystuffMapPage` exists
  - the map still reaches:
    - `ready === true`
    - `catalog.layers.length === 7`

Next tasks from here:

- keep shrinking the remaining bootstrap surface until the live shell depends only on:
  - Datastar local signals
  - shell-scoped live controllers
  - the explicit bridge contract
- restore any still-unverified live bookmark affordances, especially import, under the clean-slate
  controller path

### 2026-03-31: Dropped legacy loader-era map storage fallback from the live path

The live clean-slate map no longer reads the old loader-era `inputUi` / `inputFilters` shape when
serializing its persisted UI snapshot.

What changed:

- `site/assets/map/map-page-live.js`
  - removed the dead `legacyInputUi` / `legacyInputFilters` merge path from
    `uiStorageSnapshot(...)`
  - removed the old `fishystuff.map.prefs.v1` cleanup call from the live restore path

Why this matters:

- the clean-slate live map should not keep carrying forward the old transitional storage schema
- this removes loader-shaped compatibility logic from the main live persistence path
- it keeps the current storage source of truth explicit:
  - `_map_ui`
  - `_map_bridged`
  - `_map_bookmarks`
  - `_map_session`

Validation:

- `node --check site/assets/map/map-page-live.js`
- `node --test site/assets/map/map-page-live.test.mjs site/assets/map/map-app-live.test.mjs`
- served `/map/map-page-live.js` still matches `site/.out`

### 2026-03-31: Localized live map patch orchestration to the shell subtree

The live map app no longer reacts to the document-wide `datastar-signal-patch` bus directly.

What changed:

- `site/assets/map/map-page-live.js`
  - mirrors Datastar patch events onto the shell subtree as:
    - `fishymap:datastar-signal-patch`
- `site/assets/map/map-app-live.js`
  - now listens to that shell-local patch event instead of a document listener

Why this matters:

- it narrows the clean-slate reactive surface to the map shell itself
- it keeps the live map controllers and bridge orchestration local to the subtree they own
- it is a better fit for the Datastar design goal here:
  - page signals are still global in the Datastar graph
  - map runtime orchestration no longer depends on a document-level patch bus

Validation:

- `node --check site/assets/map/map-page-live.js`
- `node --check site/assets/map/map-app-live.js`
- `node --test site/assets/map/map-page-live.test.mjs site/assets/map/map-app-live.test.mjs`
- served:
  - `/map/map-page-live.js`
  - `/map/map-app-live.js`
  still match `site/.out`
- `bash tools/scripts/map-browser-smoke.sh`
  - `PASS`

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

## Twenty-eighth implementation slice landed

The site no longer publishes the legacy `loader.js` asset. The live map is now shipped only
through the clean-slate shell/app path.

What changed:

- `site/zine.ziggy`
  - removed `map/loader.js` from the published map asset set

Why this slice matters:

- it turns the clean-slate path into the only runtime-served path instead of just the preferred one
- it prevents accidental reintroduction of the legacy loader in live pages or manual debugging
- it makes the remediation concrete:
  - the live map runs on `map-shell.html` + `map-app-live.js`
  - `loader.js` is now legacy-only source, not a served runtime entrypoint

Validation for this slice:

- rebuild site output
- served `/map/loader.js` now returns `404`
- live Chromium reload still reached:
  - `window.FishyMapBridge.getCurrentState().ready === true`
  - layer catalog length `7`

## Twenty-ninth implementation slice landed

The live map page bootstrap is now a single clean-slate script. The live page no longer loads the
old `map-page-state.js`, `map-page-signals.js`, or `js/pages/map-page.js` helper stack.

What changed:

- `site/assets/map/map-page-live.js`
  - new self-contained page bootstrap script for:
    - restore
    - persist
    - shell signal patch application
    - query-aware restore filtering
    - shared-fish restore
- `site/assets/map/map-page-live.test.mjs`
  - focused restore / shell patch / persist coverage for the new live bootstrap
- `site/layouts/map.shtml`
  - now loads only:
    - `map/map-page-live.js`
    - `js/datastar.js`
    - `map/map-app-live.js`
- `site/zine.ziggy`
  - publishes `map/map-page-live.js`

Why this slice matters:

- it removes two more live helper globals from the page:
  - `window.__fishystuffMapPageState`
  - `window.__fishystuffMapPageSignals`
- it reduces the live bootstrap to one Datastar-facing page script plus one clean-slate map app
- it further decouples the live runtime from the legacy loader-era bootstrap files

Validation for this slice:

- `node --test site/assets/map/map-page-live.test.mjs site/assets/map/map-app-live.test.mjs site/assets/map/map-shell.test.mjs site/assets/map/map-runtime-adapter.test.mjs`
- `node --check site/assets/map/map-page-live.js`
- rebuild site output
- served `/map/` now includes:
  - `/map/map-page-live.js`
  - `/js/datastar.js`
  - `/map/map-app-live.js`
  - and no longer includes the three old page bootstrap scripts
- live Chromium reload confirmed:
  - `window.FishyMapBridge.getCurrentState().ready === true`
  - layer catalog length `7`
  - `window.__fishystuffMapPageState === undefined`
  - `window.__fishystuffMapPageSignals === undefined`
- repeatable browser validation recovered:
  - `bash tools/scripts/map-browser-smoke.sh`
    - `PASS`
    - `bridge reached ready with fish catalog`
  - `python3 tools/scripts/map_browser_profile.py load_map --output-json /tmp/map-load.current.json`
    - `PASS`
    - `frame_avg_ms=110.217`
    - `p95_ms=117.700`
  - `python3 tools/scripts/map_browser_profile.py zone_mask_hover_sweep --timeout-seconds 90 --output-json /tmp/map-hover.current.json`
    - `PASS`
    - `frame_avg_ms=7.761`
    - `p95_ms=14.000`

## Thirtieth implementation slice landed

The old live page bootstrap files are no longer published. Just like `loader.js`, they now exist
only as in-repo legacy/migration sources and tests, not as served runtime entrypoints.

What changed:

- `site/zine.ziggy`
  - removed publication of:
    - `js/pages/map-page.js`
    - `map/map-page-state.js`
    - `map/map-page-signals.js`

Why this slice matters:

- the served map runtime now has a single page bootstrap path:
  - `map-page-live.js`
- it prevents accidental live regressions from the old three-script page bootstrap stack
- it makes the runtime boundary clearer:
  - page bootstrap
  - Datastar
  - clean-slate map app

Validation for this slice:

- rebuild site output
- served assets now return `404` for:
  - `/js/pages/map-page.js`
  - `/map/map-page-state.js`
  - `/map/map-page-signals.js`
- live Chromium reload still reached:
  - `window.FishyMapBridge.getCurrentState().ready === true`
  - layer catalog length `7`
  - no old helper globals present on `window`

## Thirty-first implementation slice landed

`map-app-live` is now the single Datastar patch-dispatch point for the live map controllers. The
live controllers no longer need to each subscribe to `datastar-signal-patch` on `document`.

What changed:

- `site/assets/map/map-app-live.js`
  - now imports the controller patch-match helpers
  - routes incoming Datastar patch details to the relevant controller schedules in one place
- `site/assets/map/map-window-manager.js`
- `site/assets/map/map-layer-panel-live.js`
- `site/assets/map/map-bookmark-panel-live.js`
- `site/assets/map/map-search-panel-live.js`
- `site/assets/map/map-zone-info-panel-live.js`
  - each now accepts `listenToSignalPatches`
  - the live app passes `false`, so their document-level Datastar listeners are disabled in the
    real page while remaining available for isolated tests or standalone use

Why this slice matters:

- the clean-slate live path now has one orchestration point for shell patch reactions
- it removes another batch of implicit global listeners from the live runtime
- this keeps controller modules focused on:
  - DOM behavior
  - render scheduling
  - local interactions
  rather than each owning their own page-global Datastar listener

Validation for this slice:

- `node --test site/assets/map/map-app-live.test.mjs site/assets/map/map-page-live.test.mjs site/assets/map/map-layer-panel-live.test.mjs site/assets/map/map-window-manager.test.mjs site/assets/map/map-runtime-adapter.test.mjs site/assets/map/map-shell.test.mjs`
- `node --check site/assets/map/map-app-live.js site/assets/map/map-window-manager.js site/assets/map/map-layer-panel-live.js site/assets/map/map-bookmark-panel-live.js site/assets/map/map-search-panel-live.js site/assets/map/map-zone-info-panel-live.js`
- rebuild site output
- live Chromium reload confirmed:
  - `window.FishyMapBridge.getCurrentState().ready === true`
  - layer catalog length `7`
  - canvas click still updates Zone Info immediately:
    - `selection.pointKind === "clicked"`
    - zone info status becomes `Clicked point`

## Thirty-second implementation slice landed

Live zone-name search is restored on the clean-slate map app, and shell-driven bridge writes now
refresh runtime state after the deferred Wasm apply.

What changed:

- `site/assets/map/map-zone-catalog.js`
  - new clean-slate zone catalog loader + normalizer from `/api/v1/zones`
- `site/assets/map/map-search-state.js`
  - search now uses zone-name matching from the loaded zone catalog instead of dropping
    zone-name-only queries like `Depth 4`
  - zone matches are ranked ahead of semantic fallback matches
- `site/assets/map/map-search-panel-live.js`
  - now accepts late zone-catalog injection and rerenders once the catalog arrives
- `site/assets/map/map-app-live.js`
  - loads the zone catalog asynchronously on startup and hands it to the search controller
  - after shell-driven bridge input patches, now flushes the pending state patch immediately and
    schedules a next-frame bridge snapshot refresh
- `site/zine.ziggy`
  - publishes `map/map-zone-catalog.js`

Why this slice matters:

- clean-slate map search can now resolve actual zone names again without falling back through the
  legacy loader behavior
- the live shell no longer waits for an unrelated later interaction before runtime state catches up
  to a shell-driven bridge patch
- this restored the practical layer-toggle path on the clean-slate live app:
  signal state, bridge input state, and runtime state converge again after a direct shell action

Validation for this slice:

- `node --test site/assets/map/map-app-live.test.mjs site/assets/map/map-runtime-adapter.test.mjs site/assets/map/map-search-state.test.mjs site/assets/map/map-zone-catalog.test.mjs`
- `node --check site/assets/map/map-app-live.js site/assets/map/map-zone-catalog.js site/assets/map/map-search-panel-live.js site/assets/map/map-search-state.js`
- rebuild site output
- live Chromium checks:
  - focusing the search box and typing `Depth 4` shows `17 matches`
  - clicking `Zenato Sea - Depth 4` updates `_map_bridged.filters.semanticFieldIdsByLayer.zone_mask`
  - toggling `Node Waypoints` now updates:
    - `_map_bridged.filters.layerIdsVisible`
    - `FishyMapBridge.getCurrentInputState().filters.layerIdsVisible`
    - `FishyMapBridge.getCurrentState().filters.layerIdsVisible`
    - the runtime layer `visible` flag
  - toggling `Fish Evidence` hidden/visible now keeps signal state, bridge input, and runtime state
    in sync

## Thirty-third implementation slice landed

The bridge now refreshes its own cached snapshot when callers read state during incomplete
bootstrap, so fresh browsers no longer get stranded on stale pending snapshots.

What changed:

- `site/assets/map/map-host.js`
  - added `shouldRefreshStateOnRead(...)`
  - `getCurrentState()` now forces a full Wasm state read while bootstrap is still incomplete
    (`ready !== true`)
- `site/assets/map/map-host.test.mjs`
  - added regression coverage for reading current state while the cached snapshot is still in the
    incomplete bootstrap phase

Why this slice matters:

- the clean-slate live app was already working in an interactive page, but fresh headless browsers
  could still time out reading a stale pending `currentState`
- this was also the root cause of the earlier initial layer-state mismatches:
  shell state and bridge input could be current while `getCurrentState()` still served an older
  bootstrap snapshot
- fixing the freshness at the host boundary is cleaner than teaching each caller or smoke harness
  to force-refresh manually

Validation for this slice:

- `node --test site/assets/map/map-host.test.mjs site/assets/map/map-app-live.test.mjs site/assets/map/map-runtime-adapter.test.mjs site/assets/map/map-search-state.test.mjs site/assets/map/map-zone-catalog.test.mjs`
- `node --check site/assets/map/map-host.js`
- rebuild site output
- restore tracked font artifacts after rebuild
- headless validation:
  - `bash tools/scripts/map-browser-smoke.sh`
    - `PASS`
    - `bridge reached ready with fish catalog`
  - `python3 tools/scripts/map_browser_profile.py zone_mask_hover_sweep --timeout-seconds 90 --output-json /tmp/map-hover.current.json`
    - `PASS`
    - `frame_avg_ms=7.644`
    - `p95_ms=11.400`
- live Chromium checks:
  - fresh `/map/` reload reaches:
    - `_map_runtime.ready === true`
    - `_map_runtime.catalog.layers.length === 7`
    - `_map_runtime.catalog.fish.length === 496`
  - `Node Waypoints` toggles cleanly from hidden -> visible -> hidden with signal state, bridge
    input state, and runtime state staying aligned
  - `Reset UI` still returns the page to `Ready` with `7` layers

Next tasks from here:

- continue deleting transitional duplication around page bootstrap state now that the live map is
  functionally restored again
- keep replacing remaining imperative helper calls in the raw map shell with direct Datastar
  expressions or narrowly scoped clean-slate modules

## Thirty-fourth implementation slice landed

Bookmark placement is restored on the clean-slate live map path without reintroducing hover as a
shared Datastar signal.

What changed:

- `site/assets/map/map-bookmark-panel-live.js`
  - added `buildBookmarkPlacementSelectionResult(...)` as the clean-slate placement decision helper
  - bookmark placement still ignores passive unchanged selection signal patches
  - bookmark placement now also consumes explicit `fishymap:selection-changed` runtime events
  - explicit clicked-point selection events are allowed to place a bookmark even when the clicked
    world point matches the currently selected point
  - placement stays scoped to real clicked map points:
    - `pointKind === "clicked"`
    - no dependency on runtime hover mirrors
- `site/assets/map/map-bookmark-panel-live.test.mjs`
  - added regression coverage for:
    - explicit same-point clicked selection placement
    - ignoring unchanged passive signal patches
    - rejecting non-clicked focus selections for placement mode

Why this slice matters:

- the clean-slate bookmark panel had drifted into a weaker model than `main`:
  - it only reacted to a changed selection key
  - so clicking the already-selected map point while in placement mode did nothing
- the fix stays aligned with the intended Datastar contract:
  - hover remains out of shared Datastar runtime state
  - the bookmark panel responds only to the coarse runtime event it actually cares about:
    `fishymap:selection-changed`
- this restores the practical user flow:
  - click map to select a point
  - click `New bookmark`
  - click the map point again
  - bookmark is created immediately and placement mode exits

Validation for this slice:

- `node --check site/assets/map/map-bookmark-panel-live.js`
- `node --test site/assets/map/map-bookmark-panel-live.test.mjs site/assets/map/map-app-live.test.mjs site/assets/map/map-runtime-adapter.test.mjs site/assets/map/map-zone-catalog.test.mjs site/assets/map/map-host.test.mjs`
- rebuild site output
- restore tracked font artifacts after rebuild
- live Chromium checks on the served build:
  - boot reaches:
    - `_map_runtime.ready === true`
    - `catalog.layers.length === 7`
    - `catalog.fish.length === 496`
  - search still resolves `Depth 4` zone matches
  - zone info tabs still switch correctly after canvas click
  - `Reset UI` still returns to `Ready`
  - bookmark placement now succeeds from the live shell when a `fishymap:selection-changed`
    clicked-point event arrives while placement mode is armed:
    - `_map_bookmarks.entries.length` increments
    - `_map_ui.bookmarks.placing` becomes `false`
    - `_map_ui.bookmarks.selectedIds` selects the new bookmark
- smoke:
  - `bash tools/scripts/map-browser-smoke.sh`
    - `PASS`

Current restored interaction sweep:

- boot:
  - ready pill reaches `Ready`
  - live runtime catalogs load
- toolbar:
  - window visibility buttons reflect open state immediately
- search:
  - zone-name queries work again
  - applying a search result updates bridged filter signals
- layers:
  - visibility/settings toggles stay aligned between shell, bridge input, and runtime
- zone info:
  - canvas clicks populate details and tabs switch correctly
- settings:
  - no indefinite `Loading`
  - `Reset view` / `Reset UI` complete cleanly
- bookmarks:
  - placement mode can create a bookmark again on the live shell

Next tasks from here:

- continue replacing remaining imperative page bootstrap seams with direct Datastar expressions or
  narrowly scoped clean-slate modules
- keep the bridge contract explicit and coarse:
  - `_map_bridged`
  - `_map_actions`
  - `_map_session`
  - `_map_runtime`
- only add new runtime event consumers when a page module genuinely needs a coarse event boundary,
  as with bookmark placement

## Thirty-fifth implementation slice landed

Shell-originated live map patches now re-enter the same reactive path as native Datastar patches.

What changed:

- `site/assets/map/map-page-live.js`
  - shell-dispatched `fishymap-signals-patch` updates now also emit a document-level
    `datastar-signal-patch` event after mutating the live signal graph
  - this keeps controller-originated writes aligned with the live app layer that:
    - persists durable state
    - projects `_map_bridged` into the bridge
    - schedules bridge snapshot refreshes back into `_map_runtime`
- `site/assets/map/map-page-live.test.mjs`
  - added regression coverage proving that a shell-originated patch is re-emitted as a Datastar
    signal patch event

Why this slice matters:

- the clean-slate live shell had a split-brain reactive model:
  - native Datastar signal mutations emitted `datastar-signal-patch`
  - shell/controller mutations emitted only `fishymap-signals-patch`
- that meant controller actions could update the signal graph without necessarily driving the same
  bridge refresh/runtime projection path
- the concrete symptom was stale runtime/catalog state after shell-originated interactions unless a
  manual refresh happened later
- the clearest reproduced case was layer reordering:
  - `_map_bridged.filters.layerIdsOrdered` changed
  - `FishyMapBridge.getCurrentInputState()` changed
  - but `FishyMapBridge.getCurrentState().catalog.layers[*].displayOrder` could remain stale until
    `refreshCurrentStateNow()` was forced

Validation for this slice:

- `node --check site/assets/map/map-page-live.js`
- `node --test site/assets/map/map-page-live.test.mjs site/assets/map/map-app-live.test.mjs site/assets/map/map-bookmark-panel-live.test.mjs site/assets/map/map-runtime-adapter.test.mjs site/assets/map/map-zone-catalog.test.mjs site/assets/map/map-host.test.mjs`
- rebuild site output
- restore tracked font artifacts after rebuild
- served asset verification:
  - `/map/map-page-live.js`
  - `/map/map-bookmark-panel-live.js`
  - both match `site/.out`
- live Chromium checks:
  - shell-originated layer reorder patch now updates:
    - `_map_bridged.filters.layerIdsOrdered`
    - `FishyMapBridge.getCurrentInputState().filters.layerIdsOrdered`
    - `FishyMapBridge.getCurrentState().catalog.layers[*].displayOrder`
    without manual runtime refresh
  - `Reset UI` returns to:
    - `_map_runtime.ready === true`
    - `catalog.layers.length === 7`
  - explicit shell patch bookmark placement still works end-to-end:
    - bookmark count increments
    - placement mode exits
    - new bookmark becomes selected
- headless validation:
  - `python3 tools/scripts/map_browser_profile.py load_map --output-json /tmp/map-load.current.json`
    - `PASS`
  - `python3 tools/scripts/map_browser_profile.py zone_mask_hover_sweep --timeout-seconds 90 --output-json /tmp/map-hover.current.json`
    - `PASS`
  - `bash tools/scripts/map-browser-smoke.sh`
    - clean retry `PASS`
    - the earlier failing run was accompanied by Chromium shared-image allocation errors and did not
      reproduce on a clean rerun

Current restored interaction sweep after the fix:

- boot:
  - ready pill reaches `Ready`
  - runtime catalogs populate
  - layer count reaches `7`
- shell/controller interactions:
  - layer ordering updates runtime draw order immediately
  - bookmark placement updates signals, runtime-facing bookmark input, and selection cleanly
  - `Reset UI` rehydrates runtime/output state cleanly
- profiling/smoke:
  - clean headless load smoke passes
  - hover sweep remains within the expected band

Next tasks from here:

- keep deleting remaining legacy/bootstrap-only duplication now that the live shell patch path is
  unified again
- continue replacing old page-global seams with direct Datastar expressions or narrowly scoped
  clean-slate modules
- keep using the explicit bridge contract as the guardrail:
  - `_map_bridged`
  - `_map_actions`
  - `_map_session`
  - `_map_runtime`

## Thirty-sixth restoration sweep completed

The clean-slate live map is back in a restored state for the currently migrated surface area.

Restoration status at this checkpoint:

- boot:
  - ready pill reaches `Ready`
  - runtime catalog layer count reaches `7`
  - fish catalog reaches `496`
- live shell interactions:
  - toolbar visibility state stays aligned with open/hidden windows
  - layer ordering updates runtime draw order without manual refresh
  - bookmark placement works again through the clean-slate controller path
  - `Reset UI` returns the map to a ready runtime state
  - zone search and zone info work again on the live shell
- validation:
  - `bash tools/scripts/map-browser-smoke.sh`
    - `PASS`
  - `python3 tools/scripts/map_browser_profile.py load_map --output-json /tmp/map-load.current.json`
    - `PASS`
  - `python3 tools/scripts/map_browser_profile.py zone_mask_hover_sweep --timeout-seconds 90 --output-json /tmp/map-hover.current.json`
    - `PASS`
  - `python3 tools/scripts/map_browser_profile.py vector_region_groups_dom_toggle --output-json /tmp/map-vector-region-groups-dom-toggle.current.json`
    - `PASS`

Notes from the validation sweep:

- concurrent headless profile runs can still exhaust Chromium's SwiftShader/shared-image path and
  fail before bridge readiness
- those failures did not reproduce on clean single-scenario reruns and are currently treated as
  harness contention rather than product regressions

Next cleanup priorities after restoration:

- keep migrating functionality away from legacy loader assumptions instead of extracting old logic
  1:1
- reduce the remaining page-global/bootstrap surface further
- continue replacing old imperative seams with:
  - direct Datastar expressions in the shell
  - narrow clean-slate modules
  - the explicit bridge contract only:
    - `_map_bridged`
    - `_map_actions`
    - `_map_session`
    - `_map_runtime`

### 2026-03-31: Slice 9 landed

- Removed the production-side `__fishystuffMapAppAutoStart` test hook from the live map module.
- Added a dedicated side-effectful entry module:
  - `site/assets/map/map-app-live-entry.js`
- `site/assets/map/map-app-live.js` is now a side-effect-free module exporting:
  - `start()`
  - `startWhenDomReady()`
- `site/layouts/map.shtml` now loads the entry module instead of the implementation module directly.

Why this matters:

- the clean-slate live map module no longer needs a global test-only escape hatch
- tests can import the implementation module directly without influencing production boot behavior
- production keeps an explicit entrypoint and implementation split, matching the broader remediation goal of smaller, clearer responsibilities

Validation:

- `node --test site/assets/map/map-app-live.test.mjs site/assets/map/map-page-live.test.mjs site/assets/map/map-host.test.mjs site/assets/map/map-shell.test.mjs`
- `devenv shell -- bash -lc '''cd site && just build-release-no-tailwind'''`

### 2026-03-31: Slice 10 landed

- Kept hover facts page-owned and local to the live shell controllers instead of reintroducing hover into shared Datastar runtime signals.
- The live layer settings preview now prefers current hover samples, with selection as the fallback preview source.
- Fixed the clean-slate layer panel render key so hover fact tables rerender when preview values or per-fact visibility change.
- Added focused coverage in:
  - `site/assets/map/map-hover-facts.test.mjs`
  - `site/assets/map/map-layer-panel.test.mjs`

Why this matters:

- restores the old hover-fact UX direction without broad hover-to-signal churn
- keeps bookmark management independent from hover data
- lets per-layer fact toggles remain cheap page-owned configuration while hover stays transient

Validation:

- `node --test site/assets/map/map-hover-facts.test.mjs site/assets/map/map-layer-panel.test.mjs site/assets/map/map-layer-panel-live.test.mjs site/assets/map/map-app-live.test.mjs site/assets/map/map-page-live.test.mjs site/assets/map/map-host.test.mjs`
- `devenv shell -- bash -lc '''cd site && just build-release-no-tailwind'''`
- served `/map/map-layer-panel.js`, `/map/map-layer-panel-live.js`, and `/map/map-hover-facts.js` matched `site/.out`

### 2026-03-31: Slice 11 landed

- Added direct clean-slate controller coverage for the live hover tooltip in:
  - `site/assets/map/map-hover-tooltip-live.test.mjs`
- The test exercises the real live controller path:
  - synthetic pointer activation
  - `fishymap:hover-changed` event input
  - ordered tooltip row rendering
  - per-layer fact visibility overrides
  - pointerleave hide behavior

Why this matters:

- restores confidence in the live hover path itself, not just the pure helper functions
- anchors the old hover-facts UX on the clean-slate modules
- keeps future remediation work from silently regressing tooltip behavior while replacing the remaining imperative seams

Validation:

- `node --test site/assets/map/map-hover-tooltip-live.test.mjs site/assets/map/map-hover-facts.test.mjs site/assets/map/map-layer-panel.test.mjs site/assets/map/map-layer-panel-live.test.mjs site/assets/map/map-app-live.test.mjs site/assets/map/map-page-live.test.mjs site/assets/map/map-host.test.mjs`
- `devenv shell -- bash -lc '''cd site && just build-release-no-tailwind'''`

### 2026-04-01: Slice 12 in progress

- Added a new clean-slate shared overview fact module:
  - `site/assets/map/map-overview-facts.js`
- Added a new clean-slate generic info state module:
  - `site/assets/map/map-info-state.js`
- Replaced the old live zone-info controller with a generic Info controller:
  - `site/assets/map/map-info-panel-live.js`
- Reworked bookmark overview rows to show semantic facts instead of only world coordinates:
  - folded zone title by default
  - `Resources`
  - `Origin`
- Updated the live map app to load the new bookmark/info modules and zone-catalog dependency.

What changed conceptually:

- bookmarks now consume the same ordered overview-fact derivation used by the Info window
- the Info window is no longer modeled as per-layer tabs
- the clean-slate pane model is now:
  - `Zone`
  - `Territory`
  - `Trade`
- fish presence evidence is carried as a dedicated section in the `Zone` pane instead of being mixed
  into the old layer-tab rendering

Live validation findings:

- served `/map/`, `/map/map-app-live.js`, `/map/map-bookmark-panel-live.js`,
  `/map/map-info-panel-live.js`, `/map/map-info-state.js`, and `/map/map-overview-facts.js`
  matched `site/.out`
- live bookmark rendering now shows:
  - folded zone title
  - `World`
  - `Resources`
  - `Origin`
- live generic Info rendering works on the real runtime-selection path:
  - a synthetic `fishymap:selection-changed` bridge event produced the new `Zone` / `Territory` /
    `Trade` tabs and pane content
  - bookmark inspect now drives runtime selection and the generic Info pane content again

Bug found during live validation:

- `map-info-panel-live.js` was calling `cloneJson(...)` in `setZoneCatalog(...)` without defining it,
  which caused a live promise rejection when the zone catalog finished loading
- fixed locally so the controller no longer throws during startup

Next tasks from here:

- commit the bookmark/info restoration slice
- start the clean-slate search/filter/clipping contract in a new module instead of extending the
  old ad hoc search semantics
- keep the bridge contract explicit:
  - page-only search UX and bookmark manager state stay out of `_map_bridged`
  - only layer-relevant filter/clipping outputs cross into Bevy

### 2026-04-01: Slice 13 landed

- Added a new clean-slate search contract module:
  - `site/assets/map/map-search-contract.js`
- Added:
  - `site/assets/map/map-search-contract.test.mjs`

This slice establishes a canonical owner for live map search selection:

- `_map_ui.search.selectedTerms`

Each selected search term is now normalized as one of:

- `fish-filter`
- `fish`
- `zone`
- `semantic`

What the new contract owns:

- search-term normalization and deduplication
- legacy fallback derivation from older bridged filter state
- projection from selected terms into the explicit Bevy-facing bridge inputs:
  - `_map_bridged.filters.fishIds`
  - `_map_bridged.filters.zoneRgbs`
  - `_map_bridged.filters.semanticFieldIdsByLayer`
  - `_map_bridged.filters.fishFilterTerms`
- an explicit layer search/filter/clipping support matrix

Current layer support matrix:

- `zone_mask`
  - term kinds:
    - `fish`
    - `fish-filter`
    - `zone`
  - clip modes:
    - none
- `fish_evidence`
  - term kinds:
    - `fish`
    - `fish-filter`
    - `zone`
  - clip modes:
    - `zone-membership`
- `regions`
  - term kinds:
    - `semantic`
  - clip modes:
    - `mask-sample`
- `region_groups`
  - term kinds:
    - `semantic`
  - clip modes:
    - `mask-sample`
- `minimap`
  - term kinds:
    - none
  - clip modes:
    - `mask-sample`
- `bookmarks`
  - term kinds:
    - none
  - clip modes:
    - none
- `node_waypoints`
  - term kinds:
    - none
  - clip modes:
    - none

Live/search integration changes:

- `site/assets/map/map-search-state.js`
  - live search state now resolves selected terms from `_map_ui.search.selectedTerms`
  - result selection/removal mutates the selected-term array first
  - bridged filter state is derived by projection instead of acting as the canonical owner
- `site/assets/map/map-query-state.js`
  - URL parsing now seeds `_map_ui.search.selectedTerms`
  - older query shapes are normalized through the same contract
- `site/assets/map/map-page-live.js`
  - page persistence/restore now includes `_map_ui.search.selectedTerms`
- `site/assets/map/map-signal-contract.js`
  - defaults and shell bootstrap now include:
    - `_map_ui.search.selectedTerms`
    - `_map_bridged.filters.fishFilterTerms`

Important transitional note:

- `_map_bridged.filters.{fishIds,zoneRgbs,semanticFieldIdsByLayer,fishFilterTerms}` remain in
  persisted page state for now
- they are no longer the canonical owner
- they are derived projections kept for cold-boot/runtime stability while the rest of the
  clean-slate search/filter path is migrated

Why this matters:

- search-term ownership is now explicit and page-owned instead of spread across ad hoc bridged
  filter branches
- this gives the remaining filter/clipping remediation a stable source of truth
- it also creates a clean place to define which layers support which term kinds and clip modes
  without growing the legacy loader model again

Validation:

- `node --check site/assets/map/map-search-contract.js`
- `node --check site/assets/map/map-search-state.js`
- `node --check site/assets/map/map-query-state.js`
- `node --check site/assets/map/map-page-live.js`
- `node --test site/assets/map/map-search-contract.test.mjs site/assets/map/map-search-state.test.mjs site/assets/map/map-query-state.test.mjs site/assets/map/map-signal-contract.test.mjs site/assets/map/map-page-live.test.mjs`
- rebuilt site output
- restored tracked font artifacts after rebuild
- verified served:
  - `/map/map-search-contract.js`
  - `/map/map-search-state.js`
  match `site/.out`
- live DevTools checks confirmed after hard reload:
  - selecting `Add Missing` writes `_map_ui.search.selectedTerms = [{kind:"fish-filter",term:"missing"}]`
  - the projected bridge state updates `_map_bridged.filters.fishFilterTerms = ["missing"]`
  - removing that chip clears both the selected term and the projected bridged filter state

Next tasks from here:

- commit the canonical search-term slice
- move the remaining live search/filter behavior onto this contract instead of the old direct
  bridged-filter assumptions
- use the explicit support matrix to drive generic filter/clipping affordances per layer


## Slice 14 — Bookmark cards stay semantic on the clean-slate path

The bookmark manager was still carrying a legacy `World` coordinate row even after the title and
overview facts had moved to the clean-slate Datastar model.

What changed:

- `site/assets/map/map-bookmark-state.js`
  - `buildBookmarkOverviewRows(...)` now returns:
    - folded bookmark title
    - semantic overview facts (`Zone` only when it differs from the title, plus `Resources` and
      `Origin`)
  - the always-present `World` row was removed from bookmark cards
- `site/assets/map/map-bookmark-state.test.mjs`
  - added coverage proving bookmark rows now prefer semantic facts over raw coordinates

Why:

- the bookmark card is now aligned with the intended map fact model:
  - title carries the primary zone label by default
  - follow-up rows carry the meaningful summary facts
  - raw world coordinates are no longer given equal prominence in the bookmark summary UI

Validation:

- `node --test site/assets/map/map-bookmark-state.test.mjs`
- rebuilt site output
- live DevTools reload confirmed bookmark cards now show:
  - `Valencia Sea - Depth 5`
  - `Resources`
  - `Origin`
  - with no `World` row

Next:

- restore the richer generic info pane content on the clean-slate path
- wire the new generic layer search/filter/clipping contract into `_map_bridged`


## Slice 15 — Bookmark labels and imported bookmark facts recover on the clean-slate path

The live bookmark flow still had two regressions:

- new bookmarks could inherit a semantic point label like `Margoria (RG218)` instead of using the
  zone name
- imported bookmarks stayed bare page-owned entries and never received the runtime-enriched
  `layerSamples` needed to render `Zone` / `Resources` / `Origin`

What changed:

- `site/assets/map/map-bookmark-state.js`
  - `preferredSelectionLabel(...)` now prefers the overview `Zone` fact before `selection.pointLabel`
  - added `buildRuntimeBookmarkDetailsPatch(...)` to merge runtime-enriched bookmark details back
    into canonical `_map_bookmarks.entries`
- `site/assets/map/map-runtime-adapter.js`
  - added `projectRuntimeBookmarkDetailsToSignals(...)`
- `site/assets/map/map-app.js`
  - clean-slate app now exposes `projectRuntimeBookmarkDetails(...)`
- `site/assets/map/map-app-live.js`
  - live bridge snapshot projection now applies:
    - coarse runtime patch
    - bookmark enrichment patch
    - session patch
  - `resolveBridgeSnapshot(...)` now merges `ui` across partial bridge events
- tests:
  - `site/assets/map/map-bookmark-state.test.mjs`
  - `site/assets/map/map-runtime-adapter.test.mjs`
  - `site/assets/map/map-app.test.mjs`
  - `site/assets/map/map-app-live.test.mjs`

Why:

- bookmark naming must follow the semantic map fact model, not whichever lower-level layer label
  happened to be selected first
- imported bookmarks should become first-class semantic bookmarks after the runtime samples their
  world point; that enrichment belongs on the clean-slate Datastar path, not in legacy loader code

Validation:

- `node --test site/assets/map/map-bookmark-state.test.mjs site/assets/map/map-runtime-adapter.test.mjs site/assets/map/map-app.test.mjs site/assets/map/map-app-live.test.mjs`
- rebuilt site output
- live DevTools reload confirmed:
  - a bare injected bookmark under `_map_bookmarks.entries` came back with runtime `layerSamples`
  - bookmark cards resumed showing semantic facts for imported bookmarks

Next:

- fold the zone-specific fish/group pane back into the generic Info window
- remove redundant search-clipping settings in favor of attachment-driven clipping
- continue the clean-slate search/filter/clipping contract without reintroducing loader-era state


## Slice 16 — Runtime-owned point labels drive bookmark titles again

The bookmark flow still had one conceptual split-brain:

- Bevy selection snapshots did not deterministically choose a canonical `pointLabel`
- bookmark creation worked around that in JS by preferring overview facts over `selection.pointLabel`
- bookmark cards therefore could not cleanly distinguish:
  - the saved bookmark title
  - the current runtime-resolved label for that world point

What changed:

- `map/fishystuff_ui_bevy/src/map/selection_query.rs`
  - added a deterministic point-label resolver that walks sampled layers in runtime order
  - each sampled layer gets one chance to provide a label from:
    - its preferred visible fact
    - zone-name bootstrap fallback for `zone_mask`
    - first target label fallback
  - `selected_info_at_world_point(...)` now uses that resolver before falling back to the caller's
    explicit `point_label`
  - the same resolver now populates `point_label` for:
    - hover-promoted selections
    - `selected_info_for_zone_rgb(...)`
    - `selected_info_for_semantic_field(...)`
- `map/fishystuff_ui_bevy/src/bridge/host/input/commands/selection.rs`
  - passes `bootstrap.zones` into the selection resolver so zone-mask samples can resolve zone
    names even when metadata facts are sparse
- `map/fishystuff_ui_bevy/src/plugins/mask.rs`
  - updated the direct click-selection path to use the same bootstrap-backed point-label resolver

- `site/assets/map/map-overview-facts.js`
  - added `preferredPointLabelForLayerSamples(...)` so page-side bookmark subtitles can mirror the
    runtime label policy from sampled facts without another query
- `site/assets/map/map-bookmark-state.js`
  - bookmark creation now uses `selection.pointLabel` first again
  - added:
    - `bookmarkCurrentPointLabel(...)`
    - `bookmarkCurrentPointSubtitle(...)`
  - bookmark detail rows now suppress the zone row when it matches either:
    - the saved title
    - the current runtime-resolved point label shown as subtitle
- `site/assets/map/map-bookmark-panel.js`
  - bookmark cards now render a subtitle below the title when the saved title differs from the
    current runtime-resolved point label
- `site/assets/map/map.css`
  - added bookmark subtitle styling

Validation:

- Rust:
  - `cargo test --offline -p fishystuff_ui_bevy map::selection_query::tests -- --nocapture`
  - `cargo check -p fishystuff_ui_bevy`
- JS:
  - `node --test site/assets/map/map-overview-facts.test.mjs site/assets/map/map-bookmark-state.test.mjs site/assets/map/map-bookmark-panel.test.mjs site/assets/map/map-bookmark-panel-live.test.mjs`
- live DevTools checks on the served `/map/`:
  - confirmed a live bookmark card can render:
    - title: `Margoria (RG218)`
    - subtitle: `Margoria South`
    - detail rows: `Resources` only, with the duplicate zone row suppressed

Caveat:

- `bash tools/scripts/map-browser-smoke.sh` timed out in headless Chromium before
  `FishyMapBridge.ready` because Chromium failed shared-image allocation (`Creation of
  StagingBuffer's SharedImage failed`). That failure happened at startup before this bookmark slice
  was exercised, and does not match the live DevTools behavior above.

Next:

- restore the generic Info window panes around the runtime-owned label model
- fold the combined zone fish/group view into the Zone pane
- continue the clean-slate search/filter/clipping work without reintroducing bookmark/title
  heuristics on the JS side


## Slice 17 — Runtime bookmark labels and immediate bookmark enrichment

Two bookmark regressions remained on the clean-slate map path:

- imported bookmarks only showed `Resources` / `Origin` after some unrelated later interaction
  such as `Clear selection`
- bookmark subtitles were still derived from frontend heuristics instead of a runtime-owned
  current label, so they could drift from the actual live layer order

What changed:

- `map/fishystuff_ui_bevy/src/bridge/contract/input.rs`
  - added `point_label` to `FishyMapBookmarkEntry`
- `map/fishystuff_ui_bevy/src/bridge/host/snapshot/state/ui.rs`
  - bookmark snapshot enrichment now also computes a runtime-owned bookmark `point_label`
  - that label is derived from the current live layer stack order, not the static registry order
  - the same bookmark snapshot continues to carry sampled `layer_samples`
- `map/fishystuff_ui_bevy/src/bridge/host/snapshot/mod.rs`
  - passes both bootstrap zone names and live `LayerRuntime` into bookmark UI snapshot enrichment
- `site/assets/map/map-host.js`
  - preserves bookmark `pointLabel` when normalizing bridge snapshots
- `site/assets/map/map-bookmark-state.js`
  - bookmark subtitle/current-label logic now prefers runtime `pointLabel`
  - runtime bookmark merges now update:
    - `pointLabel`
    - `layerSamples`
    - `zoneRgb`
- `site/assets/map/map-app-live.js`
  - bookmark writes now schedule one extra deferred bridge snapshot refresh
  - this gives Bevy time to enrich imported bookmarks before the page consumes the runtime snapshot

Why:

- bookmark titles remain the saved user-facing label
- bookmark subtitles should represent the current runtime-resolved point label for that world
  point, and that label must follow the current layer order
- imported bookmarks should become first-class semantic bookmarks immediately after import,
  without waiting for a later unrelated selection or hover event

Validation:

- JS:
  - `node --test site/assets/map/map-bookmark-state.test.mjs site/assets/map/map-app-live.test.mjs site/assets/map/map-runtime-adapter.test.mjs site/assets/map/map-host.test.mjs site/assets/map/map-bookmark-panel-live.test.mjs`
- Rust/build:
  - rebuilt the Wasm runtime via `./tools/scripts/build_map.sh`
- live DevTools checks on served `/map/`:
  - dispatching a bare imported bookmark patch produced bookmark card facts immediately:
    - `Resources`
    - `Origin`
  - no `Clear selection` or extra click was needed
  - changing `_map_bridged.filters.layerIdsOrdered` changed the runtime bookmark `pointLabel`
    for the same bookmark entry, proving the current-label path now follows live layer order

Next:

- restore the generic Info window panes:
  - `Zone`
  - `Territory`
  - `Trade`
- integrate the combined zone fish/group fact pane
- continue the clean-slate search/filter/clipping work with runtime-owned semantic filtering and
  attachment-driven clipping


## Slice 18 — Remove redundant search-clipping UI state

One drift from the intended layer model was a second explicit `Search clipping` settings surface in
the layer panel.

That drift was wrong for two reasons:

- the clipping relationship is already expressed structurally through the layer stack / attachment
  model
- it introduced extra page-owned UI state (`_map_ui.layers.searchClipsByLayer`) for behavior that
  should not be user-toggled per layer from a separate menu

What changed:

- `site/assets/map/map-shell.html`
  - removed `searchClipsByLayer` from the clean-slate shell signal bootstrap
- `site/assets/map/map-signal-contract.js`
  - removed `searchClipsByLayer` from `_map_ui.layers`
  - removed its normalization path from `normalizeMapUiSignalState(...)`
- `site/assets/map/map-page-live.js`
  - stopped persisting/restoring `searchClipsByLayer`
  - stopped treating it as an exact-patch canonical branch
- `site/assets/map/map-layer-panel.js`
  - removed the `Search clipping` settings table from the live layer panel
- `site/assets/map/map-layer-panel-live.js`
  - removed the event wiring that mutated `searchClipsByLayer`
- `site/assets/map/map-runtime-adapter.js`
  - keeps the current internal default clip behavior by projecting
    `DEFAULT_LAYER_SEARCH_CLIPS` directly into `buildLayerSearchEffects(...)`
  - so the clean-slate runtime behavior remains stable while the redundant page UI/state is gone

Validation:

- JS:
  - `node --check site/assets/map/map-layer-panel.js`
  - `node --check site/assets/map/map-layer-panel-live.js`
  - `node --check site/assets/map/map-page-live.js`
  - `node --test site/assets/map/map-signal-contract.test.mjs site/assets/map/map-runtime-adapter.test.mjs site/assets/map/map-page-live.test.mjs`
- served-vs-output checks:
  - rebuilt site output
  - confirmed neither served `/map/` nor `site/.out/map/index.html` contains:
    - `searchClipsByLayer`
    - `Search clipping`
    - `data-layer-search-clip`

Next:

- replace the misleading Zone-pane `Fish Presence` evidence list with a proper page-owned zone
  facts data source
- keep the Info window generic (`Zone`, `Territory`, `Trade`) while moving the zone fish view
  toward the calculator-style droprate + species rows instead of evidence-share percentages


## Slice 19 — Replace Zone-pane evidence shares with page-owned loot summary

The old clean-slate `Zone` pane was still wrong in one important way:

- it reused `zone_stats` ranking evidence distribution and rendered that as if it were the zone’s
  actual fish presence / catch composition

That violated the drift-correction requirement. The `Zone` pane needs a combined catch-profile view
closer to the calculator loot-flow, but limited to:

- left droprate metric
- item/species icon + name

and explicitly not the silver-side columns.

Design choice:

- do **not** widen the Bevy bridge with calculator-derived group/species data
- keep the map bridge focused on map/runtime state
- fetch the zone catch profile page-side, directly from a dedicated API endpoint

What changed:

- `lib/fishystuff_api/src/models/zone_loot_summary.rs`
  - added a dedicated request/response contract for the Zone-pane catch profile
- `api/fishystuff_server/src/app.rs`
  - added `POST /api/v1/zone_loot_summary`
- `api/fishystuff_server/src/routes/meta.rs`
  - documented the endpoint in the lightweight OpenAPI summary
- `api/fishystuff_server/src/routes/calculator.rs`
  - added `post_zone_loot_summary(...)`
  - added `load_zone_loot_summary_data(...)`
  - added `derive_zone_loot_summary_response(...)`
  - derives group/species rows from existing calculator loot helpers, but filters them down to
    the visible group/species rows needed for the map Zone pane
  - preserves original group slot ids so missing groups do not collapse the grouping incorrectly
- `site/assets/map/map-zone-loot-summary.js`
  - added the page-side fetch/normalization helper for the new endpoint
- `site/assets/map/map-info-state.js`
  - removed the misleading ranking-evidence section
  - replaced it with a `Catch Profile` section built from the fetched zone loot summary
- `site/assets/map/map-info-panel-live.js`
  - now loads the zone loot summary asynchronously when selection enters a zone
  - dedupes same-zone in-flight requests so repeated runtime selection patches do not spam the API
- `site/assets/map/map-app-live.js`
  - clean-slate orchestration now triggers zone-loot refresh on runtime selection changes
- `site/assets/map/map.css`
  - added styling for grouped catch-profile rows
- `site/zine.ziggy`
  - publishes the new `map-zone-loot-summary.js` asset
- `site/assets/map/map-info-state.test.mjs`
  - updated from the old evidence expectations to the new `zone-loot` section
- `site/assets/map/map-zone-loot-summary.test.mjs`
  - covers selection rgb derivation, normalization, and request body formatting

Validation:

- JS:
  - `node --check site/assets/map/map-zone-loot-summary.js`
  - `node --check site/assets/map/map-info-state.js`
  - `node --check site/assets/map/map-info-panel-live.js`
  - `node --check site/assets/map/map-app-live.js`
  - `node --test site/assets/map/map-info-state.test.mjs site/assets/map/map-zone-loot-summary.test.mjs`
- Rust:
  - `cargo check -p fishystuff_server`
- site:
  - rebuilt `site/.out`
  - confirmed served `/map/map-zone-loot-summary.js` matches the new module
- live browser:
  - local `just watch` API remained stale because it was a plain long-lived `cargo run`, not an
    autoreload watcher
  - validated end-to-end with a temporary fresh API on `127.0.0.1:8081`
  - opening the `Zone` tab after inspecting a point/bookmark now shows:
    - `Zone` facts
    - `Catch Profile`
    - grouped droprate + species rows
  - no ranking-evidence share percentages remain in that pane
  - when the default `127.0.0.1:8080` API is still on an older build and returns `404` for
    `POST /api/v1/zone_loot_summary`, the page now performs exactly one request per selected zone
    and surfaces a clear endpoint-unavailable message instead of retriggering the same failed load
    on every repeated runtime selection patch

Why this is aligned with the remediation goal:

- the Info window stays generic:
  - `Zone`
  - `Territory`
  - `Trade`
- the catch profile is page-owned Datastar/UI state, not a new broad bridge mirror
- the Bevy bridge remains focused on map/runtime selection and semantic facts

Next:

- restore the bookmark title/fact behavior around imported and newly created bookmarks where still
  inconsistent with the live point label
- continue the search/filter/clipping remediation using the explicit bridge whitelist, but keep
  attachment-driven clipping as the primary model instead of reintroducing redundant per-layer
  clip toggles

## 2026-04-01: idle FPS regression root cause and fix

Observed regression:

- local fresh-map FPS had fallen from the expected ~200+ range down toward the ~30 range reported
  by the user
- this reproduced even after stashing the unrelated in-progress clipping/filter work
- the cleanest measurable symptom was not raster or Bevy draw cost; it was bridge snapshot work

Measured before fix on a fresh isolated `/map/` page:

- `frame_time_ms.avg` was roughly `5.8ms` to `6.1ms`
- `bridge.snapshot_sync` was costing about `2.0ms` to `2.3ms` every frame on an otherwise idle
  page
- the dominant sub-cause was `semantic_catalog` projection rebuilding every frame

Root cause:

- several runtime resources were being held as mutable in always-running systems even when they
  were only read
- bridge snapshot gating relied on raw Bevy `Res::is_changed()` for those resources
- this is especially bad for the Datastar/bridge path because the bridge then reprojects large
  payloads even when the exported snapshot is semantically unchanged
- the most expensive case was `FieldMetadataCache`
  - it is polled every frame
  - `is_changed()` was therefore unsuitable as the trigger for rebuilding semantic search terms

What changed:

- `map/fishystuff_ui_bevy/src/bridge/host/snapshot/mod.rs`
  - snapshot branches now compare the projected payload before rewriting the stored snapshot
  - `filters_changed` no longer depends on `LayerRuntime::is_changed()`
  - `ui_changed` no longer depends on `MapDisplayState::is_changed()`
  - semantic term rebuilding now keys off an explicit metadata revision instead of raw
    `FieldMetadataCache::is_changed()`
- `map/fishystuff_ui_bevy/src/map/field_metadata.rs`
  - added a monotonic `revision` that advances only when ready metadata actually changes
- `map/fishystuff_ui_bevy/src/plugins/api/requests/poll.rs`
  - downgraded `LayerRegistry` and `LayerRuntime` from `ResMut` to `Res` in the request poller
    because that system only reads them

Measured after fix on a fresh isolated `/map/` page:

- idle `frame_time_ms.avg` dropped to about `3.56ms`
- `bridge.snapshot_sync` dropped to about `0.044ms` average per frame
- synthetic hover remained healthy at about `3.61ms` average frame time

Why this matters for the remediation:

- this was not a clipping/filter regression
- it was a direct example of why the bridge must stay explicit and sparse
- Datastar-facing projections should react to semantic state changes, not incidental mutable
  resource access in Bevy systems

Next:

- keep the new snapshot path as the baseline
- continue restoring remaining map behavior on top of this lower-churn bridge path
- when touching bridge snapshot state in the future, prefer explicit revisions or output
  comparison over raw `is_changed()` for resources that are polled every frame

## 2026-04-01: clean-slate window drag jitter regression

Observed regression:

- window dragging felt jittery even when the Bevy canvas was no longer the main FPS bottleneck
- this was especially confusing because it looked like another runtime perf problem while the
  actual regression lived entirely in the page-side clean-slate window manager

Root cause:

- the clean-slate `map-window-manager.js` drag path updates the active window position directly in
  the DOM during `pointermove`
- but `applyFromSignals()` still reapplied signal-owned `x/y` for every window whenever shell or
  bridge-driven controller work scheduled a window sync
- during an active drag, that means stale signal coordinates can briefly overwrite the live drag
  position until `pointerup` finally persists the new coordinates

What changed:

- `site/assets/map/map-window-manager.js`
  - `applyFromSignals()` now skips the actively dragged window
  - the drag path remains authoritative until the final `pointerup` patch commits the new
    coordinates
- `site/assets/map/map-window-manager.test.mjs`
  - added a regression harness that simulates an in-progress drag and verifies that a stale
    signal apply does not snap the active window back to its old coordinates

Why this matters for the remediation:

- this is a concrete example of why the clean-slate page controllers must treat direct local UI
  interaction as authoritative while it is in progress
- Datastar-owned durable state should persist the result of the interaction, but it must not fight
  the interaction itself frame-by-frame

## 2026-04-01: bookmark detail feedback caused delivered FPS drop

Observed regression:

- with active bookmark entries present, delivered FPS on the live `/map/` page dropped from the
  expected ~60 back down into the high-40s even though Bevy CPU frame cost stayed low
- hiding the bookmark layer did not help
- clearing bookmark entries entirely restored delivered FPS immediately

Measured on the served page:

- with one bookmark entry present:
  - delivered FPS was about `48.6`
  - average Bevy CPU frame cost stayed around `4.8ms`
- with bookmark layer hidden but bookmark entries still present:
  - delivered FPS stayed around `46.3`
- with bookmark entries cleared:
  - delivered FPS returned to about `60.0`

Root cause:

- the clean-slate path was merging runtime-enriched bookmark details back into durable
  `_map_bookmarks.entries`
- those patches were re-entering the full Datastar shell/persist/render path even though the
  runtime bookmark detail payload is ephemeral UI state, not canonical bookmark input
- this was the wrong ownership model:
  - bookmark coordinates/titles belong in canonical bookmark state
  - runtime layer samples / point labels / enriched facts belong in runtime state only

What changed:

- `site/assets/map/map-runtime-adapter.js`
  - runtime snapshot projection now exposes `ui.bookmarks` under `_map_runtime.ui.bookmarks`
  - stopped projecting runtime bookmark details back into `_map_bookmarks`
- `site/assets/map/map-app.js`
  - removed the separate `projectRuntimeBookmarkDetails(...)` path
- `site/assets/map/map-app-live.js`
  - stopped combining a `_map_bookmarks` patch into every bridge snapshot projection
- `site/assets/map/map-bookmark-state.js`
  - bookmark panel state now merges canonical bookmarks with `_map_runtime.ui.bookmarks`
    ephemerally at read time
  - bookmark signal invalidation now watches `_map_runtime.ui.bookmarks`

Validation:

- focused JS tests passed for:
  - `map-runtime-adapter`
  - `map-app`
  - `map-bookmark-state`
- live page measurement after the fix:
  - one live bookmark entry present
  - delivered FPS returned to about `59.95`
  - average Bevy CPU frame cost stayed around `4.1ms`

Why this is aligned with the remediation:

- runtime-enriched bookmark facts are now treated as ephemeral runtime output instead of durable
  Datastar page input
- this keeps the bookmark UI functional while removing a large, high-churn feedback path from the
  clean-slate shell

## 2026-04-01: simplify Zone pane catch profile down to rate + identity

Observed drift:

- the clean-slate `Info` window already had the correct pane structure:
  - `Zone`
  - `Territory`
  - `Trade`
- but the `Zone` pane catch profile still exposed calculator-style extra metrics:
  - group share
  - expected catches
  - row-level expected count
- that made the pane read like a mini calculator instead of a compact world-point summary

What changed:

- `lib/fishystuff_api/src/models/zone_loot_summary.rs`
  - removed `countShareText` / `expectedCountText` from group rows
  - removed `expectedCountText` from species rows
- `api/fishystuff_server/src/routes/calculator.rs`
  - zone loot summary note now describes the payload as grouped in-group droprates
  - stopped serializing the removed calculator-only fields
- `site/assets/map/map-zone-loot-summary.js`
  - normalizer now keeps only group identity plus row droprate/identity data
- `site/assets/map/map-info-panel-live.js`
  - group headers now show just the group label
  - row metric now shows only the droprate, alongside the fish/item icon + name

Validation:

- focused JS tests passed for:
  - `map-info-state`
  - `map-zone-loot-summary`
- `cargo test --offline -p fishystuff_server routes::calculator::tests:: -- --skip ignored`
  passed after removing the now-unused `percent_text()` helper

Why this is aligned with the remediation:

- the `Info` window remains a concise fact pane instead of inheriting calculator presentation
- calculator-backed grouping and rates are still reused, but only the minimal world-map-relevant
  subset crosses into the map UI

## 2026-04-01: selected search terms are canonical again

Observed drift:

- the clean-slate shell had already moved search selection ownership to `_map_ui.search.selectedTerms`
- but the live map still depended on imperative search UI actions patching `_map_bridged.filters`
  at the same time
- that meant a direct signal-level `selectedTerms` patch did not actually reach Bevy
  - `_map_ui.search.selectedTerms` changed
  - `_map_bridged.filters` stayed stale
  - bridge input stayed stale too
- this was exactly the kind of FRP drift the remediation is supposed to remove

What changed:

- added `site/assets/map/map-search-projection.js`
  - pure search-term projection from page-owned selected terms into the bridged runtime filter subset
  - canonicalized comparison so the patch is only emitted when the projected bridged filters
    actually differ
- `site/assets/map/map-runtime-adapter.js`
  - bridge input patch generation now derives search filters from the canonical selected-term
    projection instead of trusting `_map_bridged.filters` blindly
- `site/assets/map/map-app-live.js`
  - on any shell patch that changes `_map_ui.search.selectedTerms`, the clean-slate shell now emits
    an internal `_map_bridged.filters` projection patch before reconciling bridge input
  - the same projection is also applied once during initial bootstrap, so query/restore state does
    not depend on the search UI having been used interactively
- `site/assets/map/map-search-contract.js`
  - legacy fallback now also treats top-level `zoneRgbs` as canonical zone terms when restoring
    older storage/query state

Validation:

- focused JS tests passed for:
  - `map-search-contract`
  - `map-search-projection`
  - `map-search-state`
  - `map-runtime-adapter`
  - `map-app-live`
- served assets were checked against `site/.out` for:
  - `/map/map-search-projection.js`
  - `/map/map-app-live.js`
- live DevTools validation:
  - direct shell patch with `{ _map_ui.search.selectedTerms: [{ kind: 'zone', zoneRgb: 3793 }] }`
    now updates:
    - `_map_bridged.filters.zoneRgbs`
    - `FishyMapBridge.getCurrentInputState().filters.zoneRgbs`
    - `FishyMapBridge.getCurrentState().filters.zoneRgbs`
  - direct shell patch with `{ kind: 'semantic', layerId: 'regions', fieldId: 430 }`
    now updates the semantic bridged filter path the same way
  - direct shell patch with `{ kind: 'fish-filter', term: 'missing' }` now resolves through the
    live fish catalog/shared-fish state into a large effective `fishIds` set for the bridge input
- live runtime clipping proof:
  - baseline Fish Evidence query:
    - `represented=26938`
  - `Margoria South` zone term (`zoneRgb=16742655`) with Fish Evidence visible:
    - `represented=433`
  - this confirms the selected-term projection now reaches the runtime and the zone-membership
    filter path is active on the live map

Next:

- continue the still-dirty runtime-side semantic/vector filtering slice and validate it as directly
  as the points zone-membership path
- keep attachment-driven clipping as the primary user model and avoid reintroducing redundant
  explicit clipping settings

## 2026-04-01: runtime zone-membership and semantic filter hooks

What changed:

- `map/fishystuff_ui_bevy/src/plugins/api/state/filters.rs`
  - added `ZoneMembershipLayerFilterState`
- `map/fishystuff_ui_bevy/src/bridge/host/input/state/filters.rs`
  - browser input now applies `filters.zone_membership_layer_ids` into that resource
- `map/fishystuff_ui_bevy/src/plugins/points/query/refresh.rs`
  - Fish Evidence now only consumes the zone filter when `zoneMembershipLayerIds` includes
    `fish_evidence`
- `map/fishystuff_ui_bevy/src/map/vector/build.rs`
  - vector build jobs can now filter features by selected semantic field ids using the source’s
    `feature_id_property`
- `map/fishystuff_ui_bevy/src/plugins/vector_layers.rs`
  - vector cache revisions now include selected semantic field ids
  - visible cache keys and build jobs are keyed off that semantic selection too

Validation:

- `cargo check -p fishystuff_ui_bevy`
- targeted Rust tests passed:
  - `filters_features_by_selected_feature_ids`
  - `zone_membership_layer_filter_state_normalizes_and_clears_layer_ids`
  - `empty_zone_membership_layer_ids_clear_existing_overrides`
- live runtime proof for zone-membership clipping:
  - baseline Fish Evidence:
    - `represented=26938`
  - with `Margoria South` (`zoneRgb=16742655`) selected and Fish Evidence visible:
    - `represented=433`

Notes:

- local `regions` / `region_groups` vector overlays currently report `manifestStatus="missing"` in
  the live dev page, so semantic vector-filtering could not be visually verified there yet
- the semantic path is therefore currently validated by:
  - bridge state inspection
  - vector-build unit coverage
  - revision/build-path wiring

Next:

- investigate why the local dev page has missing vector manifests for `regions` and
  `region_groups`, because that blocks direct live visual validation of the semantic filter path
- after that, validate attachment-driven clipping for raster/vector layers with the same directness
  as the Fish Evidence zone-membership proof

## 2026-04-01: search-driven clipping now follows actual layer attachments

Observed drift:

- the clean-slate runtime adapter still carried a hidden `layerSearchClips` abstraction
  with an implicit default:
  - `fish_evidence -> zone-membership`
- that no longer matched the intended user model
  - users already express clipping by attaching layers in the Layers window
  - there should not be a second hidden or separate "search clip" setting
- the residual abstraction also made the runtime adapter harder to reason about:
  - search filters were page-owned and canonical
  - attachments were page-owned and canonical
  - but clipping behavior still depended on an injected extra default

What changed:

- `site/assets/map/map-search-contract.js`
  - renamed clip capability metadata to `attachmentClipModes`
  - this makes it explicit that clipping support is about attachment semantics, not a separate UI
    preference
- `site/assets/map/map-layer-search-effects.js`
  - removed `DEFAULT_LAYER_SEARCH_CLIPS`
  - removed the unused `layerSearchClips` normalization/toggle helpers
  - search-driven clipping is now derived only from:
    - active search filters
    - the actual `layerClipMasks` attachment graph
    - per-layer attachment clip capability
- `site/assets/map/map-runtime-adapter.js`
  - stopped injecting any default search-clip state
  - now forwards attachment-driven clipping only

New rule:

- active zone-search filters only produce `zoneMembershipLayerIds` for layers that are:
  - attached to `zone_mask`
  - and declare `attachmentClipModes` including `zone-membership`
- raster/vector mask clipping still comes directly from `layerClipMasks`
- no hidden search-clip preference layer remains

Validation:

- focused JS tests passed for:
  - `map-search-contract`
  - `map-layer-search-effects`
  - `map-runtime-adapter`
- served assets were checked against `site/.out` for:
  - `/map/map-search-contract.js`
  - `/map/map-layer-search-effects.js`
  - `/map/map-runtime-adapter.js`
- live DevTools validation after hard reload:
  - attaching `fish_evidence -> zone_mask` with an active zone term produced:
    - `layerClipMasks = { fish_evidence: "zone_mask" }`
    - `zoneMembershipLayerIds = ["fish_evidence"]`
  - clearing that attachment produced:
    - `layerClipMasks = {}`
    - `zoneMembershipLayerIds = []`

Why this is aligned with the remediation:

- attachment remains the only user-facing clipping model
- search-driven clipping is now an effect of canonical state, not a second control path
- this removes another piece of legacy loader-era glue logic from the clean-slate map path

Next:

- continue the generic search/filter work by expanding supported layer-term projections
  without reintroducing hidden secondary state
- keep the attachment graph as the only clipping source of truth on the page side

## 2026-04-01: selected search terms project again on the live clean-slate path

Observed regression:

- the canonical search state already lived under:
  - `_map_ui.search.selectedTerms`
- and the bridge adapter already knew how to derive the runtime-relevant subset from that state:
  - `_map_bridged.filters.{fishIds,zoneRgbs,semanticFieldIdsByLayer,fishFilterTerms}`
- but the live map app was attempting to build that projection against the current pre-patch signal
  snapshot inside `site/assets/map/map-app-live.js`
- when a shell patch carried new selected terms directly, the bridge side saw:
  - no projected `_map_bridged.filters` update
  - no `zoneMembershipLayerIds`
  - no runtime clipping/filter change

What changed:

- `site/assets/map/map-app-live.js`
  - added `buildSearchProjectionPatchForSignalPatch(signals, patch)`
  - it clones the current live signal graph, applies the incoming shell patch to that clone, and
    only then computes the projected `_map_bridged.filters` patch
  - the live shell event handler now uses that post-patch projection instead of the stale
    pre-patch signal snapshot
- `site/assets/map/map-app-live.test.mjs`
  - added coverage proving a `{ _map_ui.search.selectedTerms: [...] }` patch produces the expected
    `_map_bridged.filters` projection even when the current live bridge-filter state is still
    empty

Validation:

- focused JS tests passed:
  - `site/assets/map/map-app-live.test.mjs`
  - `site/assets/map/map-runtime-adapter.test.mjs`
  - `site/assets/map/map-search-projection.test.mjs`
- `node --check site/assets/map/map-app-live.js`
- rebuilt the site and verified the served assets match `site/.out` for:
  - `/map/map-app-live.js`
  - `/map/map-search-projection.js`
- live DevTools validation on the isolated `/map/` page confirmed:
  - shell patch with `{ _map_ui.search.selectedTerms: [{ kind: "zone", zoneRgb: 333333 }] }`
    now updates:
    - `_map_bridged.filters.zoneRgbs = [333333]`
    - `_map_bridged.filters.semanticFieldIdsByLayer.zone_mask = [333333]`
    - `FishyMapBridge.getCurrentInputState().filters.zoneRgbs = [333333]`
  - with `layerClipMasks = { fish_evidence: "zone_mask" }`, the same patch now also yields:
    - `zoneMembershipLayerIds = ["fish_evidence"]`

Why this matters:

- the clean-slate shell contract is usable again for direct signal patches, not just for
  search-panel UI clicks
- attachment-driven clipping is now fed by the canonical selected-term state again
- this restores the next phase of the clipping/filtering remediation without reintroducing
  loader-era bridge assumptions

Next:

- validate whether the default layer graph should include an explicit visible attachment such as
  `fish_evidence -> zone_mask`, instead of relying on manual attachment before clipping becomes
  useful
- continue the generic search/filter work by extending live validation beyond the fish-evidence
  zone-membership path to raster/vector attachment clipping

## 2026-04-01: fish evidence now defaults to the zone mask attachment

Observed usability gap:

- the clean-slate clipping/filtering path was now structurally correct:
  - selected search terms projected into `_map_bridged.filters`
  - attachment-driven clipping was derived only from `layerClipMasks`
- but on a fresh map load, `layerClipMasks` still defaulted to `{}` on the live shell
- that meant the most useful clipping path remained opt-in through a manual drag/drop attach:
  - `fish_evidence -> zone_mask`
- this undercut the intended default experience for zone-focused filtering:
  - Fish Evidence points should already be bounded to the Zone Mask unless the user explicitly
    detaches them

Important constraint:

- simply changing `DEFAULT_MAP_BRIDGED_SIGNAL_STATE.filters.layerClipMasks` would have been wrong on
  its own
- the current `normalizeMapBridgedSignalState(...)` path deep-merged nested objects
- so an explicit persisted clear like:
  - `{ layerClipMasks: {} }`
  would have been rehydrated back into the new default attachment
- that would make detach impossible to persist cleanly

What changed:

- `site/assets/map/map-signal-contract.js`
  - `DEFAULT_MAP_BRIDGED_SIGNAL_STATE.filters.layerClipMasks` now defaults to:
    - `{ fish_evidence: "zone_mask" }`
  - `normalizeMapBridgedSignalState(...)` now preserves an explicit raw `layerClipMasks` object
    when that field is present, instead of always inheriting the default nested object
- `site/assets/map/map-shell.html`
  - the live shell `data-signals` now starts with the same default attachment:
    - `fish_evidence -> zone_mask`
- `site/assets/map/map-page-live.test.mjs`
  - updated the clean-slate default live signal fixture accordingly
- `site/assets/map/map-signal-contract.test.mjs`
  - added coverage that:
    - fresh normalization includes the default attachment
    - explicit `{ layerClipMasks: {} }` keeps the detached state instead of restoring the default

Validation:

- focused JS tests passed:
  - `site/assets/map/map-signal-contract.test.mjs`
  - `site/assets/map/map-page-live.test.mjs`
  - `site/assets/map/map-app-live.test.mjs`
  - `site/assets/map/map-runtime-adapter.test.mjs`
- rebuilt the site and verified the served clean-slate assets reflect the new defaults:
  - `/map/map-shell.html`
  - `/map/map-signal-contract.js`
- live DevTools validation on a fresh isolated `/map/` page confirmed:
  - initial shell/runtime state:
    - `_map_bridged.filters.layerClipMasks = { fish_evidence: "zone_mask" }`
    - `FishyMapBridge.getCurrentInputState().filters.layerClipMasks = { fish_evidence: "zone_mask" }`
  - after dispatching an explicit detached patch:
    - `{ _map_bridged: { filters: { layerClipMasks: {} } } }`
    - shell state became `{}` immediately
    - persisted UI storage wrote `layerClipMasks: {}`
    - reloading the page kept the detached `{}` state instead of re-injecting the default

Why this is aligned with the remediation:

- clipping still has exactly one user-facing model:
  - layer attachment
- there is still no second clipping settings surface
- the default graph is now useful out of the box
- explicit detach remains first-class and persistent

Next:

- continue the generic search/filter work by validating and extending live semantic layer filtering
  and mask clipping beyond Fish Evidence
- keep the attachment graph as the only clipping source of truth while making more of the default
  layer graph intentionally useful

## 2026-04-01: field-backed semantic vector layers render again in 2D

Observed blocker:

- after the search/clipping contract fixes, the next live validation target was:
  - attach `regions -> zone_mask`
  - attach `region_groups -> zone_mask`
  - enable those layers and validate semantic clipping visually
- local runtime inputs were initially missing because the CDN field/vector assets had not been
  regenerated yet
- after rebuilding the map assets, the required local inputs were present again under:
  - `data/cdn/public/fields/regions.v1.bin`
  - `data/cdn/public/fields/region_groups.v1.bin`
  - `data/cdn/public/region_groups/regions.v1.geojson`
  - `data/cdn/public/region_groups/v1.geojson`
- but even with those assets restored, the live map still reported:
  - `regions.vectorStatus = "not-requested"`
  - `region_groups.vectorStatus = "not-requested"`
- the cause was in `map/fishystuff_ui_bevy/src/plugins/vector_layers.rs`:
  - 2D vector activation/rendering still treated `layer.field_url().is_some()` as a reason to
    suppress vector rendering
- that was the wrong rule for `regions` / `region_groups`
  - these layers are intentionally field-backed for hover/selection semantics
  - but they still need to render visually in 2D so clipping and semantic filtering can be seen

What changed:

- `map/fishystuff_ui_bevy/src/plugins/vector_layers.rs`
  - removed the 2D `field_url()` gate from:
    - `should_activate_vector_layer(...)`
    - `should_render_vector_layer(...)`
  - field-backed semantic vector layers now follow the normal visible/clip-required activation path
    in 2D just like other vector overlays
  - added a regression test proving a field-backed `regions` layer still activates and renders in
    `ViewMode::Map2D`

Validation:

- Rust validation passed:
  - `cargo test --offline -p fishystuff_ui_bevy plugins::vector_layers::tests::field_backed_vector_layers_still_render_in_2d -- --exact`
  - `cargo check -p fishystuff_ui_bevy`
- rebuilt the map runtime:
  - `./tools/scripts/build_map.sh`
- live DevTools validation on the isolated `/map/` page after rebuild:
  - enabling `regions` / `region_groups` and attaching both to `zone_mask` no longer leaves them
    at `vectorStatus = "not-requested"`
  - both layers now enter the active 2D vector pipeline:
    - `regions.vectorStatus = "building"`
    - `region_groups.vectorStatus = "building"`
    - with non-zero `vectorFeatureCount`

Why this matters:

- semantic clipping/filtering validation was previously blocked by a dead render path
- the clean-slate page can now drive those semantic layers visually again
- this aligns the runtime with existing profiling scenarios such as:
  - `vector_region_groups_enable`
  - `vector_regions_enable`
  which already assume these layers participate in the 2D vector pipeline

Next:

- continue live validation of semantic clipping/filtering now that the layers actually enter the
  2D render path again
- check whether build completion/ready latency for `regions` / `region_groups` needs a separate
  follow-up, or whether it simply reflects the expected chunked vector build budget

## 2026-04-01: semantic vector filtering and attachment clipping moved forward again

Observed blockers:

- the clean-slate Datastar search projection was already correct:
  - `_map_ui.search.selectedTerms`
  - `_map_bridged.filters.semanticFieldIdsByLayer`
  were updating as expected on the live shell
- but `region_groups` semantic filtering still failed in practice because the local layer catalog
  was using the wrong vector feature id property:
  - `feature_id_property = "id"`
  - the actual GeoJSON uses `rg`
- after fixing that, vector builds could still reuse stale cached geometry when attachment-based
  clip masks changed, because the vector cache revision ignored clip-mask state entirely
- while adding direct vector-mask clipping coverage, another bug became clear:
  vector clip masks treated “outside the mask geometry” as unknown instead of `false`, so outside
  triangles would survive clipping

What changed:

- `map/fishystuff_ui_bevy/src/map/layers/catalog.rs`
  - `region_groups` now uses `feature_id_property = Some("rg")`
  - added a regression test proving the catalog emits the correct feature id property
- `map/fishystuff_ui_bevy/src/plugins/vector_layers.rs`
  - vector cache revisions now include a clip-mask state revision
  - visible-cache keys now include the same clip-mask revision, so attachment changes and mask
    readiness changes invalidate stale cached vector geometry
  - added a direct regression test for revision suffixing:
    - `...|clip:<revision>`
  - added a direct regression test proving vector-mask clipping drops outside triangles
- `map/fishystuff_ui_bevy/src/map/raster/cache/filters/clip_mask/sample.rs`
  - vector-mask sampling now returns `Some(false)` when a sampled point lies outside the mask
    geometry, instead of returning `None`
- `map/fishystuff_ui_bevy/src/map/raster/cache/filters/*`
  - widened the internal clip-mask helper visibility so vector-layer code can reuse the same
    mask-sampling and clip-state-revision logic as the raster path

Validation:

- Rust validation passed:
  - `cargo check -p fishystuff_ui_bevy`
  - `cargo test --offline -p fishystuff_ui_bevy map::layers::catalog::tests::region_group_vector_layer_filters_by_region_group_id -- --exact`
  - `cargo test --offline -p fishystuff_ui_bevy plugins::vector_layers::tests::pending_vector_builds_keep_a_small_progress_budget_when_globally_exhausted -- --exact`
  - `cargo test --offline -p fishystuff_ui_bevy plugins::vector_layers::tests::effective_revision_changes_when_clip_mask_revision_changes -- --exact`
  - `cargo test --offline -p fishystuff_ui_bevy plugins::vector_layers::tests::clip_vector_chunk_against_vector_mask_drops_outside_triangles -- --exact`
- rebuilt the runtime:
  - `./tools/scripts/build_map.sh`
- browser validation also passed through the existing clean-slate harnesses:
  - `bash tools/scripts/map-browser-smoke.sh /tmp/map-browser-smoke.current.json`
  - `python3 tools/scripts/map_browser_profile.py vector_region_groups_dom_toggle --output-json /tmp/map-vector-region-groups-dom-toggle.current.json`
- live DevTools checks on a fresh isolated `/map/` page confirmed:
  - semantic terms still project correctly from the clean-slate shell into `_map_bridged`
  - live bridge input includes attachment clip masks such as:
    - `region_groups -> zone_mask`
  - the fresh MCP page still needs a follow-up for vector `fetching/building` completion timing,
    which appears separate from the search-projection and clip-revision bugs fixed here
  - the heavier `vector_region_groups_enable` profiler scenario still hit Chromium shared-image /
    startup-resource issues in this environment before `ready`, so that specific scenario remains
    noisy as a live signal here

Why this matters:

- semantic filters now target the correct region-group ids
- attachment clipping is no longer vulnerable to stale cached vector meshes
- vector clip masks now have the correct boolean semantics:
  - inside = `true`
  - outside = `false`
- this keeps the remediation aligned with the intended model:
  - Datastar owns durable page state and attachment intent
  - the runtime consumes a small bridged contract
  - clipping behavior is determined by layer attachment, not by ad hoc frontend glue

Next:

- continue live validation on the fresh clean-slate page until `regions` / `region_groups`
  reliably reach `ready` after semantic term and attachment changes
- once that readiness path is stable, validate the user-facing behavior for:
  - semantic filtering of `regions` / `region_groups`
  - attachment clipping of those vector layers against `zone_mask`

## 2026-04-01: browser-applied filter and clip changes now invalidate a new frame

Observed live bug:

- the clean-slate shell and JS bridge input were already correct for attachment-driven clipping:
  - `_map_bridged.filters.layerClipMasks`
  - `FishyMapBridge.getCurrentInputState().filters.layerClipMasks`
- but some live filter/clip changes still appeared to do nothing until a camera interaction caused a
  new frame
- direct DevTools inspection showed the pattern clearly:
  - desired bridge input would update immediately
  - `fishymap_get_current_state_json()` / exported runtime state could lag behind
  - attached vector overlays like `region_groups -> zone_mask` only became visibly correct after an
    unrelated redraw source fired

Cause:

- the Bevy web runtime is effectively on-demand here
- `apply_browser_input_state(...)` was mutating runtime resources from browser patches, but it was
  not explicitly requesting a redraw
- if no other redraw source was active:
  - the updated clip/filter state could sit latent
  - the exported snapshot could remain stale
  - vector/raster attachment changes looked broken until pan/zoom or some other redraw trigger

What changed:

- `map/fishystuff_ui_bevy/src/bridge/host/input/state/mod.rs`
  - `apply_browser_input_state(...)` now writes `RequestRedraw` whenever `BrowserBridgeState`
    changed
  - this makes browser-applied input mutations authoritative on the next frame instead of waiting
    for unrelated map activity

Validation:

- Rust validation passed:
  - `cargo check -p fishystuff_ui_bevy`
- rebuilt the runtime:
  - `./tools/scripts/build_map.sh`
- live DevTools validation on a fresh isolated `/map/` page confirmed the intended user-facing
  effect without camera movement:
  - with `region_groups` visible and detached from `zone_mask`:
    - `vectorTriangleCount = 40351`
  - after attaching `region_groups -> zone_mask` through the clean-slate shell:
    - `state.filters.layerClipMasks = { fish_evidence: "zone_mask", region_groups: "zone_mask" }`
    - `vectorTriangleCount = 183`
  - with `regions` visible and detached from `zone_mask`:
    - `vectorTriangleCount = 108595`
  - after attaching `regions -> zone_mask`:
    - `state.filters.layerClipMasks = { fish_evidence: "zone_mask", regions: "zone_mask" }`
    - `vectorTriangleCount = 312`

Why this matters:

- the remaining clipping/filtering work can now be evaluated directly on the clean-slate page
- attachment changes no longer depend on accidental redraws to appear functional
- this keeps the remediation aligned with the Datastar/bridge goal:
  - page signals express intent
  - the bridge applies that intent coarsely
  - the runtime visibly reacts on the next frame without imperative UI nudges

Next:

- continue the search/filtering remediation itself now that attachment-driven clipping is reliable
  live again
- validate the remaining intended cross-layer behaviors, especially fish-term and zone-term driven
  visibility/clipping on the clean-slate page

## 2026-04-01: attachment graph helper coverage for the clean-slate layer panel

Follow-up hardening:

- after the redraw fix, the live clean-slate page now shows the intended clipping behavior for:
  - `region_groups -> zone_mask`
  - `regions -> zone_mask`
- the next risk was not a live runtime bug but a regression in the page-side attachment graph
  helpers that power drag/drop and detached state
- those helpers were still only indirectly covered through broader panel/runtime tests

What changed:

- added `site/assets/map/map-layer-state.test.mjs`
  - verifies live bridged clip-mask overrides are reflected by `resolveLayerEntries(...)`
  - verifies `buildLayerClipMaskPatch(...)` detaches a single layer without disturbing siblings
  - verifies subtree reattachment rewrites descendants to the new top-level mask
  - verifies `flattenLayerClipMasks(...)` collapses nested attachment chains to the root mask

Validation:

- `node --test site/assets/map/map-layer-state.test.mjs`

Why this matters:

- the clean-slate layer panel now has direct unit coverage for the user-facing clipping model:
  drag/drop attachment and detach are just transformations of the canonical `_map_bridged` graph
- this keeps the remediation aligned with the goal of replacing loader-era imperative glue with
  small, functional signal helpers

Next:

- continue validating the remaining intended search/filter combinations on the live clean-slate map
  page, especially fish-term driven zone membership and any remaining gaps between the support
  matrix and actual runtime behavior

## 2026-04-01: live clipping and search/filter combinations are behaving again

Follow-up validation after the redraw fix:

- the previous runtime-side fixes were necessary but not sufficient
- the real question was whether the intended clean-slate user model now works in the live page:
  - zone search + attachment clipping
  - semantic search + attachment clipping
  - fish search + zone-membership-driven clipping

What I validated live in DevTools on the served clean-slate `/map/` page:

- zone term + attached vector overlay:
  - `selectedTerms = [{ kind: "zone", zoneRgb: 50 }]`
  - detached `region_groups`:
    - `vectorFeatureCount = 240`
    - `vectorTriangleCount = 40351`
  - attached `region_groups -> zone_mask`:
    - `vectorFeatureCount = 240`
    - `vectorTriangleCount = 183`

- zone term + attached `regions`:
  - detached `regions`:
    - `vectorFeatureCount = 1252`
    - `vectorTriangleCount = 108595`
  - attached `regions -> zone_mask`:
    - `vectorFeatureCount = 1252`
    - `vectorTriangleCount = 312`

- semantic term + attached vector overlay:
  - `selectedTerms = [{ kind: "semantic", layerId: "region_groups", fieldId: 1 }, { kind: "zone", zoneRgb: 50 }]`
  - detached `region_groups`:
    - `vectorFeatureCount = 1`
    - `vectorTriangleCount = 102`
  - attached `region_groups -> zone_mask`:
    - `vectorFeatureCount = 1`
    - `vectorTriangleCount = 0`
  - this confirms the semantic filter and the clip mask are both affecting the same runtime build
    path

- fish term + zone-membership-driven clipping:
  - `selectedTerms = [{ kind: "fish", fishId: 42 }]`
  - runtime state reported:
    - `filters.fishIds = [42]`
    - `filters.zoneMembershipLayerIds = ["fish_evidence"]`
    - `pointsStatus ... represented=90 rendered_points=90`
  - with `regions` detached:
    - `vectorTriangleCount = 108595`
  - with `regions -> zone_mask` attached:
    - `vectorTriangleCount = 3350`
  - this confirms the fish-term path is again:
    - filtering fish evidence
    - deriving effective zone membership
    - clipping attached overlays against the resulting zone mask

Harness note:

- the existing headless smoke/profile scripts remain noisy in this environment because some runs hit
  Chromium shared-image / SwiftShader startup failures before `ready`
- that is consistent with earlier observations and did not contradict the direct live page checks

Why this matters:

- the intended clean-slate clipping/filtering contract is working again:
  - Datastar signals express selected search terms and layer attachments
  - `_map_bridged` carries only the relevant shared filter/clip intent
  - the runtime applies fish/zone/semantic filtering and attachment clipping without needing camera
    movement or extra imperative glue

Next:

- continue from a restored clipping/filtering baseline rather than a broken one
- use the support matrix and live contract as the source of truth for any remaining UX refinements
  instead of reintroducing loader-era special cases

## 2026-04-01: remove the live map custom signal-patch bus

Follow-up remediation after the clipping/filtering baseline was restored:

- the clean-slate map no longer depends on `loader.js`, but the live bootstrap still had an extra
  event layer wrapped around Datastar:
  - `fishymap-signals-patch`
  - `fishymap:datastar-signal-patch`
- `map-page-live.js` was mutating the live Datastar signal object, then rebroadcasting custom
  shell/document events so `map-app-live.js` could react
- this was cleaner than the old loader, but it was still a second patch bus instead of using the
  real Datastar signal graph directly

What changed:

- `site/assets/map/map-page-live.js`
  - the shell bootstrap API now exposes:
    - `signalObject()`
    - `patchSignals(patch)`
    - `whenRestored()`
  - direct patching now mutates the live shell signal object without routing through
    `fishymap-signals-patch`
  - the page still listens to the real `datastar-signal-patch` event for persistence, but it no
    longer emits shell-local rebroadcast events
- `site/assets/map/map-app-live.js`
  - now waits for `patchSignals()` on the shell bootstrap API
  - applies query, derived search-projection, and bridge snapshot patches through that direct API
  - now reacts to the real `datastar-signal-patch` event instead of
    `fishymap:datastar-signal-patch`
  - panel controllers and the window manager now receive an injected direct patch function in the
    live app path instead of defaulting to shell patch events
- tests updated:
  - `site/assets/map/map-page-live.test.mjs`
  - `site/assets/map/map-app-live.test.mjs`

Why this matters:

- it removes one of the biggest remaining clean-slate drifts on the live map path
- live map behavior now flows more directly through Datastar itself:
  - shell expressions mutate signals
  - direct JS callers patch the same live signal object
  - the real `datastar-signal-patch` event is the reactive seam
- it narrows the remaining custom orchestration to:
  - persistence/restore sequencing
  - bridge projection
  - imperative controllers that still need to be reduced over time

Validation:

- focused JS validation passed:
  - `node --test site/assets/map/map-page-live.test.mjs site/assets/map/map-app-live.test.mjs site/assets/map/map-shell.test.mjs`
  - `node --check site/assets/map/map-page-live.js`
  - `node --check site/assets/map/map-app-live.js`
- rebuilt the site:
  - `devenv shell -- bash -lc 'cd site && just build-release-no-tailwind'`
- served output matched `site/.out`:
  - `/map/map-page-live.js`
  - `/map/map-app-live.js`
- live DevTools check on `/map/` showed:
  - no new console errors
  - the map still booted cleanly
  - windows, layers, bookmarks, and the search shell were still present and reactive

Current remaining remediation gaps after this slice:

- the map still has a shell bootstrap global:
  - `__fishystuffMapPage`
- `map-page-live.js` still owns a fair amount of restore/persist/filtering logic
- the live panel/window controllers are still imperative JS islands
- site-wide, calculator and Fishydex still depend on:
  - `window.__fishystuffDatastarState`
  - page globals like `window.__fishystuffCalculator` / `window.Fishydex`

Next:

- keep reducing map bootstrap/orchestration responsibility in `map-page-live.js`
- then take the next larger remaining Datastar drift:
  - either the shell bootstrap global itself
  - or one of the imperative live map panel controllers

## 2026-04-01: stop central controller fan-out in `map-app-live.js`

Follow-up cleanup after switching the live map to direct signal patching:

- `map-app-live.js` was still acting like a mini reactive runtime in one important way
- after every signal patch, it manually decided which controller to poke:
  - window manager
  - bookmarks
  - hover tooltip
  - info panel
  - layer panel
  - search panel
- but those controllers already supported narrow `datastar-signal-patch` listeners of their own

What changed:

- `site/assets/map/map-app-live.js`
  - no longer imports the per-controller `patchTouches...` helpers
  - no longer instantiates the live controllers with `listenToSignalPatches: false`
  - no longer keeps:
    - `scheduleShellControllers()`
    - `scheduleControllersForPatch(...)`
  - now lets the controllers react through their own narrow Datastar listeners while the app stays
    focused on:
    - bridge patch derivation
    - bridge mount/state refresh
    - derived search projection patches
    - bookmark-detail refresh timing

Why this matters:

- it removes another layer of central orchestration from the live clean-slate map path
- it moves the architecture closer to the intended Datastar shape:
  - local controller logic listens only to the patches it cares about
  - `map-app-live.js` no longer acts as a page-wide render fan-out bus
- it also makes the remaining drift more explicit:
  - the controllers themselves are now the main imperative islands left to simplify over time

Validation:

- JS validation passed:
  - `node --test site/assets/map/map-app-live.test.mjs site/assets/map/map-page-live.test.mjs site/assets/map/map-window-manager.test.mjs site/assets/map/map-bookmark-panel-live.test.mjs site/assets/map/map-layer-panel-live.test.mjs site/assets/map/map-search-panel-live.test.mjs site/assets/map/map-zone-info-panel-live.test.mjs`
  - `node --check site/assets/map/map-app-live.js`
- rebuilt the site:
  - `devenv shell -- bash -lc 'cd site && just build-release-no-tailwind'`
- served output matched `site/.out`:
  - `/map/map-app-live.js`
- live DevTools check on `/map/` confirmed:
  - no new console errors
  - `Ready`
  - `Layers 7`
  - windows/panels still rendered correctly after reload

Next:

- continue reducing `map-page-live.js` until the remaining shell bootstrap surface is smaller and
  less framework-like
- after that, start collapsing the most imperative remaining live panel/controller seams

## 2026-04-01: restore live patch pickers from runtime catalog

The patch-window dropdowns were stuck at `Loading patches…` on the live map even though the
runtime had already loaded patch metadata.

What I found:

- the live runtime snapshot already contained `_map_runtime.catalog.patches`
- the shell showed the static loading placeholder only because nothing was projecting that
  catalog into the `fishy-searchable-dropdown` local catalog
- this was a shell/controller gap, not an API or Bevy data-loading problem

What changed:

- added `site/assets/map/map-patch-picker-live.js`
  - normalizes runtime patch summaries
  - listens only to patch-relevant `datastar-signal-patch` branches
  - projects `_map_runtime.catalog.patches` into each dropdown's
    `data-role="selected-content-catalog"`
  - keeps selected labels in sync with `_map_bridged.filters.fromPatchId` /
    `_map_bridged.filters.toPatchId`
- wired the controller into `site/assets/map/map-app-live.js`
- published the new module in `site/zine.ziggy`

Why this direction:

- it restores the broken UI without reintroducing a loader-style monolith
- it keeps the bridge/runtime contract unchanged
- it uses a narrow controller seam around an existing custom dropdown component instead of adding
  more global orchestration

Validation:

- JS validation passed:
  - `node --test site/assets/map/map-patch-picker-live.test.mjs site/assets/map/map-app-live.test.mjs`
  - `node --check site/assets/map/map-patch-picker-live.js`
- rebuilt the site:
  - `devenv shell -- bash -lc 'cd site && just build-release-no-tailwind'`
- served output matched `site/.out`:
  - `/map/map-app-live.js`
  - `/map/map-patch-picker-live.js`
- live DevTools check on `/map/` confirmed:
  - `_map_runtime.catalog.patches` contained 36 patches
  - both dropdowns now had 36 local catalog templates
  - selected patch labels updated when `_map_bridged.filters.fromPatchId` /
    `_map_bridged.filters.toPatchId` changed

Next:

- restore default patch-range selection cleanly on the live path so the patch window has a
  sensible initial selection instead of blank `Select patch` placeholders
- continue trimming remaining custom bootstrap/orchestration around the live map shell

## 2026-04-01: seed live patch defaults from the runtime catalog

The previous patch-picker fix restored the live patch catalog, but a clean page load still left
the `From` picker blank until a saved state or manual selection existed.

What changed:

- `site/assets/map/map-patch-picker-live.js`
  - now computes a narrow default patch signal patch
  - when the runtime catalog is ready and `fromPatchId` is still unset, it seeds
    `_map_bridged.filters.fromPatchId` with the oldest available patch id
  - it leaves `toPatchId` unset so the end bound remains the open-ended `Now` option
- `site/assets/map/map-app-live.js`
  - passes direct live Datastar patching into the patch-picker controller so that default seeding
    stays on the clean-slate signal path
- `site/assets/map/map-patch-picker-live.test.mjs`
  - covers the seeded-oldest default and the non-overwrite case

Why this matters:

- the clean-slate shell now owns a sensible default patch window without depending on stale
  persisted state
- `From` is explicit and stable in the Datastar signal graph
- `Until` keeps the intended semantic of `Now` instead of being forced to the latest explicit
  patch id in the picker UI

Validation:

- JS validation passed:
  - `node --test site/assets/map/map-patch-picker-live.test.mjs site/assets/map/map-app-live.test.mjs`
  - `node --check site/assets/map/map-patch-picker-live.js`
- rebuilt the site:
  - `devenv shell -- bash -lc 'cd site && just build-release-no-tailwind'`
- served output matched `site/.out`:
  - `/map/map-patch-picker-live.js`
- live DevTools check on a storage-cleared isolated `/map/` page confirmed:
  - `_map_bridged.filters.fromPatchId = "2019-02-08-node-connection"`
  - `_map_bridged.filters.toPatchId = ""`
  - `From` showed `Node Connection`
  - `Until (incl.)` showed `Now`

Next:

- continue trimming remaining custom bootstrap/orchestration around the live map shell
- likely next target: reduce the `__fishystuffMapPage` bootstrap surface and the framework-like
  restore/persist logic in `map-page-live.js`

## 2026-04-01: remove the live map shell bootstrap global

The remaining bootstrap drift after the clean-slate map work was the shell bootstrap global used
to hand `patchSignals`, `signalObject`, and `whenRestored` from `map-page-live.js` to
`map-app-live.js`.

What changed:

- `site/assets/map/map-page-live.js`
  - no longer writes `__fishystuffMapPage` to the shell
  - now answers a narrow shell-local bootstrap request event:
    - `fishymap-live-bootstrap-request`
  - and emits a narrow shell-local ready event:
    - `fishymap-live-ready`
- `site/assets/map/map-app-live.js`
  - no longer polls the shell for a bootstrap property
  - now waits for the `fishymap-live-ready` response after dispatching a
    `fishymap-live-bootstrap-request`
- tests updated:
  - `site/assets/map/map-page-live.test.mjs`
  - `site/assets/map/map-app-live.test.mjs`

Why this matters:

- it removes another framework-like shell-global seam from the live map
- bootstrap is now a narrow event handshake instead of a mutable public property
- the live map keeps using direct Datastar patching after bootstrap, but the bootstrap itself is
  now easier to reason about and less coupled

Validation:

- JS validation passed:
  - `node --test site/assets/map/map-page-live.test.mjs site/assets/map/map-app-live.test.mjs`
  - `node --check site/assets/map/map-page-live.js`
  - `node --check site/assets/map/map-app-live.js`
- rebuilt the site:
  - `devenv shell -- bash -lc 'cd site && just build-release-no-tailwind'`
- served output matched `site/.out`:
  - `/map/map-page-live.js`
  - `/map/map-app-live.js`
- live DevTools reload on `/map/` confirmed:
  - `Ready`
  - no `__fishystuffMapPage`
  - no `__fishystuffMapLiveBootstrap`
  - patch picker still seeded correctly from the live runtime state

Next:

- continue reducing the framework-like restore/persist logic in `map-page-live.js`
- after that, revisit which remaining panel controllers can be pushed closer to signal-owned DOM

## 2026-04-01: extract live map page persistence helpers

The next remaining live-map drift after removing the shell bootstrap global was the amount of
restore/persist logic still concentrated in `site/assets/map/map-page-live.js`.

What changed:

- added a new clean-slate state helper asset:
  - `site/assets/map/map-page-state.js`
- moved the pure page persistence helpers there:
  - durable snapshot building for `_map_ui`, `_map_bridged`, `_map_bookmarks`, and `_map_session`
  - restore patch loading from local/session storage
  - shared fish fallback restore
  - query-owned restore stripping
- `site/assets/map/map-page-live.js`
  - now depends on the extracted helper for restore/persist state shaping
  - keeps only the live bootstrap responsibilities:
    - shell handshake
    - direct Datastar patch application
    - debounced persistence scheduling
- `site/layouts/map.shtml`
  - now loads `map-page-state.js` before `map-page-live.js`
- tests added/updated:
  - `site/assets/map/map-page-state.test.mjs`
  - `site/assets/map/map-page-live.test.mjs`

Why this matters:

- it removes another large chunk of framework-like behavior from `map-page-live.js`
- it gives the live map a smaller bootstrap script and a separate state-focused module
- it keeps moving toward the intended end state:
  - live bootstrap code stays thin
  - state shaping becomes reusable and testable in isolation

Validation:

- JS validation passed:
  - `node --check site/assets/map/map-page-state.js`
  - `node --check site/assets/map/map-page-live.js`
  - `node --test site/assets/map/map-page-state.test.mjs site/assets/map/map-page-live.test.mjs site/assets/map/map-app-live.test.mjs`
- rebuilt the site:
  - `devenv shell -- bash -lc 'cd site && just build-release-no-tailwind'`
- served output matched `site/.out`:
  - `/map/`
  - `/map/map-page-state.js`
  - `/map/map-page-live.js`
- live DevTools reload on `/map/` confirmed:
  - no new console errors
  - `Ready`
  - `Layers 7`
  - patch picker defaults still behaved correctly

Next:

- continue reducing the remaining framework-like behavior in `map-page-live.js`
- likely next target: move more of the persistence/orchestration contract into pure modules so the
  live bootstrap becomes almost entirely event wiring plus direct Datastar patching

## 2026-04-01: move live map page bootstrap onto modules

The helper extraction in the previous slice left one temporary compromise:

- `map-page-state.js` still had to be exposed through a classic-script global so the live page
  bootstrap could keep running as a deferred classic script

This slice removed that compromise.

What changed:

- `site/assets/map/map-page-state.js`
  - converted from a classic-script helper global into a real ESM module
  - now exports:
    - `MAP_UI_STORAGE_KEY`
    - `MAP_BOOKMARKS_STORAGE_KEY`
    - `MAP_SESSION_STORAGE_KEY`
    - `loadRestoreState(...)`
    - `createPersistedState(...)`
- `site/assets/map/map-page-live.js`
  - converted from a side-effect classic script into an ESM module
  - now exports:
    - `createMapPageLive(...)`
    - the live bootstrap event constants
  - no longer reads a helper global
- added a new narrow entry module:
  - `site/assets/map/map-page-live-entry.js`
  - this is now the only page-bootstrap script loaded by the map shell for the page-state path
- `site/layouts/map.shtml`
  - now loads `map-page-live-entry.js` as a module
  - no longer loads `map-page-state.js` or `map-page-live.js` as classic scripts
- tests updated:
  - `site/assets/map/map-page-state.test.mjs`
  - `site/assets/map/map-page-live.test.mjs`

Why this matters:

- it removes another temporary global bridge from the live map
- the page-state bootstrap is now a normal module graph instead of a classic-script stack
- it moves the clean-slate map further toward the intended shape:
  - thin entry modules
  - imported pure helpers
  - less custom runtime glue around Datastar

Validation:

- JS validation passed:
  - `node --check site/assets/map/map-page-state.js`
  - `node --check site/assets/map/map-page-live.js`
  - `node --check site/assets/map/map-page-live-entry.js`
  - `node --test site/assets/map/map-page-state.test.mjs site/assets/map/map-page-live.test.mjs site/assets/map/map-app-live.test.mjs`
- rebuilt the site:
  - `devenv shell -- bash -lc 'cd site && just build-release-no-tailwind'`
- served output matched `site/.out`:
  - `/map/`
  - `/map/map-page-live-entry.js`
  - `/map/map-page-live.js`
  - `/map/map-page-state.js`
- live DevTools reload on `/map/` confirmed:
  - no new console errors
  - `Ready`
  - `Layers 7`
  - live patch picker defaults still worked

Next:

- keep reducing the remaining framework-like responsibility inside `map-page-live.js`
- or shift focus to the live panel controllers, which are now the larger remaining imperative
  islands on the clean-slate path

## 2026-04-01: extract live map page signal helpers and harden module init ordering

After the module-bootstrap slice, `map-page-live.js` still carried its own exact-patch and
persist-filter logic. While extracting that into a pure module, a real init-order regression
showed up:

- `map-page-live` could start after Datastar had already fired the shell `data-init`
- that left `page.whenRestored()` unresolved
- `map-app-live` then hung before the bridge mount and the live page stayed stuck at:
  - `Settings Loading`
  - `Layers 0`

What changed:

- added a new pure signal helper module:
  - `site/assets/map/map-page-signals.js`
  - exports:
    - `applyMapPageSignalsPatch(...)`
    - `patchMatchesMapPagePersistFilter(...)`
    - page-specific exact replacement and persist-filter constants
- `site/assets/map/map-page-live.js`
  - now imports the pure signal helpers instead of owning that logic inline
- added direct tests:
  - `site/assets/map/map-page-signals.test.mjs`
- hardened the live shell init path:
  - `site/assets/map/map-shell.html`
    - the Datastar init payload is now stored on the shell as `__fishymapInitialSignals`
      before the `fishymap-live-init` event fires
  - `site/assets/map/map-page-live.js`
    - now consumes that sticky init payload on startup if the event was missed
  - `site/assets/map/map-shell.test.mjs`
    - updated to cover the sticky init expression
  - `site/assets/map/map-page-live.test.mjs`
    - added a regression test for the missed-event path

Why this matters:

- it removes another chunk of signal semantics from `map-page-live.js`
- it keeps the page bootstrap moving toward thin event wiring plus imported pure helpers
- it fixes the real init-order race introduced by the module conversion, without reintroducing the
  old shell-global bootstrap pattern

Validation:

- JS validation passed:
  - `node --check site/assets/map/map-page-signals.js`
  - `node --test site/assets/map/map-shell.test.mjs site/assets/map/map-page-live.test.mjs site/assets/map/map-page-signals.test.mjs site/assets/map/map-page-state.test.mjs site/assets/map/map-app-live.test.mjs`
- rebuilt the site:
  - `devenv shell -- bash -lc 'cd site && just build-release-no-tailwind'`
- served output matched `site/.out`:
  - `/map/`
  - `/map/map-page-live-entry.js`
  - `/map/map-page-live.js`
  - `/map/map-page-signals.js`
- live DevTools reload on `/map/` confirmed:
  - `Settings Ready`
  - `Layers 7`
  - `FishyMapBridge.getCurrentState()` reported:
    - `ready: true`
    - `layerCount: 7`
    - `patchCount: 36`

Next:

- keep collapsing `map-page-live.js` toward pure bootstrap/event wiring
- then shift attention to the larger imperative islands still left in the live panel controllers

## 2026-04-01: collapse live map page bootstrap into the app entry

The live map still had one leftover bootstrap layer between `map-page-live` and
`map-app-live`:

- `map-page-live-entry.js`
- `fishymap-live-bootstrap-request`
- `fishymap-live-ready`

That shell handshake was no longer buying anything. It just recreated a mini bootstrap bus after
we had already removed the broader page-global bootstrap surface.

What changed:

- `site/assets/map/map-app-live.js`
  - now imports `createMapPageLive(...)` directly
  - creates and starts the page controller itself before waiting on `whenRestored()`
  - no longer exports or depends on `waitForMapPageBootstrap(...)`
- `site/assets/map/map-page-live.js`
  - no longer exports or uses:
    - `FISHYMAP_LIVE_BOOTSTRAP_REQUEST_EVENT`
    - `FISHYMAP_LIVE_READY_EVENT`
  - no longer dispatches a shell-ready API event
  - now simply exposes the controller API directly from the module:
    - `patchSignals(...)`
    - `signalObject()`
    - `whenRestored()`
- removed:
  - `site/assets/map/map-page-live-entry.js`
- updated:
  - `site/layouts/map.shtml`
  - `site/zine.ziggy`
  - map page/app tests

Why this matters:

- it removes another custom event seam around Datastar
- the live map bootstrap is now simpler and easier to reason about:
  - shell markup provides the sticky init payload
  - `map-app-live` starts `map-page-live`
  - `map-page-live` restores persisted signals
  - `map-app-live` continues once restoration is done
- it keeps the clean-slate path focused on modules and direct signal ownership, not internal
  bootstrap choreography

Validation:

- JS validation passed:
  - `node --check site/assets/map/map-page-live.js`
  - `node --check site/assets/map/map-app-live.js`
  - `node --test site/assets/map/map-page-live.test.mjs site/assets/map/map-app-live.test.mjs site/assets/map/map-page-state.test.mjs site/assets/map/map-page-signals.test.mjs`
- rebuilt the site:
  - `devenv shell -- bash -lc 'cd site && just build-release-no-tailwind'`
- served output matched `site/.out`:
  - `/map/`
  - `/map/map-app-live.js`
  - `/map/map-page-live.js`
- live DevTools reload on `/map/` confirmed:
  - no `map-page-live-entry.js` script remains in the page
  - `Settings Ready`
  - `Layers 7`

Next:

- keep reducing the remaining page-controller orchestration in `map-page-live.js`
- then move onto the larger imperative live panel controllers, starting with the smallest
  high-value seam

## 2026-04-01: make the live window manager app-driven

`site/assets/map/map-window-manager.js` is still an imperative module because dragging and pointer
capture are inherently imperative, but it did not need to own its own Datastar subscription.

Before this slice, the window manager still listened to `datastar-signal-patch` directly and
reconciled `_map_ui.windowUi` on its own. That meant one more controller was independently wired
to the global patch event when the live map app was already the natural orchestration point.

What changed:

- `site/assets/map/map-window-manager.js`
  - no longer imports `DATASTAR_SIGNAL_PATCH_EVENT`
  - no longer subscribes to document-level Datastar patch events
  - is now a pure imperative helper with two responsibilities:
    - handle drag/tap interactions
    - apply current `_map_ui.windowUi` state to the DOM
- `site/assets/map/map-app-live.js`
  - now schedules `windowManager.applyFromSignals()` whenever:
    - a direct controller patch writes `_map_ui.windowUi`
    - a real Datastar signal patch touches `_map_ui.windowUi`
  - also guards the early bootstrap path so query-driven patches do not reference the window manager
    before it exists

Why this matters:

- it removes another controller-owned Datastar listener
- the window manager is now closer to the correct role:
  - DOM/pointer behavior only
  - no independent reactive ownership
- the live map app becomes the single place that decides when `_map_ui.windowUi` signal changes
  should update the live window chrome

Validation:

- JS validation passed:
  - `node --check site/assets/map/map-app-live.js`
  - `node --check site/assets/map/map-window-manager.js`
  - `node --test site/assets/map/map-app-live.test.mjs site/assets/map/map-window-manager.test.mjs site/assets/map/map-page-live.test.mjs`
- rebuilt the site:
  - `devenv shell -- bash -lc 'cd site && just build-release-no-tailwind'`
- served output matched `site/.out`:
  - `/map/map-app-live.js`
  - `/map/map-window-manager.js`
- live DevTools reload on `/map/` confirmed:
  - `Settings Ready`
  - `Layers 7`
  - clicking the settings toolbar button immediately hid the window and flipped the toolbar button
    label to `Show settings`

Next:

- keep moving controller-owned Datastar listeners upward only where the controller is truly
  imperative
- likely next target:
  - the smallest remaining live panel controller that still owns a document-level patch listener

## 2026-04-01: make the live patch picker app-driven

The patch picker was the next-smallest live controller still subscribing to
`datastar-signal-patch` directly even though it does not own any imperative behavior beyond DOM
rendering.

What changed:

- `site/assets/map/map-patch-picker-live.js`
  - no longer imports `DATASTAR_SIGNAL_PATCH_EVENT`
  - no longer attaches its own document-level Datastar listener
  - now stays focused on:
    - deriving picker state from signals
    - rendering the two patch dropdowns
    - emitting a default `fromPatchId` patch when the runtime patch catalog first becomes ready
- `site/assets/map/map-app-live.js`
  - now imports `patchTouchesPatchPickerSignals(...)`
  - schedules `patchPicker.render()` whenever:
    - a direct controller patch touches the patch-picker inputs
    - a real Datastar signal patch touches the patch-picker inputs

Why this matters:

- it removes another unnecessary controller-owned Datastar subscription
- the patch picker is now treated as what it really is:
  - a small view/controller under the app orchestrator
- this keeps narrowing the remaining imperative islands to modules that truly need their own local
  event handling

Validation:

- JS validation passed:
  - `node --check site/assets/map/map-app-live.js`
  - `node --check site/assets/map/map-patch-picker-live.js`
  - `node --test site/assets/map/map-app-live.test.mjs site/assets/map/map-patch-picker-live.test.mjs`
- rebuilt the site:
  - `devenv shell -- bash -lc 'cd site && just build-release-no-tailwind'`
- served output matched `site/.out`:
  - `/map/map-patch-picker-live.js`
- live DevTools reload on `/map/` confirmed:
  - `Settings Ready`
  - `Layers 7`
  - patch picker labels still resolve correctly, for example:
    - `From`: `Node Connection`
    - `Until`: `Now`

Next:

- continue through the remaining live panel controllers in order of smallest high-value cleanup
- likely next candidates:
  - `map-zone-info-panel-live.js`
  - `map-info-panel-live.js`

## 2026-04-01: make the live hover tooltip app-driven

The hover tooltip was another controller with a split responsibility:

- it already had the correct imperative input for hover itself:
  - `fishymap:hover-changed`
  - canvas `pointermove` / `pointerleave`
- but it still owned a separate `datastar-signal-patch` subscription just to rerender when
  visibility or catalog state changed

That second subscription was not necessary once the live app already had the global view of signal
patches.

What changed:

- `site/assets/map/map-hover-tooltip-live.js`
  - no longer imports `DATASTAR_SIGNAL_PATCH_EVENT`
  - no longer listens to document-level Datastar patch events
  - now focuses only on:
    - pointer activity
    - shell hover-change events
    - rendering the hover rows
- `site/assets/map/map-app-live.js`
  - now imports `patchTouchesHoverTooltipSignals(...)`
  - schedules `hoverTooltip.render()` whenever:
    - a direct controller patch changes hover-tooltip-relevant signal branches
    - a real Datastar signal patch changes those branches

Why this matters:

- it removes one more controller-owned Datastar listener
- the hover tooltip now has a cleaner boundary:
  - hover events and pointer positioning stay local to the tooltip controller
  - Datastar signal orchestration stays in the app

Validation:

- JS validation passed:
  - `node --check site/assets/map/map-app-live.js`
  - `node --check site/assets/map/map-hover-tooltip-live.js`
  - `node --test site/assets/map/map-app-live.test.mjs site/assets/map/map-hover-facts.test.mjs`
- rebuilt the site:
  - `devenv shell -- bash -lc 'cd site && just build-release-no-tailwind'`
- served output matched `site/.out`:
  - `/map/map-hover-tooltip-live.js`
- live DevTools reload on `/map/` confirmed:
  - `Settings Ready`
  - `Layers 7`

Next:

- keep working through the remaining live panel controllers by size and coupling
- current likely next candidates:
  - `map-info-panel-live.js`
  - `map-search-panel-live.js`

## 2026-04-01: route live layer, search, and bookmark rerenders through `map-app-live`

The next remaining live-map drift was that several controllers still owned their own
document-level `datastar-signal-patch` listeners even though the live app already had the patch
stream:

- `map-layer-panel-live.js`
- `map-search-panel-live.js`
- `map-bookmark-panel-live.js`

That kept duplicate Datastar subscriptions alive across the live map subtree and blurred the
boundary between:

- imperative local controller behavior
- app-level signal orchestration

What changed:

- `site/assets/map/map-app-live.js`
  - now imports:
    - `patchTouchesLayerPanelSignals(...)`
    - `patchTouchesSearchPanelSignals(...)`
    - `patchTouchesBookmarkSignals(...)`
  - added `routeLiveControllerPatch(...)` as the single scheduler for:
    - window manager sync
    - patch picker rerenders
    - hover tooltip rerenders
    - layer panel rerenders
    - search panel rerenders
    - bookmark panel rerenders
  - now instantiates:
    - `createMapLayerPanelController(...)`
    - `createMapSearchPanelController(...)`
    - `createMapBookmarkPanelController(...)`
    with `listenToSignalPatches: false`
- `site/assets/map/map-search-panel-live.js`
  - re-exports `patchTouchesSearchPanelSignals(...)` for the live app scheduler
- `site/assets/map/map-bookmark-panel-live.js`
  - re-exports `patchTouchesBookmarkSignals(...)` for the live app scheduler

Why this matters:

- it removes three more controller-owned Datastar subscriptions from the live map path
- it keeps those controllers focused on:
  - rendering
  - local DOM events
  - local imperative behavior such as drag/drop or focus handling
- it keeps the app as the single place that decides when signal patches should fan out into those
  rerenders

This is still compatible with the remediation goal:

- `map-app-live.js` remains the live shell orchestrator for Datastar patch flow
- controllers keep only the imperative seams they actually need
- the next cleanup target remains the truly larger island:
  - `map-info-panel-live.js`

Validation:

- JS validation passed:
  - `node --check site/assets/map/map-app-live.js`
  - `node --check site/assets/map/map-bookmark-panel-live.js`
  - `node --check site/assets/map/map-search-panel-live.js`
  - `node --test site/assets/map/map-app-live.test.mjs site/assets/map/map-layer-panel-live.test.mjs site/assets/map/map-bookmark-panel-live.test.mjs`
- rebuilt the site:
  - `devenv shell -- bash -lc 'cd site && just build-release-no-tailwind'`
- served output matched `site/.out`:
  - `/map/map-app-live.js`
  - `/map/map-bookmark-panel-live.js`
  - `/map/map-search-panel-live.js`
- live DevTools reload on `/map/` confirmed:
  - `Search` window present
  - `Info` window present
  - `Layers 7`

Next:

- continue reducing the remaining imperative live controller islands
- current highest-value target:
  - `map-info-panel-live.js`

## 2026-04-01: delete dead Datastar listener plumbing from live layer, search, and bookmark controllers

After routing live layer/search/bookmark rerenders through `map-app-live`, those controllers still
carried dead compatibility plumbing:

- `DATASTAR_SIGNAL_PATCH_EVENT` imports
- `documentRef` options
- `listenToSignalPatches` options
- controller-local `handleSignalPatch(...)` listeners

That code no longer ran on the live path and was only preserving the appearance of a more generic
controller contract than the current clean-slate app actually uses.

What changed:

- `site/assets/map/map-layer-panel-live.js`
  - removed the controller-local Datastar listener plumbing entirely
- `site/assets/map/map-search-panel-live.js`
  - removed the controller-local Datastar listener plumbing entirely
- `site/assets/map/map-bookmark-panel-live.js`
  - removed the controller-local Datastar listener plumbing entirely
- `site/assets/map/map-app-live.js`
  - no longer passes `listenToSignalPatches: false` because the controllers no longer expose that
    option

Why this matters:

- the live path now reflects the actual clean-slate architecture directly, not just by convention
- it reduces the surface area of each controller to:
  - DOM rendering
  - local interactions
  - direct patch emission back into the live app
- it makes the remaining imperative island clearer:
  - `map-info-panel-live.js` and related info-pane work

Validation:

- JS validation passed:
  - `node --check site/assets/map/map-app-live.js site/assets/map/map-bookmark-panel-live.js site/assets/map/map-search-panel-live.js site/assets/map/map-layer-panel-live.js`
  - `node --test site/assets/map/map-app-live.test.mjs site/assets/map/map-layer-panel-live.test.mjs site/assets/map/map-bookmark-panel-live.test.mjs`
- rebuilt the site:
  - `devenv shell -- bash -lc 'cd site && just build-release-no-tailwind'`
- served output matched `site/.out`:
  - `/map/map-app-live.js`
  - `/map/map-layer-panel-live.js`
  - `/map/map-bookmark-panel-live.js`
  - `/map/map-search-panel-live.js`
- live DevTools reload on `/map/` confirmed:
  - `Search` window present
  - `Info` window present
  - `Layers 7`

Next:

- continue reducing the remaining framework-like/global seams around the live map shell
- likely next target:
  - `map-page-live.js`
- or, if we stay in panel land:
  - `map-info-panel-live.js`

## 2026-04-01: delete the dead zone-info controller branch

There was still a fully separate zone-info controller branch published in site assets:

- `site/assets/map/map-zone-info-panel-live.js`
- `site/assets/map/map-zone-info-state.js`

But by this point the live shell already used:

- `site/assets/map/map-info-panel-live.js`
- `site/assets/map/map-info-state.js`

The older zone-info branch had become dead weight:

- not imported by the live app
- not referenced by the shell
- still published in `site/zine.ziggy`
- still carrying an alternate controller/state path that no longer matched the real live map

What changed:

- removed:
  - `site/assets/map/map-zone-info-panel-live.js`
  - `site/assets/map/map-zone-info-state.js`
  - `site/assets/map/map-zone-info-state.test.mjs`
- removed their static asset publication entries from:
  - `site/zine.ziggy`

Why this matters:

- it narrows the live map surface to the controller/state path that is actually in use
- it reduces dead code that could otherwise confuse future remediation work
- it keeps the next cleanup focused on the real remaining seam:
  - `map-page-live.js`
  - or the still-dirty live info panel work

Validation:

- confirmed no remaining references:
  - `rg -n "map-zone-info-panel-live|map-zone-info-state|createMapZoneInfoPanelController|buildZoneInfoViewModel|patchTouchesZoneInfoSignals" site data/scratch/worklog`
- rebuilt the site:
  - `devenv shell -- bash -lc 'cd site && just build-release-no-tailwind'`
- live DevTools reload on `/map/` still confirmed:
  - `Search` window present
  - `Info` window present
  - `Layers 7`

Next:

- keep reducing the real remaining clean-slate drift
- highest-value clean target remains:
  - `map-page-live.js`

## 2026-04-01: trim dead public API from `map-page-live`

`map-page-live` had already moved most of its behavior into helper modules, but it still exposed
more public surface than the live app actually used:

- `connect`
- `persist`
- `restore`
- raw `state`

On the real live path, `map-app-live` only needs:

- `start()`
- `whenRestored()`
- `signalObject()`
- `patchSignals()`

What changed:

- `site/assets/map/map-page-live.js`
  - removed the dead public API members:
    - `connect`
    - `persist`
    - `restore`
    - `state`
- `site/assets/map/map-page-live.test.mjs`
  - added explicit coverage for the smaller live bootstrap surface

Why this matters:

- it makes the page bootstrap contract match the actual clean-slate live architecture
- it reduces temptation to reach back into page-internal state from future map code
- it clarifies the next remaining page-level drift:
  - document-level Datastar persistence/orchestration in `map-page-live.js`

Validation:

- JS validation passed:
  - `node --check site/assets/map/map-page-live.js`
  - `node --test site/assets/map/map-page-live.test.mjs site/assets/map/map-app-live.test.mjs`
- rebuilt the site:
  - `devenv shell -- bash -lc 'cd site && just build-release-no-tailwind'`
- served output matched `site/.out`:
  - `/map/map-page-live.js`
- live DevTools reload on `/map/` still confirmed:
  - `Search` window present
  - `Info` window present
  - `Layers 7`

Next:

- continue trimming the remaining framework-like page bootstrap behavior in `map-page-live.js`
- especially:
  - document-level persistence/orchestration

## 2026-04-01: extract `map-page-live` persistence into its own module

The next remaining framework-like seam in `map-page-live.js` was the persistence controller logic:

- debounce timer ownership
- persisted JSON dedupe state
- storage write coordination
- direct use of the page persist filter

That logic did not need to stay inside the live page bootstrap itself.

What changed:

- added:
  - `site/assets/map/map-page-persist.js`
  - `site/assets/map/map-page-persist.test.mjs`
- `site/assets/map/map-page-live.js`
  - now imports `createMapPagePersistController(...)`
  - no longer owns:
    - debounce timer state
    - persisted JSON bookkeeping
    - direct storage write loops
  - now delegates persistence scheduling and dedupe to the extracted controller
- `site/zine.ziggy`
  - now publishes:
    - `map/map-page-persist.js`

Why this matters:

- it keeps `map-page-live.js` closer to the actual live bootstrap role:
  - restore init
  - connect to the live Datastar signal object
  - apply direct signal patches
- it moves one more chunk of page-side orchestration into a dedicated pure-ish helper module with
  direct coverage

Validation:

- JS validation passed:
  - `node --check site/assets/map/map-page-live.js site/assets/map/map-page-persist.js`
  - `node --test site/assets/map/map-page-persist.test.mjs site/assets/map/map-page-live.test.mjs site/assets/map/map-app-live.test.mjs`
- rebuilt the site:
  - `devenv shell -- bash -lc 'cd site && just build-release-no-tailwind'`
- served output matched `site/.out`:
  - `/map/map-page-live.js`
  - `/map/map-page-persist.js`
- live DevTools reload on `/map/` still confirmed:
  - `Search` window present
  - `Info` window present
  - `Layers 7`

Next:

- keep reducing the remaining page bootstrap orchestration in `map-page-live.js`
- especially:
  - document-level `datastar-signal-patch` persistence binding

## 2026-04-01: route page persistence notifications through `map-app-live`

Even after extracting the persistence controller, `map-page-live.js` still bound its own
document-level `datastar-signal-patch` listener just to notify the persistor.

That meant the live map still had two global consumers of the Datastar patch stream:

- `map-app-live.js`
- `map-page-live.js`

What changed:

- `site/assets/map/map-page-live.js`
  - no longer exports or uses `DATASTAR_SIGNAL_PATCH_EVENT`
  - no longer binds a document-level patch listener
  - now exposes a narrow `handleSignalPatch(patch)` method that only forwards into the extracted
    persistence controller
- `site/assets/map/map-app-live.js`
  - now forwards the real document-level patch stream into:
    - `page.handleSignalPatch(patch)`

Why this matters:

- it removes one more global Datastar listener from the live map path
- it keeps the page controller focused on:
  - restore/init
  - direct patch application
  - persistence scheduling only when the app tells it a patch happened
- it keeps `map-app-live.js` as the single owner of the live global patch stream

Validation:

- JS validation passed:
  - `node --check site/assets/map/map-page-live.js site/assets/map/map-app-live.js`
  - `node --test site/assets/map/map-page-live.test.mjs site/assets/map/map-app-live.test.mjs site/assets/map/map-page-persist.test.mjs`
- rebuilt the site:
  - `devenv shell -- bash -lc 'cd site && just build-release-no-tailwind'`
- served output matched `site/.out`:
  - `/map/map-app-live.js`
  - `/map/map-page-live.js`
- live DevTools reload on `/map/` still confirmed:
  - `Search` window present
  - `Info` window present
  - `Layers 7`

Next:

- continue reducing the remaining framework-like behavior in `map-page-live.js`
- after that, the main remaining imperative island is still:
  - `map-info-panel-live.js`
