# RFC: Zone Profiles, Evidence Semantics, and Fishing Map Requirements

## Status

Draft.

This document replaces the previous work and defines the product semantics, guardrails, and capability requirements for fish zone assignment, fish presence support, and future quantitative fishing analysis.

This RFC is intentionally **implementation agnostic**. It defines what the system must mean and support. It does **not** require one specific spatial backend, storage model, rendering model, or inference algorithm.

---

## 1. Purpose

The project needs to answer several related but distinct questions:

1. **Which zone does a clicked point belong to?**
2. **Which fish are supported in that zone?**
3. **How strong and how fresh is that support?**
4. **Which parts of the map likely need boundary updates?**
5. **How can future player-tracked data support group inference and relative catch-rate analysis?**

Historically, the product mixed some of these concerns together. In particular, fish-level `%` values derived from ranking evidence were easy for readers to interpret as **drop rates**, even though they were actually a **stability / evidence-share signal** tied to sample assignment and effective sample size (ESS), not item probability.

This RFC formalizes the separation between:

* spatial assignment,
* fish presence support,
* ranking-evidence diagnostics,
* future player-tracked quantitative analysis,
* and boundary maintenance.

---

## 2. Scope

This RFC covers:

* core glossary and semantic definitions,
* user-facing capability requirements,
* source and evidence classes,
* public UX guardrails,
* required separations in the data/API/domain model,
* and future-proof requirements for integrating new evidence sources.

This RFC does **not** prescribe:

* a final storage schema,
* a final API shape,
* one permanent map representation,
* or one permanent implementation of zone lookup.

---

## 3. Non-Goals

This RFC does **not**:

* guarantee exact true item drop rates from current ranking evidence,
* require that current legacy fishing datasets are fully up to date,
* require a single blended confidence score,
* require a single global “truth source” for all fish presence questions,
* require immediate support for user accounts or authenticated user submissions,
* require a specific backend implementation such as bitmap masks, polygons, or a spatial index,
* require exact border-distance or ambiguity calculations in the first slice,
* or require one fish to belong to only one fish group in a zone.

---

## 4. Design Principles

### 4.1 Semantic separation before implementation

The system must first be correct in meaning. Backend and rendering choices are secondary.

### 4.2 Source provenance must be preserved

Different evidence sources answer different questions. They must not be flattened into one ambiguous score.

### 4.3 Time matters

All evidence should be time-attributed so that it can be aligned with patches, map revisions, and historical interpretation.

### 4.4 Unknown is valid

The system must be allowed to say:

* unknown,
* insufficient evidence,
* not yet modeled,
* or unavailable.

It must not fabricate precision.

### 4.5 Public UX must avoid misleading signals

The default public UI must not present ranking-derived fish-level `%` as if it were an item catch/drop rate.

### 4.6 Quantitative claims require setup-aware data

Future catch-rate analysis must only use controlled player-tracked data with setup metadata and explicit exclusions.

---

## 5. Glossary

## 5.1 Zone

A **zone** is a spatially defined region of world-space.

Fishing actions attributed to that region are interpreted through the zone-assigned fishing tables. In practice, a zone maps to an index into table-driven fishing groups and fish-group contents.

A zone may be represented internally as:

* a bitmap/mask,
* polygons,
* a spatial index,
* or another maintained structure.

This RFC only requires that the user-facing answer to “which zone is this point in?” is well-defined.

---

## 5.2 Fishing tables

Fishing is modeled as a two-stage process:

1. **Group roll**
   A fishing action lands in a fishing group/category.

2. **Item roll within that group**
   An item is chosen from the items available in that group.

Legacy resources and community references may still be useful for understanding or reconstructing this structure, but this RFC does not claim that any one legacy table snapshot is complete or current truth.

---

## 5.3 Bookmark

A **bookmark** is a world-coordinate point that can be imported/exported between the site and the game.

Bookmarks are practically useful as a user-facing location interface, but small rounding differences can occur when moving between systems.

---

## 5.4 Ranking evidence

**Ranking evidence** is fish-guide-derived observation data.

It is useful because it provides:

* positive evidence that a fish has appeared near a location,
* freshness and historical context,
* ranking-based support for fish presence,
* and a basis for assignment diagnostics such as ESS.

It is limited because:

* new samples become rarer over time,
* it is not a true catch-rate source,
* and sample attribution is approximate.

Ranking evidence is therefore a **support and diagnostics** source, not a universal quantitative truth source.

---

## 5.5 Float position

The game resolves the fishing action from a float position that is approximately **500 game units** away from the player position.

For some sample types, that means the actual catch attribution belongs to a **ring around the player position**, not the exact player coordinate.

