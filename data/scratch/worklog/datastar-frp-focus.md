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
- some user-facing UI state may be persisted when it is durable and page-owned
- never sent to eval unless it is actually part of backend-owned domain input
- examples: `_distribution_tab`, dropdown open state, search text

## UI Persistence Policy

Persist UI state explicitly, not by accident.

Rules:

1. Persist durable user-facing UI settings and view choices.

Examples:

- selected calculator distribution tab
- map window open/collapsed/position state
- Fishydex search/filter/sort choices
- panel collapsed/expanded preferences

2. Do not persist runtime/transport/ephemeral state.

Examples:

- loading flags
- computed `_calc` / `_live` state
- one-shot action tokens
- transient animation/focus state

3. Keep one owner per persisted key.

- do not persist the same semantic state from both page shell and bridge/runtime layers
- when host/runtime already owns persistence, page-level state should mirror it instead of writing a second copy

4. Prefer explicit snapshot shaping over broad include/exclude heuristics.

- page modules should define which UI branches are durable
- avoid “all underscore signals are ephemeral” and avoid “persist every non-underscore signal” as the only rule

5. Keep share/export payloads separate from persisted local UI state.

- local view state should not leak into preset/share URLs unless explicitly desired

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

### Step 12

Make map search-open and reset shell state signal-first.

Work:

- stop mutating `searchUiState.open` directly in event handlers before patching `_map_ui.search`
- stop assigning reset-time window/search/bookmark shell state locally before patching `_map_ui`
- let the existing loader signal-sync path own the local reconciliation/render pass

Status:

- implemented

Implementation:

- search input/focus/selection-close paths now patch `_map_ui.search` first
- reset UI now patches:
  - `_map_ui.windowUi`
  - `_map_ui.search`
  - `_map_ui.bookmarks`
  from reset snapshots instead of assigning loader locals first

Why this matters:

- it reduces another class of local-first shell state mutation in `loader.js`
- it keeps map shell state changes aligned with the same Datastar-first pattern already used for bookmark state
- it narrows the loader’s role toward reconciliation/rendering instead of canonical state ownership

### Step 13

Make map managed-window state signal-first.

Work:

- stop mutating `windowUiState` locally before patching `_map_ui.windowUi`
- remove the separate `persistWindowUiState()` helper from `loader.js`
- make open/collapse/drag/resize/reset window flows patch `_map_ui.windowUi` first
- let the existing `syncLocalUiStateFromSignals()` path reconcile local window state and rerender

Status:

- implemented

Implementation:

- `updateWindowUiEntry()` now computes the next window state and patches `_map_ui.windowUi`
  instead of mutating `windowUiState` directly
- `applyManagedWindows()` is now purely a visibility/clamping pass
- open/collapse/drag-finish flows no longer request persistence separately
- reset UI and resize flows now patch/reset the Datastar window state and then apply the
  managed windows without any extra persistence hook

Why this matters:

- it removes another remaining local-first state island in `loader.js`
- map window state now follows the same signal-first ownership model as bookmarks and search
- it narrows `loader.js` further toward rendering/reconciliation instead of canonical shell-state ownership

Validation:

- `node --check site/assets/map/loader.js`
- `node --test site/assets/js/pages/map-page.test.mjs site/assets/map/loader.test.mjs site/assets/map/map-host.test.mjs`
- `devenv shell -- bash -lc 'cd site && just build-release-no-tailwind'`
- served-vs-`.out` spot checks matched for:
  - `http://127.0.0.1:1990/map/`
  - `http://127.0.0.1:1990/map/loader.js`
- live Chromium validation confirmed:
  - toggling Settings updates the visible window state cleanly
  - the managed window hides/shows without relying on a separate persistence path

### Step 14

Move Fishydex signal-patch persistence into the page helper.

Work:

- remove the template-level `data-on-signal-patch` persistence hook from `dex.smd`
- let `fishydex.js` own its own Datastar signal-patch listener and debounce window
- keep the persisted signal filter logic in JS, next to the actual persistence implementation

Status:

- implemented

Implementation:

- `site/assets/js/pages/fishydex.js` now:
  - binds one `datastar-signal-patch` listener on restore
  - filters out ephemeral Fishydex signals in JS
  - debounces persistence before calling `persistSignals()`
- `site/content/en-US/dex.smd` no longer needs the hidden template-side persistence node

Why this matters:

- it removes another Datastar persistence concern from template markup
- it aligns Fishydex persistence ownership with the map page’s Datastar signal-patch model
- it keeps the “what should persist?” logic beside the persistence implementation instead of splitting it across template and JS

Validation:

- `node --check site/assets/js/pages/fishydex.js`
- `devenv shell -- bash -lc 'cd site && just build-release-no-tailwind'`
- served-vs-`.out` spot checks matched for:
  - `http://127.0.0.1:1990/dex/`
  - `http://127.0.0.1:1990/js/pages/fishydex.js`
- live Chromium validation confirmed:
  - updating the Dex search field changes `fishystuff.fishydex.ui.v1`
  - the persisted search value reflects the latest Datastar signal state without relying on template-side patch handlers

### Step 15

Extract a shared Datastar signal-patch persistence helper.

Work:

- remove duplicated signal-patch debounce/listener/filter code from:
  - `site/assets/js/pages/map-page.js`
  - `site/assets/js/pages/fishydex.js`
- centralize the reusable logic in one browser helper loaded from the base template
- keep page-specific storage serialization logic in the page helpers themselves

Status:

- implemented

Implementation:

- added `site/assets/js/datastar-persist.js`
- base layout now loads that helper globally as `window.__fishystuffDatastarPersist`
- the helper provides:
  - `patchMatchesSignalFilter(...)`
  - `createDebouncedSignalPatchPersistor(...)`
- `map-page.js` now uses the shared helper for include-filtered persistence of:
  - `_map_ui`
  - `_map_bookmarks`
- `fishydex.js` now uses the same helper for exclude-filtered persistence of:
  - non-ephemeral Fishydex page signals

Why this matters:

- it removes another repeated Datastar lifecycle pattern from page-specific scripts
- it gives future Datastar pages/components one battle-tested persistence/listener path
- it keeps the site moving toward a smaller set of reusable Datastar primitives instead of page-specific glue

Validation:

- `node --check site/assets/js/datastar-persist.js`
- `node --check site/assets/js/pages/map-page.js`
- `node --check site/assets/js/pages/fishydex.js`
- `node --test site/assets/js/datastar-persist.test.mjs site/assets/js/pages/map-page.test.mjs site/assets/map/loader.test.mjs site/assets/map/map-host.test.mjs`
- `devenv shell -- bash -lc 'cd site && just build-release-no-tailwind'`
- served-vs-`.out` spot checks matched for:
  - `http://127.0.0.1:1990/map/`
  - `http://127.0.0.1:1990/js/pages/map-page.js`
  - `http://127.0.0.1:1990/js/datastar-persist.js`
  - `http://127.0.0.1:1990/dex/`
  - `http://127.0.0.1:1990/js/pages/fishydex.js`
- live Chromium validation confirmed:
  - updating the Dex search field still persists to `fishystuff.fishydex.ui.v1`

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
- reusable `window.__fishystuffDatastarState` helper exists for object-path reads/patches from Datastar expressions
- toolbar buttons mutate `_map_ui.windowUi.*.open`
- loader syncs local window/search/bookmark UI from `_map_ui`
- loader syncs bookmark list state from `_map_bookmarks`
- loader publishes runtime snapshot into `_map_runtime`
- loader now also publishes bridge input state into `_map_input`
- loader reconciles `_map_input` back into the bridge
- bookmark persistence now lives in `site/assets/js/pages/map-page.js` as a Datastar signal-patch listener, not a template-side hidden handler
- bookmark CRUD and selection/placement paths now patch Datastar state first, then let loader signal sync reconcile local render state
- search dropdown and reset-time shell state now patch `_map_ui` first instead of mutating loader locals before signal updates
- managed window open/collapse/drag/resize/reset flows now patch `_map_ui.windowUi` first instead of mutating/persisting window state locally
- map toolbar buttons now use the shared Datastar state helper directly instead of the page-specific `window.__fishystuffMap.toggleWindow(...)`

Map controls currently routed through `_map_input`:

- search text
- patch range hidden inputs
- legend/diagnostics visibility
- desired view mode
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

Important loader guard:

- `publishMapInputSignals(...)` now suppresses `syncBridgeInputStateFromSignals()` re-entry while the loader is mirroring bridge-owned `_map_input` back into Datastar
- without that guard, the map page can fall into a `_map_input` feedback loop:
  - bridge/runtime publish
  - Datastar signal patch
  - signal-to-bridge sync listener
  - rerender
  - bridge/runtime publish again
- if a future refactor changes `_map_input` publication timing, keep this loop boundary in mind first
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

