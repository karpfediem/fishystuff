# Datastar FRP Refactor Worklog

Date: 2026-03-30
Repo: `/home/carp/code/fishystuff`
Focus: Site-wide Datastar FRP refactor

## Goal

Refactor Datastar usage across the site to follow Datastar's functional reactive model instead of relying on imperative DOM/event plumbing.

Primary user-visible bug driving this:

- removing food or buff selections does not reliably clear the effective calculator state
- AFR remains affected after removing a selected food
- refreshing the page can bring removed selections back

Primary page currently in focus:

- calculator

Additional site areas included in this refactor scope:

- Fishydex page state and persistence
- Datastar-driven detail/modal fetch flows
- site custom elements that subscribe to Datastar patch events
- eventual replacement of the site custom map bridge with Datastar-compatible state flow across:
  - `site/assets/map/loader.js`
  - `site/assets/map/map-host.js`
  - the Bevy WASM map backend

## Map Refactor Target

The map page currently has almost no Datastar surface. Its state is split across:

- imperative loader-owned UI state:
  - window open/collapsed/position state
  - search dropdown visibility
  - bookmark placement/selection UI state
- bridge-owned input state:
  - filters
  - layer controls
  - detail pane selection
  - bookmarks
- bridge-owned runtime snapshot state:
  - ready
  - view
  - selection
  - hover
  - diagnostics

The target model is:

1. Page-owned local UI state becomes Datastar local signals.

Examples:

- `_map_ui.windowUi`
- `_map_ui.search`
- `_map_ui.bookmarks`

2. Bridge/runtime state becomes Datastar-published local signals.

Examples:

- `_map_runtime.state`
- `_map_runtime.inputState`

3. The current Bevy JSON patch/snapshot contract stays temporarily as the transport layer.

That means:

- `site/assets/map/map-host.js` may keep its JSON patch/snapshot contract for now
- `site/assets/map/loader.js` should become an adapter between:
  - Datastar signals
  - the existing bridge contract

4. The final direction is to make Datastar the single page-level state graph.

That does not require rewriting the entire Bevy contract first. The first useful seam is:

- move map page UI shell state into Datastar signals
- publish bridge runtime state back into Datastar signals
- then progressively replace imperative DOM ownership with Datastar ownership

## Relevant Datastar Guidance

Read:

- `https://data-star.dev/guide/getting_started`
- `https://data-star.dev/guide/reactive_signals`
- `https://data-star.dev/guide/datastar_expressions`
- `https://data-star.dev/guide/backend_requests`
- `https://data-star.dev/guide/the_tao_of_datastar`
- `https://data-star.dev/reference/attributes`
- `https://data-star.dev/reference/actions`
- `https://data-star.dev/reference/sse_events`
- `https://data-star.dev/examples/`

Key takeaways:

- backend should remain the source of truth
- signals should be sparse and mostly represent user input
- backend requests should follow signal changes, not imperative event wiring
- local UI-only state should remain local
- patching signals and elements from the backend is the normal flow

Additional repo-level refactor rule for this effort:

- when the FRP cleanup reveals repeated UI/dataflow patterns, extract them into reusable components instead of leaving page-specific copies

## Scope Audit

Current Datastar usage found during initial audit:

- Calculator page:
  - `site/content/en-US/calculator.smd`
  - `api/fishystuff_server/src/routes/calculator.rs`
- Fishydex page:
  - `site/content/en-US/dex.smd`
  - `site/assets/js/pages/fishydex.js`
  - `api/fishystuff_server/src/routes/fish.rs`
- Datastar-aware client components:
  - `site/assets/js/components/distribution-chart.js`
  - `site/assets/js/components/loot-sankey.js`
  - `site/assets/js/components/pmf-chart.js`

The immediate bug is in the calculator, but the target architectural model must cover the whole site.

## Current Anti-Patterns

Observed before refactor, especially in the calculator:

- imperative `requestEval()` path tied to DOM `input` events
- persistence partially driven from explicit calls instead of purely signal changes
- canonical domain state mixed with transport-shaped checkbox arrays
- custom multiselects emulate signal changes through hidden checkbox inputs
- checkbox slot transport leaks into persisted/local canonical state

These create state duplication and timing sensitivity.

## Target Model

Two categories of state only:

1. Canonical domain signals