This distinction is important for sample attribution.

---

## 5.6 Sample attribution model

A **sample attribution model** defines how a sample can be spatially assigned.

At minimum, the system should distinguish between:

* **FloatPosition**
  The catch is attributed to a known point or a small-radius neighborhood around a point.

* **PlayerPositionRing**
  The catch is attributed to a ring around the player position. Ranking evidence belongs here.

The system must not collapse these into one fake “exact point” model.

---

## 5.7 Effective sample size (ESS)

ESS is a measure used for diagnostics around sample assignment stability.

ESS may be useful to communicate:

* how stable a ranking-derived support signal is,
* whether evidence is sparse,
* or whether a result should be treated cautiously.

ESS is **not** a drop-rate and must not be presented as one.

---

## 6. Source and Evidence Classes

The system must keep source families explicit.

## 6.1 Legacy/reference sources

Examples:

* legacy fishing tables,
* older map interpretations,
* static community-maintained reconstructions.

These are useful as:

* historical structure,
* baseline hints,
* and scaffolding for current work.

They are not automatically current truth.

---

## 6.2 Curated community support

Examples:

* manually maintained fish-to-zone assignments,
* screenshot-backed claims,
* workbook-style support evidence,
* explicit contributor tips.

These are useful for:

* fish presence support,
* fast incremental updates,
* and zones/fish where no better evidence exists yet.

They do **not** justify exact rates by themselves.

---

## 6.3 Ranking evidence

Ranking evidence is:

* positive evidence that a fish appeared,
* ranking-derived support,
* freshness and historical context,
* a source for assignment diagnostics and ESS.

It is **not**:

* direct catch-rate truth,
* strong absence evidence,
* or a long-term complete picture for old fish.

---

## 6.4 Player-tracked evidence

Player-tracked evidence includes:

* manual tracking,
* exact or approximate float/player position,
* setup information,
* and eventually automated helper-assisted logs.

This source is the intended foundation for:

* relative catch-rate estimation,
* fish-group inference,
* and stronger long-term quantitative modeling.

---

## 6.5 Future user-submitted evidence

Future site-submitted evidence may include:

* “I found this fish at this bookmark”
* “I found this fish in this zone”
* manual upload of controlled logs

This RFC allows those paths, but does not require login or identity systems.

---

## 7. Core Semantic Separations

These concerns must remain separate.

## 7.1 Point-to-zone assignment

Question: **Which zone does this clicked point belong to?**

This is a spatial lookup problem.

It is not:

* a support-quality problem,
* a catch-rate problem,
* or a ranking-ESS problem.

---

## 7.2 Fish presence support

Question: **What support exists that this fish is in this zone?**

This is a provenance-aware evidence problem.

It is not:

* point assignment,
* boundary certainty,
* or a true drop-rate problem.

---

## 7.3 Ranking evidence quality

Question: **How stable, fresh, and substantial is the ranking-derived support?**

This is where ESS and ranking-derived evidence-share diagnostics belong.

It is not:

* a catch rate,
* or a universal confidence score over all source families.

---

## 7.4 Quantitative catch analysis

Question: **Under a consistent setup, what relative rates can be estimated from tracked catches?**

This is a player-tracked quantitative analysis problem.

It must remain separate from:

* ranking evidence,
* legacy support,
* and anecdotal screenshots.

---

## 7.5 Fish-group inference

Question: **Which items can co-occur in which fish groups at a zone?**

This is related to controlled player tracking, especially with Triple-Float co-catch evidence.

It is separate from ordinary item-rate displays.

---

## 7.6 Boundary QA / redraw maintenance

Question: **Where are the current map boundaries likely wrong or under stress?**

This is a maintainer-facing QA concept.

It is not:

* ESS,
* a fish drop rate,
* or a click-assignment percentage.

---

## 8. Problem 1: Mapping World Coordinates to a Zone

## 8.1 User stories

* As a user, I want to click a point on the map and determine which zone it belongs to.
* As a user, I want to visually understand where zones are so I can plan fishing trips.
* As a maintainer, I want to keep zone boundaries accurate across patches and redraws.

## 8.2 Requirement

Given a point in world-space or map-space, the system must return the assigned zone.

This answer must be authoritative for the current map revision being used.

## 8.3 Backend-agnostic approaches

This RFC does not require one representation.

Possible valid implementations include:

* a fishing zone mask bitmap,
* a manually maintained fishing table in Dolt,
* a separate table assigning names to zone IDs,
* manually redrawn masks,
* polygons,
* or a spatial index.

The implementation may evolve, but the semantic behavior must remain stable.

## 8.4 Border ambiguity

Border ambiguity is optional in the first slice.