### Step 17 - Signal-First Map View Mode

Completed:

- moved the map view-mode toggle off direct bridge commands and onto `_map_input.ui.viewMode`
- extended `site/assets/map/map-host.js` so host input state understands `ui.viewMode`
- host now translates desired `ui.viewMode` changes into `setViewMode` commands
- host also mirrors actual runtime mode back into `inputState.ui.viewMode` on `view-changed`
- loader view button now patches `_map_input` instead of calling `dispatchMapCommand(...)` directly

Why this matters:

- the 2D/3D button is now in the Datastar state flow instead of bypassing it
- the desired mode and actual runtime mode can both be inspected through signals
- this is the right pattern for stateful map controls:
  - UI mutates signals
  - host reconciles signals to the bridge/runtime contract
  - runtime mirrors final state back into signals

Validation:

- `node --test site/assets/js/datastar-state.test.mjs site/assets/js/datastar-persist.test.mjs site/assets/js/pages/map-page.test.mjs site/assets/map/loader.test.mjs site/assets/map/map-host.test.mjs`
- rebuilt site output
- compared:
  - served `/map/loader.js` vs `site/.out/map/loader.js`
  - served `/map/map-host.js` vs `site/.out/map/map-host.js`
- live Chromium smoke:
  - reload `/map/`
  - verify `FishyMapBridge.getCurrentInputState().ui.viewMode === "2d"`
  - verify `_map_input.ui.viewMode === "2d"`
  - click the view toggle
  - verify bridge input state, `_map_input`, and `_map_runtime.state.view.viewMode` all become `"3d"`

### Step 18 - Shared Datastar Signal Store Helper

Completed:

- extended `site/assets/js/datastar-state.js` with `createSignalStore()`
- moved the repeated page-local Datastar signal plumbing onto that helper:
  - `site/assets/js/pages/map-page.js`
  - `site/assets/js/pages/fishydex.js`
- pages now share one pattern for:
  - `connect(signals)`
  - `signalObject()`
  - `patchSignals(patch)`
  - `readSignal(path)`

Why this matters:

- this removes another repeated slice of Datastar glue code
- page modules now rely on the same signal-store semantics instead of each hand-rolling their own minimal variant
- it makes further extraction easier because signal access/patching is now standardized across pages

Validation:

- `node --test site/assets/js/datastar-state.test.mjs site/assets/js/datastar-persist.test.mjs site/assets/js/pages/map-page.test.mjs site/assets/map/loader.test.mjs site/assets/map/map-host.test.mjs`
- rebuilt site output
- compared:
  - served `/js/datastar-state.js` vs `site/.out/js/datastar-state.js`
  - served `/js/pages/map-page.js` vs `site/.out/js/pages/map-page.js`
  - served `/js/pages/fishydex.js` vs `site/.out/js/pages/fishydex.js`
- live Chromium smoke:
  - map reloads cleanly and still reflects signal-first view mode
  - Dex search updates the catalog immediately
  - after the normal debounce, `fishystuff.fishydex.ui.v1` persists the new `search_query`

### Step 19 - Template-Driven Map View Toggle

Completed:

- moved the map view toggle button itself into a Datastar expression in `site/layouts/map.shtml`
- removed the corresponding loader-owned `click` listener from `site/assets/map/loader.js`
- the button now mutates `_map_input.ui.viewMode` directly from the template, using `_map_runtime.state.view.viewMode` to choose the next mode

Why this matters:

- this removes another imperative DOM event path from `loader.js`
- the view toggle now matches the same Datastar-first pattern as the toolbar buttons
- it keeps control intent in the template and reconciliation in the host/runtime layer

Validation:

- `node --test site/assets/js/datastar-state.test.mjs site/assets/js/datastar-persist.test.mjs site/assets/js/pages/map-page.test.mjs site/assets/map/loader.test.mjs site/assets/map/map-host.test.mjs`
- rebuilt site output
- compared:
  - served `/map/` vs `site/.out/map/index.html`
  - served `/map/loader.js` vs `site/.out/map/loader.js`
- live Chromium smoke:
- reload `/map/`
- verify the page boots in persisted 3D mode cleanly
- click the view toggle
- verify bridge input state, `_map_input`, and `_map_runtime.state.view.viewMode` all return to `"2d"`

### Step 20 - Template-Driven Map Settings Toggles

Completed:

- moved the map `Auto-adjust view` checkbox onto direct Datastar binding in `site/layouts/map.shtml`
- removed the corresponding loader-owned checkbox change listener from `site/assets/map/loader.js`
- moved the map `Diagnostics` disclosure state update into a Datastar template expression in `site/layouts/map.shtml`
- removed the corresponding loader-owned diagnostics `toggle` listener from `site/assets/map/loader.js`

Why this matters:

- the settings panel now owns two more controls directly in the Datastar graph instead of mirroring DOM state back into signals
- `Auto-adjust view` follows the same checkbox binding model as the rest of the Datastar form surface
- `Diagnostics` now expresses user intent from the template while the bridge/runtime continue to reconcile and mirror actual state
- this shrinks `loader.js` further toward an adapter role instead of a DOM event owner role

Validation:

- `node --test site/assets/js/datastar-state.test.mjs site/assets/js/datastar-persist.test.mjs site/assets/js/pages/map-page.test.mjs site/assets/map/loader.test.mjs site/assets/map/map-host.test.mjs`
- rebuilt site output
- compared:
  - served `/map/` vs `site/.out/map/index.html`
  - served `/map/loader.js` vs `site/.out/map/loader.js`
- live Chromium smoke:
  - reload `/map/`
  - verify `_map_ui.windowUi.settings.autoAdjustView === true`
  - toggle `Auto-adjust view`
  - verify `_map_ui.windowUi.settings.autoAdjustView === false`
  - expand `Diagnostics`
  - verify `_map_input.ui.diagnosticsOpen === true`
  - verify `_map_runtime.inputState.ui.diagnosticsOpen === true`

### Step 21 - Signal-Driven Map Reset View Action

Completed:

- introduced a local `_map_actions.resetViewToken` signal in `site/layouts/map.shtml`
- moved the `Reset view` button into a Datastar template expression that increments that token
- added loader-side action-signal reconciliation in `site/assets/map/loader.js`
- removed the old loader-owned `Reset view` click listener

Why this matters:

- `Reset view` is not persistent input state, so it should not be modeled as part of `_map_input`
- using a local action-signal branch keeps the intent inside the Datastar graph without pretending it is durable state
- this establishes a reusable pattern for one-shot map UI actions that still need to cross into the bridge/runtime
- it removes another direct DOM-to-bridge path from `loader.js`

Validation:

- `node --test site/assets/js/datastar-state.test.mjs site/assets/js/datastar-persist.test.mjs site/assets/js/pages/map-page.test.mjs site/assets/map/loader.test.mjs site/assets/map/map-host.test.mjs`
- rebuilt site output
- compared:
  - served `/map/` vs `site/.out/map/index.html`
  - served `/map/loader.js` vs `site/.out/map/loader.js`
- live Chromium smoke:
  - reload `/map/`
  - verify `_map_actions.resetViewToken === 0`
- instrument `FishyMapBridge.sendCommand(...)`
- click `Reset view`
- verify `_map_actions.resetViewToken === 1`
- verify the bridge receives `{ resetView: true }`

### Step 22 - Template-Driven Map Search Open State

Completed:

- moved the map search box `input` and `focus` open-state behavior into Datastar template expressions in `site/layouts/map.shtml`
- removed the corresponding loader-owned `input` and `focus` listeners from `site/assets/map/loader.js`

Why this matters:

- the search field already binds canonical query state through `_map_input.filters.searchText`
- the companion local UI state `_map_ui.search.open` should be driven from the same template surface, not from loader-owned DOM listeners
- this removes another direct DOM-to-signal bridge in `loader.js`
- the search shell now follows the same pattern as the toolbar and settings controls:
  - template mutates signals
  - loader renders from signal state

Validation:

- `node --test site/assets/js/datastar-state.test.mjs site/assets/js/datastar-persist.test.mjs site/assets/js/pages/map-page.test.mjs site/assets/map/loader.test.mjs site/assets/map/map-host.test.mjs`
- rebuilt site output
- compared:
  - served `/map/` vs `site/.out/map/index.html`
  - served `/map/loader.js` vs `site/.out/map/loader.js`
- live Chromium smoke:
  - reload `/map/`
  - type `manta` into the search field
- verify `_map_input.filters.searchText === "manta"`
- verify `_map_ui.search.open === true`
- verify the results list is visible with `2 matches`

### Step 23 - Signal-Driven Map Patch Range Normalization

Completed:

- removed the map patch-window `change` listeners for `from` and `to` from `site/assets/map/loader.js`
- moved patch-range ordering and `patchId` derivation into a Datastar signal-patch reconciliation step in `site/assets/map/loader.js`
- the loader now normalizes `_map_input.filters.fromPatchId` / `_map_input.filters.toPatchId` from signal state before bridge sync