- backend-owned
- persisted
- sent to eval
- compact values only
- examples: `food = ["item:9359"]`, `buff = ["item:721092"]`

2. Local UI signals

- frontend-only
- never persisted
- never sent to eval
- examples: `_distribution_tab`, dropdown open state, search text

## Component Refactor Principle

This refactor is allowed to introduce or reshape reusable components where useful.

Examples of good extraction candidates:

- generic Datastar-backed search/select controls
- canonical array multiselect controls
- shared signal-patch request/persist helpers
- Datastar-aware chart wrappers

Extraction rule:

- only extract when it reduces page-specific imperative glue and improves reuse
- do not extract abstractions that merely hide the same incorrect model

## Request Flow Target

Desired steady-state flow:

1. init request patches canonical signals and rendered HTML
2. canonical signal patch triggers debounced eval request
3. canonical signal patch triggers debounced persistence
4. local UI signal changes do not hit backend

What should not exist in the final model:

- root-level `data-on:input` eval plumbing
- manual `requestEval()` dispatching
- canonical persisted arrays containing empty checkbox slots

## Multiselect Direction

The existing searchable multiselect is the main structural problem.

Preferred end state:

- bind the multiselect directly to a canonical array signal
- component emits normal value changes
- no hidden checkbox transport layer

Acceptable interim state:

- hidden checkbox transport remains local-only
- canonical domain arrays are derived from local slot arrays
- only canonical arrays are persisted and sent to backend

Not acceptable in final state:

- canonical `food` / `buff` / `outfit` stored in slot-expanded form

## Step Sequence

### Step 1

Move calculator eval triggering to Datastar signal-patch flow.

Work:

- remove imperative `requestEval()` path
- remove root `data-on:input` eval plumbing
- add debounced `data-on-signal-patch` eval trigger

Status:

- implemented
- committed in `e53f8bcd` `Drive calculator evals from Datastar signal patches`

### Step 2

Separate canonical domain state from checkbox transport state.

Work:

- stop letting compact canonical arrays drift into malformed slot arrays in the client
- decide whether to:
  - bind multiselects directly to canonical arrays, or
  - introduce local-only slot arrays and derive canonical arrays from them

Status:

- implemented for calculator food and buff multiselects
- broader outfit/pet checkbox transport still remains for now

Resolution:

- the searchable multiselect now uses an external hidden `<select multiple>` transport
- local transport signals stay compact:
  - `_food_slots = ["item:9359"]`
  - `_buff_slots = ["item:721092"]`
- canonical domain signals are derived directly:
  - `food = Array.isArray($_food_slots) ? $_food_slots : []`
  - `buff = Array.isArray($_buff_slots) ? $_buff_slots : []`

Follow-up:

- the original broad underscore-based patch filter was too aggressive
- computed canonical signals do not emit a usable follow-on patch for eval/persist
- eval/persist now exclude only true ephemeral locals:
  - `_loading`
  - `_calc`
  - `_live`
  - `_distribution_tab`
- transport locals such as `_food_slots`, `_buff_slots`, `_outfit_slots`, `_pet*_skill_slots`, and `_resources` are now allowed to drive eval/persist

### Step 3

Restrict persistence to canonical domain state only.

Work:

- ensure local UI state is excluded
- ensure transport arrays are excluded
- ensure restored state round-trips canonically

Status:

- implemented in part, still needs final site-wide closeout

Current calculator result:

- persisted calculator state now round-trips as compact canonical domain arrays
- local transport arrays remain underscore-prefixed and are excluded from storage
- this still needs a final audit across other Datastar pages and future map-related state flows

### Step 4

Refactor calculator checkbox groups to the same compact transport model.

Work:

- stop binding visible outfit and pet skill checkboxes directly to slot-indexed signals
- keep compact local transport arrays:
  - `_outfit_slots`
  - `_pet{n}_skill_slots`
- derive canonical domain signals directly from those compact arrays
- introduce one reusable checkbox-group component instead of repeating page-specific transport glue

Status:

- implemented

Implementation:

- added reusable `fishy-checkbox-group`
- the component binds visible checkbox groups to an external hidden `<select multiple>`
- visible checkbox state now mirrors selected `<option>` values instead of writing transport slots directly

Validation:

- live Chromium validation confirmed:
  - removing one outfit effect updates:
    - `outfit`
    - `_outfit_slots`
    - localStorage persistence