If implemented, it must be presented as a **spatial diagnostic**:

* “this point is near a boundary”
* “assignment is close to another zone edge”

It must not be conflated with ESS or fish probability.

If not implemented, the product should return a plain assignment and avoid pretending to know more.

---

## 9. Problem 2: Determining Fish Presence

## 9.1 User stories

* As a user, I want to know which zones contain a fish I am targeting.
* As a user, I want to know which fish are supported in a zone.
* As a community contributor, I want to submit support evidence that a fish is present in a zone.

## 9.2 Requirement

The system must support a fish-presence view that is:

* source-aware,
* time-aware,
* and explicit about uncertainty.

## 9.3 Presence support states

The system should be able to distinguish at least these categories:

* **Observed recently**
* **Observed historically**
* **Reference-supported**
* **Curated-support only**
* **Weak hint**
* **Insufficient evidence**
* **Unknown**

The exact final labels may vary, but the distinction between strong, weak, historical, and unknown support must remain explicit.

## 9.4 Critical guardrail

**Missing evidence is not evidence of absence.**

Lack of ranking data or lack of curated support must not be interpreted as “this fish is not in the zone.”

---

## 10. Problem 3: Determining Catch Rates and Group Assignments

This section must be explicitly split in two.

## 10.1 Relative catch-rate estimation

This is the future quantitative feature for controlled player tracking.

### Requirement

Relative catch-rate analysis must be **setup-scoped**.

That means samples must be grouped or filtered by effective setup, including any modifiers that materially affect group or item outcomes.

Examples include:

* mastery-related effects,
* prize/rare/group modifiers,
* and other relevant fishing bonuses.

### Guardrails

The system must not:

* pool materially different setups into one “catch-rate” result,
* claim universal rates when only setup-relative rates are known,
* or derive quantitative rates from ranking evidence alone.

## 10.2 Fish-group assignment inference

This is the future feature for inferring which items belong to which fish groups.

### Primary discussed method

The Triple-Float rod can catch multiple fish from a single cast, with the important observation that the co-caught items belong to the same group.

This makes co-catch data useful for inferring fish-group membership.

### Requirement

The system must support a model where:

* items may co-occur in the same group,
* one fish may belong to one or multiple candidate groups if supported by evidence,
* and this inference remains separate from rate estimation.

## 10.3 Exclusions

Certain event-only or event-tainted items should be excluded by default from:

* relative catch-rate analysis,
* and fish-group inference.

---

## 11. Problem 4: Boundary Stress and Redraw QA

## 11.1 User story

As a maintainer, I want to understand where the current zone borders are likely wrong or under stress, so I can prioritize redraw work.

## 11.2 Requirement

The system should support a maintainers’ QA view that identifies:

* areas with conflicting evidence across neighboring zones,
* likely problematic boundaries,
* and zones whose current borders deserve review.

## 11.3 Guardrail

Boundary stress is a **maintenance signal**, not a public fish-rate statistic.

It must not be confused with:

* ESS,
* click assignment itself,
* or fish probability.

---

## 12. Public UX Requirements

## 12.1 Default public view

The default public zone/fish UI must **not** display an exact fish-level `%` as the primary signal.

Instead, the default should emphasize:

* fish presence support state,
* source badges,
* evidence freshness,
* and explicit unknown / insufficient-evidence messaging.

## 12.2 Advanced diagnostics

Advanced or expert-facing UI may show:

* ranking evidence share,
* ESS,
* freshness,
* drift,
* confidence intervals,
* and similar diagnostics.

But these must be:

* clearly labeled as **ranking-only diagnostics**,
* visually separated from catch-rate displays,
* and not framed as drop rates.

## 12.3 Explicit wording guardrail

Any retained percent-like signal derived from ranking evidence must use wording that cannot reasonably be mistaken for an item drop rate.

For example:

* “ranking evidence share”
* “ranking-derived support share”
* “assignment stability”
* or similar

and not simply “% chance” or “rate”.

---

## 13. Data / API / Domain Model Requirements

The system should preserve the following conceptual sections, whether in one endpoint or several:

* `assignment`
* `presence_support`
* `ranking_evidence`
* `catch_rates`
* `diagnostics`

## 13.1 Assignment

Must represent:

* the assigned zone,
* the point that was queried,
* and optional border-analysis diagnostics if available.

## 13.2 Presence support

Must represent:

* which fish are supported,
* support state per fish,
* which source families support that claim,
* optional time/freshness context,
* and explicit uncertainty states.

## 13.3 Ranking evidence

Must represent ranking-only diagnostics such as:

* ESS,
* weight,
* freshness,
* drift,
* and optional fish-level evidence-share metrics.