Why this matters:

- the patch pickers already bind into canonical Datastar signals through hidden inputs
- the remaining loader `change` handlers were only there to canonicalize those signal values after the fact
- canonicalization belongs in the signal reconciliation path, not in DOM event ownership
- this keeps the bridge fed from normalized Datastar state even when signals are patched programmatically or restored from persisted state

Validation:

- `node --test site/assets/js/datastar-state.test.mjs site/assets/js/datastar-persist.test.mjs site/assets/js/pages/map-page.test.mjs site/assets/map/loader.test.mjs site/assets/map/map-host.test.mjs`
- rebuilt site output
- compared:
  - served `/map/` vs `site/.out/map/index.html`
  - served `/map/loader.js` vs `site/.out/map/loader.js`
- live Chromium smoke:
  - reload `/map/`
  - patch `_map_input.filters.fromPatchId` / `toPatchId` into reversed order
  - verify `_map_input.filters` canonicalizes them into chronological order
  - verify `_map_runtime.inputState.filters` receives the canonical order
- patch both to the same patch id
- verify `_map_input.filters.patchId` is derived to that same id

### Step 24 - Explicit Calculator UI Persistence Branch

Completed:

- moved calculator distribution tab ownership off the server-rendered fragment-local `_distribution_tab`
- introduced a page-owned `_calculator_ui.distribution_tab` branch in `site/content/en-US/calculator.smd`
- updated `api/fishystuff_server/src/routes/calculator.rs` to bind tabs against `$_calculator_ui.distribution_tab`
- changed calculator persistence shaping so `_calculator_ui` is stored explicitly
- kept `_calculator_ui` excluded from eval traffic and from preset/share payloads
- added legacy normalization from `_distribution_tab` into `_calculator_ui.distribution_tab`

Why this matters:

- the selected calculator tab is durable user-facing UI state and should persist cleanly
- it should not be owned by a server-rendered fragment-local signal that can be reintroduced on each patch
- the page shell now owns calculator-local UI state, while the backend continues to own canonical calculator inputs and computed outputs
- this matches the broader persistence policy:
  - persist durable page-owned UI state explicitly
  - do not send it to eval
  - do not leak it into share/preset payloads

Validation:

- `cargo test --offline -p fishystuff_server`
- rebuilt site output
- compared:
  - served `/calculator/` vs `site/.out/calculator/index.html`
- note:
  - the local API watcher was still serving the old calculator fragment during this slice
  - route/source changes and tests are current, but live browser validation of the new tab binding needs a fresh API rebuild serving the updated fragment

### Step 16 - Shared Datastar State Helper

Completed:

- extracted `site/assets/js/datastar-state.js` as a shared helper for Datastar expression-friendly nested object state updates
- exposed:
  - `readObjectPath(root, path)`
  - `setObjectPath(root, path, value)`
  - `toggleBooleanPath($, path)`
- added `site/assets/js/datastar-state.test.mjs`
- loaded the helper from the base template so any page can use it
- replaced the map page's toolbar-only `window.__fishystuffMap.toggleWindow(...)` helper with direct `data-on:click` calls into `window.__fishystuffDatastarState.toggleBooleanPath(...)`

Why this matters:

- this removes one more page-specific Datastar bridge helper
- the map toolbar now uses a reusable Datastar-oriented state primitive instead of bespoke map glue
- it keeps template interactions closer to "mutate signals, let the rest react"

Validation:

- `node --test site/assets/js/datastar-state.test.mjs site/assets/js/datastar-persist.test.mjs site/assets/js/pages/map-page.test.mjs site/assets/map/loader.test.mjs site/assets/map/map-host.test.mjs`
- rebuilt site output
- compared:
  - served `/map/` vs `site/.out/map/index.html`
  - served `/js/datastar-state.js` vs `site/.out/js/datastar-state.js`
  - served `/js/pages/map-page.js` vs `site/.out/js/pages/map-page.js`
- live Chromium smoke:
  - reload `/map/`
  - verify no stack overflow on load
  - toggle Search from the toolbar
  - confirm `_map_ui.windowUi.search.open` drives visibility correctly

### Step 25 - Fishydex Panel State Becomes Page-Owned UI

Completed:

- removed Fishydex panel collapse persistence from the shared `fishystuff.ui.settings.v1` bucket
- moved Fishydex panel collapse state into the page-owned `fishystuff.fishydex.ui.v1` snapshot alongside:
  - search
  - filter/sort controls
- added `site/assets/js/pages/fishydex.test.mjs` coverage for Fishydex UI restore/persist behavior

Why this matters:

- Fishydex panel collapse state was previously a split-owner case:
  - page UI mostly lived in `fishystuff.fishydex.ui.v1`
  - panel collapse lived in `fishystuff.ui.settings.v1`
- after this slice, Fishydex has one page-owned UI snapshot instead of mixed ownership

Validation:

- `node --check site/assets/js/pages/fishydex.js`
- `node --check site/assets/js/pages/fishydex.test.mjs`
- `node --test site/assets/js/pages/fishydex.test.mjs`
- rebuilt site output
- compare served vs `.out` for:
  - `/dex/`
  - `/js/pages/fishydex.js`
- live Chromium smoke:
  - collapse Fishydex Progress/Filter panels, reload, verify both restore from `fishystuff.fishydex.ui.v1`

### Step 26 - Shared App Theme Settings

Completed:

- moved theme preference ownership onto the shared `fishystuff.ui.settings.v1` store under:
  - `app.theme.selected`
- updated the base template early theme boot script to read the shared app UI theme preference before paint
- updated `site/assets/js/theme.js` so new writes go through the shared UI settings store
- removed the nav template’s direct writes to the legacy raw `theme` key
- kept a one-way legacy fallback from the old raw `theme` key during read, while new writes go only to the shared UI settings store

Why this matters:

- theme preference is app-wide UI state, so it should not live in a page-owned store
- the previous setup still had a hidden second owner:
  - the nav template wrote directly to `localStorage['theme']`
- after this slice:
  - theme has one app-owned settings path
  - page templates route through the shared `window.__theme` interface

Validation:

- `node --check site/assets/js/theme.js`
- rebuilt site output
- compare served vs `.out` for:
  - `/dex/`
  - `/js/theme.js`
- live Chromium smoke:
  - set theme to `light`
  - verify `fishystuff.ui.settings.v1.app.theme.selected === "light"`
  - verify `localStorage['theme'] === null`
  - reload and verify `data-theme="light"` persists

### Step 27 - Shared Datastar Signal Store Uses Nested Patch Semantics

Completed:

- updated `site/assets/js/datastar-state.js` so `createSignalStore().patchSignals(...)` merges nested object patches in place instead of using a shallow top-level `Object.assign(...)`
- exported the merge helper from the shared Datastar state module
- aligned the page-local fallback signal stores in:
  - `site/assets/js/pages/map-page.js`
  - `site/assets/js/pages/fishydex.js`
  with the same nested patch semantics
- added regression coverage in `site/assets/js/datastar-state.test.mjs`

Why this matters:

- shallow patch semantics were destructive for branch-shaped Datastar state
- a nested patch like:
  - `{ _map_ui: { search: { open: true } } }`
  could replace the entire `_map_ui` branch in fallback/store-helper paths
- reusable signal patches now behave like state patches, not branch replacement
- this reduces page-specific defensive code and makes page-owned Datastar state safer to mutate incrementally

Validation:

- `node --check site/assets/js/datastar-state.js`
- `node --check site/assets/js/datastar-state.test.mjs`
- `node --check site/assets/js/pages/map-page.js`
- `node --check site/assets/js/pages/fishydex.js`
- `node --test site/assets/js/datastar-state.test.mjs`
- rebuilt site output
- compared served vs `.out` for:
  - `/js/datastar-state.js`
  - `/js/pages/map-page.js`
  - `/js/pages/fishydex.js`

### Step 28 - Map Page Persists Only Durable Page-Owned UI

Completed:

- narrowed `site/assets/js/pages/map-page.js` persistence filtering so page-owned map persistence reacts only to:
  - `_map_ui.windowUi`
  - `_map_bookmarks.entries`
- explicitly kept these `_map_ui` branches live-only / ephemeral:
  - `_map_ui.search`
  - `_map_ui.bookmarks`
- added focused tests in `site/assets/js/pages/map-page.test.mjs` for:
  - ignoring ephemeral `_map_ui.search` patches
  - persisting durable `_map_ui.windowUi` patches

Why this matters:

- the previous `map-page` signal-patch filter was still too broad at the `_map_ui` branch level
- page-owned persistence should store durable shell/window state only
- live UI affordances like search dropdown openness or bookmark placement mode should not dirty durable storage

Validation:

- `node --check site/assets/js/pages/map-page.test.mjs`
- `node --test site/assets/js/pages/map-page.test.mjs`
- rebuilt site output
- compared served vs `.out` for:
  - `/map/`
  - `/js/pages/map-page.js`
- live Chromium smoke:
  - clear `fishystuff.map.window_ui.v1`
  - patch `_map_ui.search.open = true`
  - verify no storage write occurs
  - patch `_map_ui.windowUi.search.open = false`
  - verify `fishystuff.map.window_ui.v1` is written with the durable window snapshot

### Step 29 - Remove Duplicate Map Detail Pane Persistence

Completed:

- removed `activeDetailPaneId` from the bridge-owned map input-state contract in:
  - `site/assets/map/map-host.js`
- removed the loader-side sync that mirrored resolved zone-info tabs back into bridge input state in:
  - `site/assets/map/loader.js`
- kept zone-info tab ownership solely in page-owned Datastar state:
  - `_map_ui.windowUi.zoneInfo.tab`

Why this matters:

- the map had two copies of the same UI selection:
  - page-owned `windowUi.zoneInfo.tab`
  - bridge-owned `inputState.ui.activeDetailPaneId`
- the bridge copy never left JS and was not required by the WASM runtime
- keeping both copies made persistence and ownership harder to reason about
- after this slice, the zone-info tab has one clear owner: the Datastar page UI state

Validation:

- `node --check site/assets/map/map-host.js`
- `node --check site/assets/map/loader.js`
- `node --test site/assets/map/loader.test.mjs site/assets/map/map-host.test.mjs`
- rebuilt site output
- compared served vs `.out` for:
  - `/map/`
  - `/map/loader.js`
  - `/map/map-host.js`
- live Chromium smoke:
  - inspect `FishyMapBridge.createPrefsSnapshot()` / `createSessionSnapshot()`
  - verify neither includes `activeDetailPaneId`
  - patch `_map_ui.windowUi.zoneInfo.tab = "zone_info"`
  - verify `fishystuff.map.window_ui.v1` stores the tab
  - verify `fishystuff.map.prefs.v1` and `fishystuff.map.session.v1` do not

### Step 30 - Keep Bookmark Selection Live-Only

Completed:

- removed `bookmarkSelectedIds` from bridge session snapshot persistence in:
  - `site/assets/map/map-host.js`
- kept `bookmarkSelectedIds` in live bridge input state for current render/runtime coordination
- updated `site/assets/map/map-host.test.mjs` accordingly

Why this matters:

- bookmark selection already has a page-owned live UI owner:
  - `_map_ui.bookmarks.selectedIds`
- persisting the same selection through bridge session snapshots was another ownership leak
- after this slice:
  - bookmark selection can still exist live in runtime input state
  - but it does not get restored/persisted from the bridge storage layer

Validation:

- `node --check site/assets/map/map-host.js`
- `node --check site/assets/map/map-host.test.mjs`
- `node --test site/assets/map/map-host.test.mjs`
- rebuilt site output
- compared served vs `.out` for:
  - `/map/map-host.js`
- live Chromium smoke:
  - set `FishyMapBridge.inputState.ui.bookmarkSelectedIds = ['bookmark-a', 'bookmark-b']`
  - verify `createSessionSnapshot().ui` omits `bookmarkSelectedIds`
  - verify `createPrefsSnapshot().ui` omits `bookmarkSelectedIds`

### Step 31 - Fix Map Window UI Restore Contract

Completed:

- fixed `site/assets/js/pages/map-page.js` so `fishystuff.map.window_ui.v1` restores back into:
  - `_map_ui.windowUi`
  instead of leaking into a stray top-level `windowUi` signal branch
- added regression coverage in `site/assets/js/pages/map-page.test.mjs`

Why this matters:

- page-owned map window UI persistence was effectively restoring into the wrong signal shape
- saved window UI state could exist in storage without actually rehydrating the Datastar-owned `_map_ui` branch
- this needed to be fixed before broadening map page-owned UI persistence further

Validation:

- `node --check site/assets/js/pages/map-page.js`
- `node --check site/assets/js/pages/map-page.test.mjs`
- `node --test site/assets/js/pages/map-page.test.mjs`
- rebuilt site output
- compared served vs `.out` for:
  - `/map/`
  - `/js/pages/map-page.js`
- live Chromium smoke:
  - write `fishystuff.map.window_ui.v1` with `search.open = false` and `zoneInfo.tab = "zone_info"`
  - reload `/map/`
  - verify:
    - `_map_ui.windowUi.search.open === false`
    - `_map_ui.windowUi.zoneInfo.tab === "zone_info"`
    - no stray top-level `windowUi` branch is used

### Step 32 - Move Remaining Map Display UI Into Page-Owned Signals

Completed:

- expanded page-owned map UI persistence in `site/assets/js/pages/map-page.js` to include:
  - `_map_input.ui.diagnosticsOpen`
  - `_map_input.ui.legendOpen`
  - `_map_input.ui.leftPanelOpen`
  - `_map_input.ui.showPoints`
  - `_map_input.ui.showPointIcons`
  - `_map_input.ui.pointIconScale`
- kept that value in the same page-owned UI storage snapshot alongside `windowUi`
- removed the remaining bridge-persisted display UI fields from prefs/session snapshot persistence in:
  - `site/assets/map/map-host.js`
- added regression coverage in `site/assets/js/pages/map-page.test.mjs`

Why this matters:

- these map display controls were already living in `_map_input.ui` at runtime
- but their persistence still lived in bridge-owned prefs/session storage
- after this slice, the page-owned Datastar signal graph owns both:
  - live state
  - persisted state
- the bridge still consumes the live values, but it no longer owns restoring or saving them

Validation:

- `node --check site/assets/js/pages/map-page.js`
- `node --check site/assets/js/pages/map-page.test.mjs`
- `node --check site/assets/map/map-host.js`
- `node --test site/assets/js/pages/map-page.test.mjs site/assets/map/map-host.test.mjs`
- rebuilt site output
- compared served vs `.out` for:
  - `/map/`
  - `/js/pages/map-page.js`
  - `/map/map-host.js`
- live Chromium smoke:
  - patch `_map_input.ui.diagnosticsOpen = true`
  - verify `fishystuff.map.window_ui.v1` stores `inputUi.diagnosticsOpen === true`
  - verify `FishyMapBridge.createSessionSnapshot().ui` omits `diagnosticsOpen`
  - verify `FishyMapBridge.createSessionSnapshot().ui` omits:
    - `legendOpen`
    - `leftPanelOpen`
    - `showPoints`
    - `showPointIcons`
    - `pointIconScale`
  - reload `/map/`
  - verify `_map_input.ui.diagnosticsOpen === true`

### Step 33 - Move Map Filter UI Persistence Into Page-Owned Signals

Completed:

- expanded page-owned map UI persistence in `site/assets/js/pages/map-page.js` to include:
  - `_map_input.filters.searchText`
  - `_map_input.filters.fromPatchId`
  - `_map_input.filters.toPatchId`
- kept these fields in the same `fishystuff.map.window_ui.v1` page-owned snapshot under:
  - `inputFilters`
- removed these fields from bridge-owned restore/snapshot persistence in:
  - `site/assets/map/map-host.js`
  - `snapshotToRestorePatch(...)`
  - `createSessionSnapshot()`
  - `createPrefsSnapshot()`
- added regression coverage in:
  - `site/assets/js/pages/map-page.test.mjs`
  - `site/assets/map/map-host.test.mjs`

Why this matters:

- search query and patch-range selection are page-visible filter controls already owned live by the Datastar signal graph
- persisting them through bridge prefs/session storage kept the map split across two owners:
  - page-owned live state
  - bridge-owned restore state
- after this slice:
  - Datastar page state owns both live and persisted filter UI values
  - bridge prefs/session no longer restore or save them
  - query-string restore still works separately through `parseQueryState(...)`
- loader-side canonicalization still applies after restore:
  - invalid or reversed patch ranges are normalized through `syncPatchRangeFromSignals()`

Validation:

- `node --check site/assets/js/pages/map-page.js`
- `node --check site/assets/map/map-host.js`
- `node --test site/assets/js/pages/map-page.test.mjs site/assets/map/map-host.test.mjs`
- rebuilt site output
- compared served vs `.out` for:
  - `/map/`
  - `/js/pages/map-page.js`
  - `/map/map-host.js`
- live Chromium smoke:
  - patch `_map_input.filters.searchText`, `fromPatchId`, `toPatchId`
  - verify `fishystuff.map.window_ui.v1` stores them under `inputFilters`
  - verify `FishyMapBridge.createSessionSnapshot().filters` omits:
    - `searchText`
    - `patchId`
    - `fromPatchId`
    - `toPatchId`
  - verify `FishyMapBridge.createPrefsSnapshot().filters` omits the same fields
  - reload `/map/`
  - verify `_map_input.filters` restores from page-owned storage