### Step 9

Introduce the first Datastar-owned map page state seam.

Work:

- add a Datastar signal graph to the map page shell
- restore/persist page-owned map UI state through Datastar instead of the hidden textarea path
- publish bridge snapshot/input state back into Datastar signals
- keep the existing Bevy JSON patch/snapshot contract temporarily as the transport layer

Status:

- implemented

Implementation:

- added `window.__fishystuffMap` in:
  - `site/assets/js/pages/map-page.js`
- the map shell now owns local Datastar signals:
  - `_map_ui.windowUi`
  - `_map_ui.search`
  - `_map_ui.bookmarks`
- the loader now publishes bridge state to:
  - `_map_runtime.state`
  - `_map_runtime.inputState`
- removed the hidden textarea window-ui persistence path from `site/layouts/map.shtml`
- the loader now reads/publishes page-owned UI state through Datastar signal patches

Validation:

- `node --check site/assets/js/pages/map-page.js`
- `node --check site/assets/map/loader.js`
- `node --test site/assets/map/loader.test.mjs site/assets/map/map-host.test.mjs`
- `devenv shell -- bash -lc 'cd site && just build-release-no-tailwind'`
- live Chromium validation confirmed:
  - toggling Bookmarks updates `_map_ui.windowUi.bookmarks.open`
  - the same toggle persists through `fishystuff.map.window_ui.v1`
  - typing in Search updates `_map_ui.search.open`
  - bridge input state is mirrored into `_map_runtime.inputState.filters.searchText`
  - reloading restores the compact persisted outfit state
  - removing the only selected `pet1` skill clears:
    - `pet1.skills`
    - `_pet1_skill_slots`
    - localStorage persistence
  - reloading restores the cleared pet skill state correctly

Result:

- calculator checkbox transport no longer depends on slot-expanded arrays for:
  - outfit
  - pet skills
- the remaining transport shape is now consistent across:
  - searchable multiselects
  - plain checkbox groups

### Step 5

Move Fishydex persistence to Datastar signal-patch flow.

Work:

- stop persisting filter UI state from `sync()` render passes
- stop persisting panel collapse state directly from button click handlers
- stop persisting caught/favourite ids directly from toggle handlers
- add one debounced `data-on-signal-patch` persistence hook for Fishydex
- keep persisted state scoped to:
  - filter/sort UI
  - caught ids
  - favourite ids
  - panel collapse state through shared UI settings

Status:

- implemented

Notes:

- Fishydex still renders imperatively, but persistence is now signal-driven
- this is a smaller, safe slice that removes state writes from click/render paths without rewriting the full page

Validation:

- live Chromium validation confirmed:
  - search filter persisted through `fishystuff.fishydex.ui.v1`
  - panel collapse persisted through `fishystuff.ui.settings.v1`
  - caught/favourite toggles persisted through their existing storage keys
  - all of the above restored correctly after reload

### Step 6

Make Fishydex `sync()` consume the actual Datastar signal graph directly.

Work:

- remove the large template-side pseudo-snapshot object in `dex.smd`
- replace it with `data-effect="window.Fishydex.sync($)"`
- keep normalization inside `fishydex.js`, where it already belongs

Status:

- implemented

Why this matters:

- it removes a broad template-side transport layer
- it reduces the risk of template and JS state models drifting apart
- it makes Fishydex closer to the same “signals first” model already used by the calculator

Validation:

- live Chromium validation confirmed the page still:
  - restores persisted filters and panel state
  - renders the catalog correctly
  - keeps caught/favourite state and card state in sync after reload

### Step 7

Extract a reusable Datastar render base for calculator charts.

Work:

- remove duplicated chart lifecycle code from:
  - `distribution-chart.js`
  - `pmf-chart.js`
  - `loot-sankey.js`
- centralize:
  - Datastar signal-patch subscription
  - requestAnimationFrame render scheduling
  - optional child reset observation
  - optional resize observation
  - shared calculator signal-path reading

Status:

- implemented

Implementation:

- added reusable `FishyDatastarRenderElement`
- added shared `readCalculatorSignal(path)`
- converted:
  - `fishy-distribution-chart`
  - `fishy-pmf-chart`
  - `fishy-loot-sankey`
  to extend the shared render base

Why this matters:

- it removes repeated Datastar patch-listener boilerplate
- it makes chart behavior more consistent across patch cycles
- it creates a reusable foundation for future Datastar-bound visual components

Validation:

- JS syntax checks passed for:
  - `datastar-render-element.js`
  - `distribution-chart.js`
  - `pmf-chart.js`
  - `loot-sankey.js`
- rebuilt site output successfully
- live Chromium validation confirmed:
  - `Groups` still renders
  - `Silver` still renders
  - `Silver` survives a zone change while active
  - `Loot Flow` still renders
  - `Target Fish` PMF still renders once a target is selected

### Step 8

Extract shared hidden `<select multiple>` transport for Datastar array inputs.

Work:

- remove duplicated bound-select wiring from:
  - `searchable-multiselect.js`
  - `checkbox-group.js`
- centralize:
  - bound select lookup
  - bound option lookup
  - `select` input/change subscription
  - external `option.selected` observation

Status:

- implemented

Implementation:

- added reusable `bound-select.js`
- `fishy-searchable-multiselect` now uses the shared bound-select transport
- `fishy-checkbox-group` now uses the same shared bound-select transport

Why this matters:

- it keeps the Datastar array-input transport consistent across searchable and non-searchable controls
- it removes another source of transport drift between components
- checkbox groups now react to external `option.selected` changes through the same mechanism as searchable multiselects

Validation:

- JS syntax checks passed for:
  - `bound-select.js`
  - `checkbox-group.js`
  - `searchable-multiselect.js`
- rebuilt site output successfully
- live Chromium validation confirmed:
  - adding Balacs Lunchbox through the searchable multiselect sets:
    - `food = ["item:9359"]`
    - `_food_slots = ["item:9359"]`
    - AFR back to `72%`
  - removing Balacs Lunchbox returns:
    - `food = []`
    - `_food_slots = []`
    - AFR back to `65%`
  - removing the selected buff clears:
    - `buff`
    - `_buff_slots`
    - persisted localStorage state
  - removing an outfit checkbox keeps:
    - `outfit`
    - `_outfit_slots`
    compact and in sync
  - removing a pet skill checkbox keeps:
    - `pet{n}.skills`
    - `_pet{n}_skill_slots`
    compact and in sync
  - reload restores the same compact persisted state

### Step 9

Make Fishydex details selection local and signal-driven.

Work:

- move detail selection from persisted/canonical page state to local UI signal state
- let `sync($)` own modal open/close rendering again
- stop imperative `openDetails()` / `closeDetails()` paths from forcing rerenders directly
- keep async best-spot fetching as an implementation detail keyed off the selected fish signal

Status:

- implemented

Implementation:

- Fishydex detail selection now uses `_selected_fish_id`
- `sync($)` now:
  - renders the modal from the current selected fish signal
  - triggers best-spot loading when a fish is selected
  - restores focus to the last card when the modal closes
- imperative helpers now mutate signal state only:
  - `openDetails()` patches `_selected_fish_id`
  - `closeDetails()` patches `_selected_fish_id = 0`
- the detail modal no longer depends on direct `renderDetails()` calls from open/close helpers

Why this matters:

- it pushes Fishydex closer to the same “signals drive UI” model as the calculator
- it removes another class of manual rerender coupling
- it keeps modal selection local and explicitly non-persisted

### Step 10

Extract shared caught/favourite fish-state storage helpers.

Work:

- remove duplicated caught/favourite id normalization and storage parsing logic from:
  - `site/assets/js/pages/fishydex.js`
  - `site/assets/map/loader.js`
  - `site/assets/map/map-host.js`
- define one shared browser helper around the existing storage keys and normalized fish-id arrays
- use that helper as the current cross-page seam between Fishydex progress state and map-side shared fish filtering

Status:

- implemented

Implementation:

- added `site/assets/js/shared-fish-state.js`
- base layout now loads the helper globally as `window.__fishystuffSharedFishState`
- Fishydex uses it for:
  - fish id normalization
  - load/corruption reset handling
  - persistence of caught/favourite ids
- map frontend/runtime bridge code uses it for:
  - shared caught/favourite state loading in `loader.js`
  - shared filter state loading in `map-host.js`

Why this matters:

- it removes another cross-page duplication point
- it makes “shared fish progress state” explicit as a reusable frontend contract
- it gives the future Datastar map-bridge replacement a cleaner seam to integrate with instead of reaching into duplicated localStorage code