These must not be the default public fish-rate display.

## 13.4 Catch rates

Must represent player-tracked quantitative analysis only.

If unavailable, the product should say so explicitly.

## 13.5 Diagnostics

Must represent:

* public-state warnings,
* insufficient-evidence states,
* optional boundary stress,
* and explanatory notes.

---

## 14. Architectural Guardrails

## 14.1 Spatial backend is replaceable

The project may use bitmaps today and something else tomorrow. The product semantics must not depend on one backend.

## 14.2 Evidence provenance must remain visible

Different sources must not be merged into one undifferentiated score.

## 14.3 Time attribution is required

All meaningful evidence should be time-attributed so it can be aligned with:

* patches,
* map revisions,
* and historical analysis.

## 14.4 Sample attribution geometry must be explicit

The system must support distinct sample attribution models such as:

* `FloatPosition`
* `PlayerPositionRing`

and not pretend they are all exact points.

## 14.5 Click assignment and sample evidence are distinct

The clicked point belonging to a zone is not the same thing as a ranking sample being exactly attributable to that zone.

## 14.6 Unavailable is acceptable

If a capability is not yet modeled or not supported by current evidence, the system should return “unavailable”, “unknown”, or “insufficient evidence”, rather than a fabricated number.

---

## 15. Migration Plan

## Phase 1 — Immediate public correction

* Remove exact fish-level `%` from the default public zone panel.
* Keep fish presence support and source/freshness visible.
* If needed, move ranking evidence share into an advanced diagnostics section with explicit labeling.

## Phase 2 — Semantic hardening

* Preserve or introduce explicit separation between assignment, presence support, ranking evidence, catch rates, and diagnostics.
* Avoid collapsing these back into one blended response.

## Phase 3 — Spatial assignment refinement

* Continue improving clicked-point zone assignment.
* Optionally add border proximity / ambiguity diagnostics.

## Phase 4 — Sample attribution framework

* Add explicit sample attribution models.
* Support exact and ring-based evidence attribution.

## Phase 5 — Player-tracked quantitative pipeline

* Add controlled player-tracked data ingestion.
* Add setup-scoped relative rate analysis.
* Add Triple-Float-based group inference.

## Phase 6 — Boundary QA tooling

* Add maintainers’ views for stress and redraw prioritization.

---

## 16. Verified Facts vs Maintainer Assumptions

## 16.1 Verified or accepted project facts

* Fishing behavior is modeled as a two-stage group-and-item process.
* Bookmark transfer between site and game is useful but subject to small rounding differences.
* Ranking evidence becomes sparse over time.
* Ranking evidence should be time-attributed.
* Float/player-position geometry matters for evidence attribution.

## 16.2 Maintainer assumptions to preserve explicitly

These may be highly plausible and operationally useful, but they should still be treated as assumptions until specifically validated:

* rare/prize/group modifiers act at the group-selection stage,
* Triple-Float co-catches are same-group evidence,
* event-only or event-tainted items should be excluded by default from quantitative analysis,
* ring-based attribution for ranking evidence is the correct spatial model.

The product may use these assumptions, but it should avoid presenting them as stronger than they are.

---

## 17. Normative Requirements Summary

## Must

* The default public UI must not present ranking-derived exact `%` as the primary fish-level signal.
* The system must separate:

    * point assignment,
    * fish presence support,
    * ranking evidence quality,
    * catch-rate estimation,
    * fish-group inference,
    * and boundary stress.
* The system must preserve source provenance.
* The system must preserve time attribution for evidence.
* The system must not treat missing evidence as absence.
* The system must not present ranking evidence share as a drop/catch rate.
* The system must support setup-scoped quantitative analysis for future player tracking.
* The system must treat click assignment and sample attribution as separate concepts.
* The system must support explicit unavailable / unknown / insufficient-evidence states.

## Should

* The default public UI should emphasize fish support states, source badges, and freshness.
* Advanced diagnostics should remain available behind explicit ranking-only framing.
* Boundary stress should be available to maintainers for redraw QA.
* Fish-group inference should allow multiple candidate groups if evidence supports that.
* New implementation work should be modular by concern.

## May

* Border-distance / ambiguity may be added later.
* Advanced ESS / drift / confidence displays may be expanded in expert-facing tools.
* Additional evidence classes may be introduced later.

---

## 18. Practical Immediate Product Change

The immediate action implied by this RFC is simple:

> **Stop showing exact fish-level `%` in the default public zone view.**

If that signal remains anywhere, it must be:

* relabeled as a ranking-only support diagnostic,
* moved under an advanced section,
* and visually separated from anything that looks like a drop-rate display.