### Step 34 - Move Map Layer Display Filters Into Page-Owned Signals

Completed:

- expanded page-owned map UI persistence in `site/assets/js/pages/map-page.js` to include the remaining layer-display filter controls under:
  - `_map_input.filters.layerIdsVisible`
  - `_map_input.filters.layerIdsOrdered`
  - `_map_input.filters.layerOpacities`
  - `_map_input.filters.layerClipMasks`
  - `_map_input.filters.layerWaypointConnectionsVisible`
  - `_map_input.filters.layerWaypointLabelsVisible`
  - `_map_input.filters.layerPointIconsVisible`
  - `_map_input.filters.layerPointIconScales`
- kept these fields in the same page-owned `fishystuff.map.window_ui.v1` snapshot under:
  - `inputFilters`
- removed these fields from bridge-owned restore/snapshot persistence in:
  - `site/assets/map/map-host.js`
  - `snapshotToRestorePatch(...)`
  - `createSessionSnapshot()`
  - `createPrefsSnapshot()`
- updated regression coverage in:
  - `site/assets/js/pages/map-page.test.mjs`
  - `site/assets/map/map-host.test.mjs`

Why this matters:

- these controls are visible, user-driven layer UI state, but persistence still lived in bridge prefs/session storage
- that kept the map in a split ownership model:
  - Datastar-owned live state in `_map_input.filters`
  - bridge-owned restore state for the same fields
- after this slice:
  - page-owned Datastar state owns both live and persisted layer-display filter state
  - bridge prefs/session no longer restore or save these layer-display values
  - the bridge still consumes the live values and forwards them to the WASM runtime

Validation:

- `node --check site/assets/js/pages/map-page.js`
- `node --check site/assets/map/map-host.js`
- `node --test site/assets/js/pages/map-page.test.mjs site/assets/map/map-host.test.mjs`
- rebuilt site output
- compared served vs `.out` for:
  - `/map/`
  - `/js/pages/map-page.js`
  - `/map/map-host.js`
- live Chromium smoke:
  - patch `_map_input.filters.layerIdsVisible`, `layerIdsOrdered`, `layerOpacities`, `layerClipMasks`, `layerWaypointConnectionsVisible`, `layerWaypointLabelsVisible`, `layerPointIconsVisible`, `layerPointIconScales`
  - verify `fishystuff.map.window_ui.v1` stores them under `inputFilters`
  - verify `FishyMapBridge.createSessionSnapshot().filters` omits these layer-display keys
  - verify `FishyMapBridge.createPrefsSnapshot().filters` omits these layer-display keys
  - reload `/map/`
  - verify `_map_input.filters` restores the stored layer-display values

### Step 35 - Move Remaining Selected Map Filters Into Page-Owned Signals

Completed:

- expanded page-owned map UI persistence in `site/assets/js/pages/map-page.js` to include the remaining selected-filter controls under:
  - `_map_input.filters.fishIds`
  - `_map_input.filters.zoneRgbs`
  - `_map_input.filters.semanticFieldIdsByLayer`
  - `_map_input.filters.fishFilterTerms`
- kept these values under the same page-owned `fishystuff.map.window_ui.v1` snapshot:
  - `inputFilters`
- removed these fields from bridge-owned restore/snapshot persistence in:
  - `site/assets/map/map-host.js`
  - `snapshotToRestorePatch(...)`
  - `createSessionSnapshot()`
- updated regression coverage in:
  - `site/assets/js/pages/map-page.test.mjs`
  - `site/assets/map/map-host.test.mjs`

Why this matters:

- these were the last remaining user-facing filter selections still restored by bridge session storage
- that meant the map still had a split ownership model for visible filter state:
  - Datastar-owned live filter signals
  - bridge-owned restore state for some selected filters
- after this slice:
  - page-owned Datastar state owns all visible map filter UI persistence
  - bridge-owned persistence is now primarily about runtime/session state such as:
    - selection
    - view/camera
    - runtime-driven restore commands

Validation:

- `node --check site/assets/js/pages/map-page.js`
- `node --check site/assets/map/map-host.js`
- `node --test site/assets/js/pages/map-page.test.mjs site/assets/map/map-host.test.mjs`
- rebuilt site output
- compared served vs `.out` for:
  - `/map/`
  - `/js/pages/map-page.js`
  - `/map/map-host.js`
- live Chromium smoke:
  - patch `_map_input.filters.fishIds`, `semanticFieldIdsByLayer`, `fishFilterTerms`
  - verify `fishystuff.map.window_ui.v1` stores them under `inputFilters`
  - verify `FishyMapBridge.createSessionSnapshot().filters` is now empty for these page-owned filter fields
  - reload `/map/`
  - verify the stored selected-filter values rehydrate into `_map_input.filters`

### Step 36 - Fix Map Query-Precedence and Startup Signal Sync

Completed:

- tightened page-owned restore precedence in `site/assets/js/pages/map-page.js` so stored
  `fishystuff.map.window_ui.v1` state does not override query-owned map input on load
- added query-aware stripping for page-owned restore fields under:
  - `_map_input.ui.diagnosticsOpen`
  - `_map_input.ui.legendOpen`
  - `_map_input.filters.fishIds`
  - `_map_input.filters.fishFilterTerms`
  - `_map_input.filters.searchText`
  - `_map_input.filters.fromPatchId`
  - `_map_input.filters.toPatchId`
  - `_map_input.filters.layerIdsVisible`
- added a regression in `site/assets/js/pages/map-page.test.mjs` proving that stored page UI
  state does not overwrite query-owned map input fields during restore
- fixed loader startup ordering in `site/assets/map/loader.js` so signal-to-bridge `_map_input`
  sync stays disabled until after the bridge has mounted and the initial bridge state has been
  pulled back into Datastar state
- fixed a re-entrant Datastar patch loop in `syncPatchRangeFromSignals()` by guarding canonical
  patch-range rewrites while they are being applied

Why this matters:

- after moving more map filter state into page-owned Datastar storage, startup precedence became
  wrong in two ways:
  - stored page UI state could beat query-string state on first load
  - loader-side canonicalization of patch ranges could recursively patch `_map_input` and blow
    the Datastar stack
- the new model is:
  - query string owns query-owned startup state
  - page-owned Datastar storage fills only the remaining non-query-owned UI state
  - signal-to-bridge sync only starts after the bridge has published authoritative input state

Validation:

- `node --check site/assets/map/loader.js`
- `node --test site/assets/js/pages/map-page.test.mjs site/assets/map/map-host.test.mjs`
- rebuilt site output
- compared served vs `.out` for:
  - `/map/`
  - `/js/pages/map-page.js`
  - `/map/loader.js`
- live Chromium smoke:
  - seed `fishystuff.map.window_ui.v1` with conflicting stored filter/ui values
  - navigate to `/map/?fish=91&fishTerms=missing&search=url-search&fromPatch=2023-04-06-trade-price-x2&toPatch=2025-04-30-tidal-draughts&layers=zones,terrain&diagnostics=true&legend=true`
  - verify `_map_input` and `FishyMapBridge.getCurrentInputState()` both keep the query-owned:
    - `fishIds`
    - `fishFilterTerms`
    - `searchText`
    - `fromPatchId`
    - `toPatchId`
    - `layerIdsVisible`
    - `diagnosticsOpen`
    - `legendOpen`
  - verify no `Maximum call stack size exceeded` errors remain during startup

### Step 37 - Remove Dead Bridge-Owned Map Prefs Persistence

Completed:

- removed the obsolete bridge-owned `prefs` storage path from `site/assets/map/map-host.js`
  - removed `FISHYMAP_STORAGE_KEYS.prefs`
  - removed local-prefs restore from `buildInitialRestorePatch(...)`
  - removed `createPrefsSnapshot()`
  - removed `saveLocalPrefsNow()`
  - removed local-prefs writes on signal changes and pagehide/visibilitychange
- removed the last loader-side cleanup reference to the old prefs key in
  `site/assets/map/loader.js`
- updated map bridge tests in `site/assets/map/map-host.test.mjs` to assert the new
  ownership model:
  - bridge session snapshots still exist
  - bridge prefs snapshots no longer exist
  - URL restore precedence is now only against bridge session state
- added page-level cleanup in `site/assets/js/pages/map-page.js` so stale
  `fishystuff.map.prefs.v1` is deleted on map restore
- added regression coverage in `site/assets/js/pages/map-page.test.mjs` for clearing the
  legacy prefs key

Why this matters:

- after previous slices, the bridge-owned prefs snapshot had become empty dead weight
- leaving it around still created a misleading second persistence owner:
  - page-owned Datastar state for durable visible map UI
  - bridge-owned localStorage prefs that no longer restored anything useful