### Step 11

Move map bookmark list state into its own Datastar signal branch.

Work:

- stop treating bookmarks as a loader-owned localStorage side channel
- add an explicit `_map_bookmarks.entries` branch for bookmark list state
- restore/persist bookmark entries through the page-owned Datastar graph
- keep bookmark selection/placement UI in `_map_ui.bookmarks`
- keep bridge input mirroring in `_map_input.ui.bookmarks`

Status:

- implemented

Implementation:

- map shell now defines:
  - `_map_bookmarks.entries`
- `site/assets/js/pages/map-page.js` now:
  - restores bookmark entries from `fishystuff.map.bookmarks.v1`
  - persists bookmark entries from the connected Datastar signal graph
  - listens to `datastar-signal-patch` directly and debounces persistence there
- `site/assets/map/loader.js` now:
  - initializes bookmark list state from `_map_bookmarks.entries`
  - patches `_map_bookmarks.entries` when bookmark CRUD or derived metadata changes
  - mirrors `_map_bookmarks.entries` back into `_map_input.ui.bookmarks` when signal-owned bookmark state changes
  - treats bookmark list, selection, and placement changes as signal-first mutations instead of local-first state updates

Why this matters:

- bookmark list state is no longer a hidden parallel storage path
- bookmark persistence is now tied to the same Datastar signal graph as the rest of the page shell
- the bridge sees bookmark state through a Datastar-owned input branch instead of owning the canonical list itself

Validation:

- `node --check site/assets/js/pages/map-page.js`
- `node --check site/assets/map/loader.js`
- `node --test site/assets/js/pages/map-page.test.mjs site/assets/map/loader.test.mjs site/assets/map/map-host.test.mjs`
- `devenv shell -- bash -lc 'cd site && just build-release-no-tailwind'`

Notes:

- local `devenv up` site serving on `:1990` may lag behind rebuilt `.out` without a site watcher/restart
- for this slice, source checks and emitted `.out` inspection were used as the reliable validation path

## Current Evidence

Earlier live browser probe revealed duplicated canonical food state:

- `food: ["item:9359", "item:9359", "", ...]`

That explained why removing one visible selection did not clear AFR:

- one duplicate remained active in the actual signal state
- persistence then wrote the wrong state back to localStorage

Current clean-profile browser probe after the select-based transport refactor:

- before:
  - `food = ["item:9359"]`
  - `_food_slots = ["item:9359"]`
  - `afr = "72%"`
- after removing Balacs Lunchbox:
  - `food = []`
  - `_food_slots = []`
  - `afr = "65%"`
  - persisted storage contains `"food":[]`

This confirms the bug was structural and that the current calculator food-removal path is fixed in the live signal graph.

## Current Working Changes

Planned next audit targets after calculator state is stable:

- `site/content/en-US/dex.smd`
- `site/assets/js/pages/fishydex.js`
- remaining Datastar-triggered UI event glue in `site/content/en-US/calculator.smd`
- broader Fishydex render/event flow beyond persistence
- eventual map bridge replacement planning:
  - identify the current bridge contract in `site/assets/map/map-host.js`
  - identify which frontend-side map state should become Datastar-visible canonical or local signals
  - identify how the Bevy WASM runtime can consume state patches and emit state snapshots without the current custom event bridge

Most recent completed calculator slice:

- `Clear` now resets the live calculator signal graph from a backend-sourced default snapshot instead of deleting storage and reloading the whole page
- the reset snapshot is patched into local signal state as `_defaults` during calculator init
- existing signal-patch-driven eval and persistence then recompute/persist the cleared state naturally

Unrelated local changes not part of this refactor:

- `site/assets/map/loader.js`
- `site/assets/map/loader.test.mjs`

## Next Move

Expand the same analysis to the rest of the site:

- identify canonical backend-owned signals vs local UI signals
- remove imperative request/persistence plumbing where present
- align custom component boundaries with Datastar signal flow
- extract reusable components where the cleaned-up FRP model repeats
- continue with Fishydex detail/modal flows
- then start the map bridge audit/refactor plan in earnest:
  - frontend map host state
  - bridge events/commands
  - Bevy WASM patch/snapshot interface

## Current Architecture Snapshot

