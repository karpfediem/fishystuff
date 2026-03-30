# Datastar FRP Refactor Worklog

Date: 2026-03-30
Repo: `/home/carp/code/fishystuff`
Focus: Calculator Datastar state flow

## Goal

Refactor the calculator to follow Datastar's functional reactive model instead of relying on imperative DOM/event plumbing.

Primary user-visible bug driving this:

- removing food or buff selections does not reliably clear the effective calculator state
- AFR remains affected after removing a selected food
- refreshing the page can bring removed selections back

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

## Current Calculator Anti-Patterns

Observed before refactor:

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

- partially investigated
- not solved yet

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

## Current Evidence

Live browser probe revealed duplicated canonical food state before the latest server-side patch:

- `food: ["item:9359", "item:9359", "", ...]`

That explains why removing one visible selection did not clear AFR:

- one duplicate remained active in the actual signal state
- persistence then wrote the wrong state back to localStorage

This confirms the bug is structural, not just a last-item edge case.

## Current Working Changes

Uncommitted calculator-related changes currently in progress:

- `api/fishystuff_server/src/routes/calculator.rs`
- `site/content/en-US/calculator.smd`

Unrelated local changes not part of this refactor:

- `site/assets/map/loader.js`
- `site/assets/map/loader.test.mjs`

## Next Move

Finish Step 2 cleanly:

- choose a single canonical representation for multiselect state
- keep transport local or remove it entirely
- verify live removal for food and buff before committing