- removing it simplifies the model:
  - page-owned Datastar storage owns durable map UI/filter persistence
  - bridge-owned session storage only owns runtime/session restore state that has not been
    migrated yet

Validation:

- `node --check site/assets/js/pages/map-page.js`
- `node --check site/assets/map/map-host.js`
- `node --check site/assets/map/loader.js`
- `node --test site/assets/js/pages/map-page.test.mjs site/assets/map/map-host.test.mjs`
- rebuilt site output
- compared served vs `.out` for:
  - `/js/pages/map-page.js`
  - `/map/map-host.js`
- live Chromium smoke:
  - seed `localStorage['fishystuff.map.prefs.v1']`
  - reload `/map/`
  - verify the map boots cleanly
  - verify `localStorage['fishystuff.map.prefs.v1'] === null`
  - verify query-owned `_map_input` state still survives startup

### Step 38 - Move Map Session Restore/Persistence into Page-Owned Datastar State

Completed:

- added a page-owned `_map_session` Datastar branch in `site/layouts/map.shtml`
  for durable map session state:
  - `view`
  - `selection`
- extended `site/assets/js/pages/map-page.js` so map-page restore/persist now also owns:
  - `sessionStorage['fishystuff.map.session.v1']`
  - restore into `_map_session`
  - persistence from `_map_session`
- added page-level session restore/persist regression coverage in
  `site/assets/js/pages/map-page.test.mjs`
- removed bridge-owned session storage restore/write behavior from
  `site/assets/map/map-host.js`
  - `buildInitialRestorePatch(...)` is now query/default only
  - session persistence hooks/timers are gone
  - bridge session snapshots remain available only as pure derived helpers
- added `createSessionSnapshotFromState(...)` in `site/assets/map/map-host.js`
  so the page/loader can reuse the same compact session snapshot shape
- updated `site/assets/map/loader.js` so Datastar-owned `_map_session` now drives bridge
  startup/session reconciliation:
  - build bridge initial restore patch from `_map_session`
  - wait for `window.__fishystuffMap.restore($)` before mounting the bridge
  - reconcile restored `_map_session` into the bridge before allowing runtime session
    mirroring to overwrite it
  - mirror runtime `view` / `selection` back into `_map_session` once the bridge catches up
- simplified the `viewChanged` path so it goes back through `renderCurrentState(...)`,
  keeping runtime/session signal publishing consistent
- updated `site/assets/map/map-host.test.mjs` to assert the new ownership model:
  - build-initial-restore ignores session storage entirely
  - page-owned session restore is no longer bridge-owned

Why this matters:

- before this slice, session restore was double-owned:
  - page-owned `_map_session` had started persisting
  - bridge-owned `sessionStorage` restore/save still existed
- that created a broken startup ordering:
  - restored Datastar session could be overwritten by stale bridge runtime state
  - the map could boot in the wrong mode/selection even with valid stored session data
- the new model is:
  - page-owned Datastar `_map_session` is the single durable owner
  - loader adapts `_map_session` into the existing bridge/WASM contract
  - bridge publishes runtime state back into `_map_session` only after it has reconciled
    to the desired restored session

Validation:

- `node --check site/assets/js/pages/map-page.js`
- `node --check site/assets/map/loader.js`
- `node --check site/assets/map/map-host.js`
- `node --test site/assets/js/pages/map-page.test.mjs site/assets/map/map-host.test.mjs`
- rebuilt site output
- compared served vs `.out` for:
  - `/map/`
  - `/js/pages/map-page.js`
  - `/map/loader.js`
  - `/map/map-host.js`
- live Chromium smoke:
  - seed `sessionStorage['fishystuff.map.session.v1']` with:
    - 3D view
    - camera values
    - bookmark/world-point selection
    - fish id
  - reload `/map/`
  - verify:
    - `_map_session` still matches the stored session
    - `FishyMapBridge.getCurrentState().view.viewMode === '3d'`
    - `FishyMapBridge.getCurrentState().selection` reflects the stored world point
    - `FishyMapBridge.getCurrentInputState().filters.fishIds` includes the stored fish id

### Step 39 - Route Map Reset UI Through Datastar Actions

Completed:

- expanded `_map_actions` in `site/layouts/map.shtml` with `resetUiToken`
- moved the `Reset UI` button to a Datastar action-token update in the template instead of a
  loader-owned direct click listener
- updated `site/assets/map/loader.js` so `syncMapActionsFromSignals()` now owns both:
  - `resetViewToken`
  - `resetUiToken`
- removed the last direct `elements.resetUi.addEventListener("click", ...)` path from loader

Why this matters:

- `Reset UI` was still bypassing the Datastar signal graph entirely
- after the session-ownership migration in Step 38, the remount/reset path is now better aligned
  with page-owned signal state, so routing the action through `_map_actions` removes another
  imperative DOM island without changing the underlying reset implementation

Validation:

- `node --check site/assets/map/loader.js`
- rebuilt site output
- compared served vs `.out` for:
  - `/map/`
  - `/map/loader.js`

Note:

- this slice has not yet been re-run through a live browser reset/remount smoke after moving the
  click path into `_map_actions`; the existing raw-CDP helper needs local socket access that is
  blocked in the current sandbox

### Step 40 - Make Loader Read Page-Owned Map UI State Directly

Completed:

- removed the old loader-owned live mirrors for page-owned map UI branches:
  - `windowUiState`
  - `searchUiState`
  - `bookmarks`
  - `bookmarkUi`
  - `mapActionState`
- replaced those mirrors in `site/assets/map/loader.js` with direct Datastar-backed reads:
  - `currentUiState()`
  - `currentWindowUiState()`
  - `currentSearchUiState()`
  - `currentBookmarks()`
  - `currentBookmarkUiState()`
- converted the remaining broad read sites to treat those branches as live signal-backed state
  instead of local loader-owned truth:
  - render paths
  - bookmark bridge sync
  - managed-window visibility/position logic
  - search open/close logic
  - reset-action handling
- kept only minimal previous-snapshot bookkeeping for transition side effects:
  - newly opened windows still come to front
  - search still blurs when the window closes
  - closing the bookmark window still clears placement mode
- renamed the old signal reconciliation hook from a local-state sync model to
  `reconcileUiStateFromSignals()`

Why this matters:

- until this slice, the page-owned Datastar state existed, but loader still behaved as if it
  owned the active UI state and merely synchronized it
- that kept a large feedback-loop surface alive and made the map harder to reason about:
  - two live copies of the same UI state
  - reconciliation bugs
  - bridge publish loops
- the loader is now closer to its intended role:
  - read page-owned Datastar state
  - adapt it into the bridge/runtime
  - publish runtime output back into Datastar

Validation:

- `node --check site/assets/map/loader.js`
- `node --test site/assets/js/pages/map-page.test.mjs site/assets/map/loader.test.mjs site/assets/map/map-host.test.mjs`
- rebuilt site output
- compared served vs `.out` for:
  - `/map/`
  - `/map/loader.js`
  - `/js/pages/map-page.js`
- live browser smoke:
  - `bash tools/scripts/map-browser-smoke.sh /tmp/map-browser.json`
  - bridge reached ready cleanly with the current dev server stack after the direct-read change

### Step 41 - Route Fishydex Import/Export and Modal Close Through Datastar Actions

Completed:

- added a local `_fishydex_actions` branch in `site/content/en-US/dex.smd`:
  - `exportCaughtToken`
  - `importCaughtToken`
  - `closeDetailsToken`
- moved the Fishydex action buttons in the template off direct `window.Fishydex.*(...)` calls:
  - `Export`
  - `Import`
  - details close button
  - details backdrop
- updated `site/assets/js/pages/fishydex.js` so `sync($)` consumes those action tokens and
  triggers the side effects from page state instead of from imperative template calls
- removed these no-longer-needed globals from `window.Fishydex`:
  - `exportCaught`
  - `importCaught`
  - `openDetails`
  - `closeDetails`
- kept the rendered-card click handling intact for now, so the grid can still open details and
  toggle caught/favourite state without reworking the whole card render path in the same slice
- added page-level regression coverage in `site/assets/js/pages/fishydex.test.mjs` for:
  - export token -> clipboard copy + status message
  - import token -> caught id patch + status message

Why this matters:

- Fishydex still had several template-level direct calls into `window.Fishydex`, which meant
  those user actions bypassed the Datastar signal graph
- moving them onto local action tokens keeps the same user behavior while making the page
  contract more consistent:
  - template mutates signals
  - page module reacts
  - side effects patch signals back
- this also shrinks the required global API surface for Fishydex and makes later refactors
  around details/modal ownership easier

Validation:

- `node --check site/assets/js/pages/fishydex.js`
- `node --test site/assets/js/pages/fishydex.test.mjs`
- rebuilt site output
- compared served vs `.out` for:
  - `/dex/`
  - `/js/pages/fishydex.js`