This section is the current handoff snapshot for a fresh Codex session.

### Datastar Event Contract

Important distinction:

- server-sent SSE event type remains `datastar-patch-signals`
- client-side reactive signal patch event is `datastar-signal-patch`

The client event name matters for:

- `site/assets/js/components/datastar-render-element.js`
- `site/assets/map/loader.js`
- any future custom element or page helper that wants to react to local signal mutation

Do not listen for `datastar-patch-signals` in browser-side reactivity helpers.

### Rust / Datastar Boundary

Current intended model:

- Axum/API-side Datastar responses should use the Rust `datastar` crate
  - current example:
    - `api/fishystuff_server/src/routes/calculator.rs`
- browser page state should use Datastar client signals as the single page-level graph
- the map Bevy/WASM runtime does **not** directly use the server-side Rust Datastar crate

Current map direction:

- Datastar owns browser page state
- `site/assets/map/loader.js` is the current adapter between:
  - Datastar signals
  - `site/assets/map/map-host.js`
  - the existing Bevy JSON patch/snapshot contract

So for the map stack:

- use Datastar as the page state model
- keep the host/wasm bridge as a temporary transport adapter
- progressively replace imperative host DOM ownership with Datastar-owned state
- only after that revisit whether the Bevy/WASM side itself should expose a more Datastar-native contract

### Map Signal Domains

Current map page signal branches:

- `_map_ui`
  - page-local shell state
  - window open/collapsed/position
  - search dropdown open state
  - bookmark placement + selected bookmark ids
  - persisted by `site/assets/js/pages/map-page.js`
- `_map_input`
  - current bridge input state made Datastar-visible
  - should become the canonical page-side source for map controls
  - currently includes the first migrated controls:
    - `filters.searchText`
    - `filters.patchId`
    - `filters.fromPatchId`
    - `filters.toPatchId`
    - `ui.diagnosticsOpen`
- `_map_bookmarks`
  - canonical page-owned bookmark list state
  - persisted by `site/assets/js/pages/map-page.js`
- `_map_runtime`
  - bridge/runtime snapshot published back into Datastar
  - current state/view/selection/hover/diagnostic mirror

### Map Refactor State

Already implemented:

- map shell root has Datastar `data-signals`
- toolbar buttons mutate `_map_ui.windowUi.*.open`
- loader syncs local window/search/bookmark UI from `_map_ui`
- loader syncs bookmark list state from `_map_bookmarks`
- loader publishes runtime snapshot into `_map_runtime`
- loader now also publishes bridge input state into `_map_input`
- loader reconciles `_map_input` back into the bridge
- bookmark persistence now lives in `site/assets/js/pages/map-page.js` as a Datastar signal-patch listener, not a template-side hidden handler
- bookmark CRUD and selection/placement paths now patch Datastar state first, then let loader signal sync reconcile local render state

Map controls currently routed through `_map_input`:

- search text
- patch range hidden inputs
- legend/diagnostics visibility
- search selection chip removals
- zone-evidence fish selection rows
- detail-pane active id
- layer visibility and layer settings
- layer ordering / clip-mask drop actions
- bookmark-selected ids and bookmark list state mirrored into bridge input state

Observed live behavior after the `_map_input` seam:

- typing in search updates:
  - `_map_input.filters.searchText`
  - `_map_runtime.inputState.filters.searchText`
- toggling a layer visibility control updates:
  - `_map_input.filters.layerIdsVisible`
  - `_map_runtime.inputState.filters.layerIdsVisible`
- diagnostics toggling updates:
  - `_map_input.ui.diagnosticsOpen`
  - `_map_runtime.inputState.ui.diagnosticsOpen`
- toolbar toggles now update both:
  - `_map_ui.windowUi.*.open`
  - actual window visibility

### Immediate Remaining Map Migration Inventory

The map is not yet fully signal-owned.

Still imperative / loader-owned today:

- many bridge input controls under layers/settings
- detail-pane selection sync
- view toggle / command dispatch
- bridge event-to-DOM rendering outside the Datastar signal graph

Recommended next map slices:

1. Move more settings/layer controls onto `_map_input`
2. Reduce direct DOM state ownership in `loader.js`
3. Revisit `site/assets/map/map-host.js` as a thinner adapter
4. Only then assess what Bevy/WASM contract changes are actually necessary
