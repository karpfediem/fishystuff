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

- implemented in current working tree
- not yet committed

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

- planned

### Step 4

Add browser regression coverage for multiselect removal.

Scenarios:

- remove only selected food
- remove one of multiple foods
- remove only selected buff
- reload and verify cleared state remains cleared

Status:

- planned

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

Current in-progress changes currently focus on:

- `api/fishystuff_server/src/routes/calculator.rs`
- `site/content/en-US/calculator.smd`
- `site/assets/js/components/searchable-multiselect.js`

Planned next audit targets after calculator state is stable:

- `site/content/en-US/dex.smd`
- `site/assets/js/pages/fishydex.js`
- Datastar patch-listener custom elements under `site/assets/js/components/`

Unrelated local changes not part of this refactor:

- `site/assets/map/loader.js`
- `site/assets/map/loader.test.mjs`

## Next Move

Finish calculator Step 3 cleanly:

- audit stored calculator signal canonicalization now that food/buff transport is compact
- decide whether outfit and pet skill checkbox transport should also move off slot arrays
- add browser regression coverage for food/buff removal and reload

Then expand the same analysis to the rest of the site:

- identify canonical backend-owned signals vs local UI signals
- remove imperative request/persistence plumbing where present
- align custom component boundaries with Datastar signal flow
- extract reusable components where the cleaned-up FRP model repeats