### Step 42 - Clear Fishydex Feedback From Signal Patches

Completed:

- added a page-level feedback-clear listener in `site/assets/js/pages/fishydex.js`
  that reacts to user-owned Fishydex signal patches for:
  - search
  - caught/missing/favourite filter
  - grade filters
  - method filters
  - dried toggle
  - sort field/direction
- the listener now clears these transient feedback signals centrally:
  - `_status_message`
  - `_api_error_message`
  - `_api_error_hint`
- removed the repeated inline feedback-reset glue from `site/content/en-US/dex.smd` across:
  - search input
  - status filter chips
  - grade filter chips
  - method filter chips
  - clear-filters button
  - sort buttons
- added page-level regression coverage in `site/assets/js/pages/fishydex.test.mjs`
  to prove a matching signal patch clears transient feedback

Why this matters:

- Fishydex still had many template expressions mutating the same transient feedback signals over
  and over again
- that was repetitive, easy to miss when adding new controls, and kept feedback lifecycle logic
  scattered through the template instead of in the page state model
- the page now owns the rule:
  - user changes durable filter/sort state
  - Fishydex clears transient status/error feedback

Validation:

- `node --check site/assets/js/pages/fishydex.js`
- `node --test site/assets/js/pages/fishydex.test.mjs`
- rebuilt site output
- compared served vs `.out` for:
  - `/dex/`
  - `/js/pages/fishydex.js`

### Step 43 - Move Fishydex Filter Toggles Into Shared Datastar Helpers

Completed:

- added a reusable `toggleOrderedValue(...)` helper to
  `site/assets/js/datastar-state.js`
- added shared regression coverage in `site/assets/js/datastar-state.test.mjs`
  for deterministic ordered toggling
- updated `site/content/en-US/dex.smd` so Fishydex grade/method filter chips now use:
  - `window.__fishystuffDatastarState.toggleOrderedValue(...)`
  instead of page-specific `window.Fishydex` helpers
- removed the now-unused page-specific pure helpers from
  `site/assets/js/pages/fishydex.js`:
  - `toggleGradeFilters`
  - `toggleMethodFilters`
- trimmed the `window.Fishydex` global surface further so it only exposes the
  remaining page-level entry points still needed by the template

Why this matters:

- these filter toggles were pure ordered-array state transforms, not Fishydex-specific
  side effects
- keeping them on `window.Fishydex` made the page template depend on a larger global API
  than necessary
- moving them into the shared Datastar state helper:
  - reduces Fishydex-specific glue
  - gives other pages/components a reusable ordered-toggle primitive
  - keeps the remaining `window.Fishydex` surface focused on actual page orchestration

Validation:

- `node --check site/assets/js/datastar-state.js`
- `node --check site/assets/js/pages/fishydex.js`
- `node --test site/assets/js/datastar-state.test.mjs site/assets/js/pages/fishydex.test.mjs`
- rebuilt site output
- compared served vs `.out` for:
  - `/dex/`
  - `/js/pages/fishydex.js`
  - `/js/datastar-state.js`

### Step 44 - Route Calculator Toolbar Actions Through Datastar Signals

Completed:

- added a local `_calculator_actions` branch in `site/content/en-US/calculator.smd`:
  - `copyUrlToken`
  - `copyShareToken`
  - `clearToken`
- extended `window.__fishystuffCalculator` in the calculator page script with:
  - `actionState(...)`
  - `syncActions(...)`
  - token bookkeeping for handled toolbar actions
- added a hidden reactive hook in the server-rendered calculator fragment so toolbar actions now
  flow through the Datastar signal graph:
  - `data-effect="window.__fishystuffCalculator.syncActions($)"`
- moved these buttons in `api/fishystuff_server/src/routes/calculator.rs` off direct
  imperative helper/toast calls and onto token updates:
  - `Copy URL`
  - `Copy Share`
  - `Clear`
- kept the existing user-visible behavior:
  - copy URL still copies the preset URL
  - copy share still copies share text
  - clear still restores defaults and shows the toast

Why this matters:

- the calculator still had a small but obvious imperative island in the toolbar
- this keeps the same UX while aligning the toolbar with the same Datastar action-token model now
  used in Fishydex and the map page
- it also reduces the amount of direct template glue that a later calculator page-module
  extraction will need to preserve

Validation:

- rebuilt site output
- compared served vs `.out` for:
  - `/calculator/`
- `cargo test --offline -p fishystuff_server routes::calculator::tests::init_returns_html_fragment_with_initial_signals -- --exact`
- extended that init-fragment test to assert the new calculator action-token wiring is present

## Step 45: Extract calculator page helper into a real page module

What changed:

- moved the large inline `window.__fishystuffCalculator` helper out of
  `site/content/en-US/calculator.smd` into:
  - `site/assets/js/pages/calculator-page.js`
- switched the calculator page to load that module with:
  - `<script src="/js/pages/calculator-page.js"></script>`
- added focused VM coverage in:
  - `site/assets/js/pages/calculator-page.test.mjs`
- added the new page asset to the static site manifest in:
  - `site/zine.ziggy`

Why this matters:

- the calculator template had become one of the largest remaining inline imperative
  Datastar glue islands on the site
- moving it into a real page asset makes it testable, shareable, and much easier to
  continue refactoring toward a page-owned signal model
- it also removes a template/runtime split-brain risk where the inline helper could drift
  away from the shared Datastar helpers already extracted elsewhere

Important details:

- the public `window.__fishystuffCalculator` contract was kept intact so the existing
  server-rendered Datastar markup continues to work unchanged
- the extracted module uses the shared `window.__fishystuffDatastarState.createSignalStore()`
  helper when present, with a tiny internal fallback for test/runtime safety
- while extracting, the toolbar action-token handler was tightened so actions only fire
  on token increments, not any token change; this avoids duplicate copy/clear handling
  after resets rewrite token state

Validation:

- `node --check site/assets/js/pages/calculator-page.js`
- `node --test site/assets/js/pages/calculator-page.test.mjs`
- `cargo test --offline -p fishystuff_server routes::calculator::tests::init_returns_html_fragment_with_initial_signals -- --exact`
- rebuilt site output
- compared served vs `.out` for:
  - `/calculator/`
  - `/js/pages/calculator-page.js`
- confirmed the served calculator HTML now references the external page module and no longer
  embeds the old inline `window.__fishystuffCalculator = ...` helper

## Step 46: Move calculator client-only Datastar listeners into the page module

What changed:

- moved calculator persistence ownership fully into `site/assets/js/pages/calculator-page.js`
- moved `_calculator_actions` handling fully into `site/assets/js/pages/calculator-page.js`
- removed the server-rendered hidden client-only hooks from
  `api/fishystuff_server/src/routes/calculator.rs`:
  - hidden debounced persist node
  - hidden `data-effect="window.__fishystuffCalculator.syncActions($)"` node
- kept the server-rendered eval hook in place:
  - hidden debounced `@post(window.__fishystuffCalculator.evalUrl())`

Why this matters:

- persistence and action-token handling are page-local client concerns, not server-rendered UI
- Fishydex and the map page already moved to page-owned Datastar signal-patch listeners
- this keeps the calculator aligned with that same pattern and reduces hidden template glue

Implementation notes:

- the page module now binds:
  - a debounced persist listener via `window.__fishystuffDatastarPersist`
  - an action listener that reacts only to `_calculator_actions` patches
- restore now binds those listeners before hydrating stored state, and only enables them after
  restore completes to avoid startup churn
- the action handler now treats tokens as monotonic counters and fires only on increments
  instead of any token change

Validation:

- `node --check site/assets/js/pages/calculator-page.js`
- `node --test site/assets/js/pages/calculator-page.test.mjs`
- `cargo test --offline -p fishystuff_server routes::calculator::tests::init_returns_html_fragment_with_initial_signals -- --exact`
- rebuilt site output
- compared served vs `.out` for:
  - `/calculator/`
  - `/js/pages/calculator-page.js`
- confirmed the served calculator HTML no longer contains the hidden client-only
  persist/action hooks, while the page module still contains the corresponding listener logic

## Step 47: Share page signal-store bootstrapping across Datastar page modules

What changed:

- added `createPageSignalStore()` to `site/assets/js/datastar-state.js`
- switched all Datastar page modules to use that shared helper:
  - `site/assets/js/pages/calculator-page.js`
  - `site/assets/js/pages/fishydex.js`
  - `site/assets/js/pages/map-page.js`
- removed the duplicated per-page fallback signal-store bootstrap implementations
- extended `site/assets/js/datastar-state.test.mjs` to cover the new page-store helper
- updated the Fishydex and map-page VM harnesses to load `datastar-state.js` explicitly,
  matching real page boot order

Why this matters:

- calculator, Fishydex, and map page were still each carrying their own copy of the same
  signal-store bootstrap code
- centralizing that bootstrap reduces drift and makes future changes to page-owned Datastar
  state behavior happen in one place
- it also makes the tests reflect the actual page dependency chain more faithfully

Validation:

- `node --check site/assets/js/datastar-state.js site/assets/js/pages/calculator-page.js site/assets/js/pages/fishydex.js site/assets/js/pages/map-page.js`
- `node --test site/assets/js/datastar-state.test.mjs site/assets/js/pages/calculator-page.test.mjs site/assets/js/pages/fishydex.test.mjs site/assets/js/pages/map-page.test.mjs`
- rebuilt site output
- compared served vs `.out` for:
  - `/js/datastar-state.js`
  - `/js/pages/calculator-page.js`
  - `/js/pages/fishydex.js`
  - `/js/pages/map-page.js`

## Step 48: Share monotonic Datastar action-token handling

What changed:

- added shared counter-token helpers to `site/assets/js/datastar-state.js`:
  - `normalizeCounterTokenState(...)`
  - `consumeIncrementedCounterTokens(...)`
- moved calculator action-token handling onto those helpers in:
  - `site/assets/js/pages/calculator-page.js`
- moved Fishydex action-token handling onto those helpers in:
  - `site/assets/js/pages/fishydex.js`
- extended `site/assets/js/datastar-state.test.mjs` to cover the new helper behavior

Why this matters:

- both calculator and Fishydex were still hand-rolling the same token normalization and
  delta-consumption logic
- the calculator already exposed why `token changed` is the wrong semantic; action tokens need
  to be treated as monotonic counters and should fire only on increments
- centralizing that logic reduces duplication and makes the intended Datastar action-token model
  explicit and testable

Scope note:

- the map loader still has its own action-token delta handling for now
- that path should be moved onto the shared helper in a later slice once the loader/runtime
  boundary is tackled directly

Validation:

- `node --check site/assets/js/datastar-state.js site/assets/js/pages/calculator-page.js site/assets/js/pages/fishydex.js`
- `node --test site/assets/js/datastar-state.test.mjs site/assets/js/pages/calculator-page.test.mjs site/assets/js/pages/fishydex.test.mjs`
- rebuilt site output
- compared served vs `.out` for:
  - `/js/datastar-state.js`
  - `/js/pages/calculator-page.js`
  - `/js/pages/fishydex.js`

## Step 49: Share monotonic Datastar action-token handling with the map loader

What changed:

- added a small `datastarStateHelper()` bridge in `site/assets/map/loader.js`
- moved the map loader's `_map_actions` normalization onto the shared helper:
  - `normalizeCounterTokenState(...)`
- moved map action-token consumption onto the shared helper:
  - `consumeIncrementedCounterTokens(...)`
- `syncMapActionsFromSignals()` now consumes:
  - `resetViewToken`
  - `resetUiToken`
  using the same monotonic counter semantics already shared by calculator and Fishydex

Why this matters:

- the map loader still had the last bespoke action-token implementation on the site
- centralizing that logic keeps the Datastar action model consistent across:
  - calculator
  - Fishydex
  - map
- it also fixes a subtle behavioral edge:
  - the old map path returned early after handling `resetViewToken`
  - if both tokens incremented in the same patch, `resetUiToken` could be skipped

Validation:

- `node --check site/assets/map/loader.js`
- `node --test site/assets/js/datastar-state.test.mjs site/assets/map/loader.test.mjs site/assets/map/map-host.test.mjs`
- rebuilt site output
- compared served vs `.out` for:
  - `/map/loader.js`
- `bash tools/scripts/map-browser-smoke.sh /tmp/map-smoke-datastar-frp.json`

## Step 50: Remove stale map reset storage ownership from the loader

What changed:

- removed the last direct `fishystuff.map.window_ui.v1` storage mutation from
  `site/assets/map/loader.js`
- `Reset UI` now relies on the page-owned Datastar persistence flow in
  `site/assets/js/pages/map-page.js` instead of trying to clear localStorage itself

Why this matters:

- map UI persistence is already owned by the page-level Datastar module
- letting the loader clear storage directly created a second owner for the same persisted
  state
- this keeps reset behavior aligned with the FRP model:
  - patch signal state to defaults
  - let the debounced persistence layer write that state

Validation:

- `node --check site/assets/map/loader.js`
- `node --test site/assets/js/datastar-state.test.mjs site/assets/map/loader.test.mjs site/assets/map/map-host.test.mjs`
- rebuilt site output
- compared served vs `.out` for:
  - `/map/loader.js`

## Step 51: Persist map layer-settings expansion through Datastar UI state

What changed:

- moved layer-settings expansion out of the loader-local `Set` in `site/assets/map/loader.js`
- added durable page-owned Datastar state under:
  - `_map_ui.layers.expandedLayerIds`
- wired loader rendering, reconciliation, and `Reset UI` to read/write that signal-owned
  layer expansion state
- extended `site/assets/js/pages/map-page.js` persistence/restore shaping so
  `fishystuff.map.window_ui.v1` now includes:
  - `layers.expandedLayerIds`
- added VM coverage in `site/assets/js/pages/map-page.test.mjs`

Why this matters:

- expanded layer settings are durable user-facing UI state, not loader-internal runtime state
- persisting them through the page-owned Datastar graph matches the repo-wide UI-state policy:
  - durable view choices persist
  - ephemeral interaction state does not
- it also removes another local-first state pocket from the loader

Validation:

- `node --check site/assets/map/loader.js site/assets/js/pages/map-page.js site/assets/js/pages/map-page.test.mjs`
- `node --test site/assets/js/datastar-state.test.mjs site/assets/js/pages/map-page.test.mjs site/assets/map/loader.test.mjs site/assets/map/map-host.test.mjs`
- rebuilt site output
- compared served vs `.out` for:
  - `/map/loader.js`
  - `/js/pages/map-page.js`
- `bash tools/scripts/map-browser-smoke.sh /tmp/map-smoke-layer-ui.json`

## Step 52: Let the page own shared fish filter state for the map bridge

What changed:

- moved the map bridge boundary toward page-owned state for favourite/missing fish filters
- `site/assets/map/loader.js` now injects the current shared fish state into the effective
  map input state before bridge synchronization
- `site/assets/map/map-host.js` now accepts `ui.sharedFishState` in the input-state contract
  and uses it to resolve fish filter terms
- storage reads remain only as a fallback when page-provided shared fish state is absent
- added host coverage in `site/assets/map/map-host.test.mjs` for:
  - shared fish state normalization
  - page-provided shared fish state overriding storage fallback for fish filter resolution

Why this matters:

- previously `map-host.js` still owned the shared-fish storage read itself
- that made the bridge responsible for state the page had already derived
- this change makes the page the preferred owner and leaves bridge storage access as a legacy
  compatibility fallback instead of the primary source of truth

Validation:

- `node --check site/assets/map/loader.js site/assets/map/map-host.js site/assets/map/map-host.test.mjs`
- `node --test site/assets/js/datastar-state.test.mjs site/assets/map/loader.test.mjs site/assets/map/map-host.test.mjs`
- rebuilt site output
- compared served vs `.out` for:
  - `/map/loader.js`
  - `/map/map-host.js`
- `bash tools/scripts/map-browser-smoke.sh /tmp/map-smoke-shared-fish-state.json`

## Step 53: Restore shared fish filter state into Datastar signals on the map page

What changed:

- `site/assets/js/pages/map-page.js` now restores shared fish state into a dedicated
  Datastar branch:
  - `_shared_fish.caughtIds`
  - `_shared_fish.favouriteIds`
- the restore path prefers the shared helper from `site/assets/js/shared-fish-state.js`
  and falls back to the legacy Fishydex storage keys only when that helper is unavailable
- `site/assets/map/loader.js` now prefers `_shared_fish` from the Datastar signal graph
  before falling back to storage reads
- added restore coverage in `site/assets/js/pages/map-page.test.mjs`

Why this matters:

- Step 52 made the page the preferred owner of shared fish filter state at the bridge boundary,
  but restore still depended on storage being read ad hoc at runtime
- this slice completes that handoff by materializing shared fish state inside the page-owned
  Datastar graph during restore
- the loader can now treat `_shared_fish` as the canonical shared-fish input source for the map
  page instead of reaching for storage first

Validation:

- `node --check site/assets/js/pages/map-page.js site/assets/js/pages/map-page.test.mjs site/assets/map/loader.js`
- `node --test site/assets/js/datastar-state.test.mjs site/assets/js/pages/map-page.test.mjs site/assets/map/loader.test.mjs site/assets/map/map-host.test.mjs`
- rebuilt site output
- compared served vs `.out` for:
  - `/js/pages/map-page.js`
  - `/map/loader.js`
- `bash tools/scripts/map-browser-smoke.sh /tmp/map-smoke-shared-fish-signals.json`
