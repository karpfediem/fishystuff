# RFC: Zone Profile V2

## Status

Draft.

This RFC covers:

- audit of the current `zone_stats` public surface
- verified semantics of the live server path
- comparison against the documented and standalone analytics semantics
- a proposed `zone_profile_v2` domain model and migration plan
- low-risk scaffolding direction only

This RFC does not replace `/api/v1/zone_stats` yet.

## Verification notes

Verified directly from code:

- `docs/problem-and-scope.md`
- `docs/zone-signatures.md`
- `docs/confidence-and-recency.md`
- `docs/ingestion-and-indexing.md`
- `docs/boundary-qa-and-updates.md`
- `docs/implementation-plan.md`
- `lib/fishystuff_api/src/models/zone_stats.rs`
- `lib/fishystuff_api/src/models/events.rs`
- `api/fishystuff_server/src/routes/zone_stats.rs`
- `api/fishystuff_server/src/app.rs`
- `site/assets/map/loader.js`
- `api/fishystuff_server/src/store/dolt_mysql.rs`
- `api/fishystuff_server/src/store/dolt_mysql/stats.rs`
- `api/fishystuff_server/src/store/queries/mod.rs`
- `lib/fishystuff_analytics/src/lib.rs`
- `lib/fishystuff_store/src/sqlite.rs`
- `lib/fishystuff_core/src/masks.rs`
- `api/sql/schema_fishing.sql`
- `api/sql/migrations/20260301_events_evidence_pipeline.sql`

Verified from local workbook sources via `devenv shell -- xlsx2csv`:

- `data/fishing_tables_101/Fishing Data.xlsx`
- `data/fishing_tables_101/BDO - PRIZE FISHES.xlsx`
- `data/fishing_tables_101/fishing_tables_101.md`
- `data/data/excel/Fishing_Table.xlsx`
- `data/data/excel/ItemMainGroup_Table.xlsx`
- `data/data/excel/ItemSubGroup_Table.xlsx`
- `data/data/excel/Item_Table.xlsx`

Verified from local runtime data via `devenv shell -- dolt sql`:

- `events`, `event_zone_assignment`, `fishing_table`, `fish_table`, `item_main_group_table`, `item_sub_group_table`, `fishing_zone_slots`, `item_main_group_options`, and `item_sub_group_item_variants` are present.
- `events.source_kind` currently has one live value only: `1`, with `26938` rows.
- `event_zone_assignment` has `26938` rows for `layer_revision_id='v1'`.
- `fishing_table` has `276` rows.
- `fish_table` has `300` rows.
- `item_main_group_table` has `405` rows.
- `item_sub_group_table` has `1676` rows.
- `item_main_group_options` has `469` rows.
- `item_sub_group_item_variants` has `1330` rows.

Verified from local workbook content:

- `Fishing Data.xlsx` contains a translated sheet named `Fishing Data Translated` with `158` RGB-keyed rows.
- That sheet includes columns `R,G,B,Harpoon ID,Prize Catch,Rare,Large,General,Treasure,Min Bite (s),Max Bite (s),Description`.
- `BDO - PRIZE FISHES.xlsx` contains a `DATA` sheet with structured per-zone fish rows and status-like remarks.
- The `DATA` sheet yielded `126` distinct zone references in quick local inspection.
- The same `DATA` sheet yielded `322` rows labeled `CONFIRMED`, `60` labeled `UNCONFIRMED`, and `53` labeled `DATA INCOMPLETE`.
- The workbook also contains dedicated tabs such as `CONFIRMED`, `UNCONFIRMED`, `INCOMPLETE`, `NEW PRIZE FISHES`, and per-fish tabs.

Not runtime-verified:

- the remote Google Sheet contents were not queried from the local environment
- no existing player-log table or log-ingestion pipeline was found in runtime tables or repo code

## 1) Current Public Surface Audit

### What the current product exposes

The live product exposes one zone-evidence response model, `ZoneStatsResponse`, via `POST /api/v1/zone_stats`.

Current API shape:

- zone identity: `zone_rgb_u32`, `zone_rgb`, `zone_name`
- echoed analysis window: `window`
- confidence block: `ess`, `total_weight`, `last_seen_ts_utc`, `age_days_last`, `status`, `notes`, optional `drift`
- fish distribution rows: `fish_id`, `item_id`, `encyclopedia_key`, `encyclopedia_id`, `fish_name`, `evidence_weight`, `p_mean`, `ci_low`, `ci_high`

Current route and app surface:

- `api/fishystuff_server/src/app.rs` mounts `POST /api/v1/zone_stats`
- `api/fishystuff_server/src/routes/zone_stats.rs` is a thin cache wrapper around the store call

Current client request defaults:

- the Bevy request builder sends `fish_norm: false`
- default parameters come from API meta defaults
- current UI behavior therefore reflects recency weighting plus the live server defaults, not the richer standalone analytics semantics

### Where the confusion surface actually is

The public confusion surface is not abstract. It is in the current UI:

- `site/assets/map/loader.js` shows `ESS`, `weight`, `last seen`, and `drift` in the zone evidence summary
- the same file renders each fish row with a percent badge from `entry.pMean`
- the tooltip labels that row as `p ... · weight ... · CI ...`

This means the current product presents a fish-level percent directly in the default map panel, even though the docs explicitly say the value is not a true in-game drop rate.

### Semantically risky public fields

Highest risk:

- `distribution[].p_mean`
  - technically a posterior mean evidence share
  - publicly rendered as a percent badge
  - easy for normal users to read as drop rate

Medium risk:

- `confidence.ess`
  - valid confidence proxy for weighting stability
  - easily misread as certainty that the clicked point is safely inside the zone

- `confidence.total_weight`
  - meaningful to power users
  - not self-explanatory in public UX

- `confidence.drift`
  - meaningful for patch-aware diagnostics
  - easy to confuse with border instability or map-assignment instability

### Authoritative code paths today

Authoritative live server path:

- request model: `lib/fishystuff_api/src/models/zone_stats.rs`
- route: `api/fishystuff_server/src/routes/zone_stats.rs`
- store entrypoint: `api/fishystuff_server/src/store/dolt_mysql.rs` `Store::zone_stats`
- live aggregation: `compute_zone_stats` and `compute_window_summary`
- event-loading SQL: `api/fishystuff_server/src/store/queries/mod.rs` `EVENTS_WITH_ZONE_SQL`

Standalone analytics comparison path:

- `lib/fishystuff_analytics/src/lib.rs`
- backing SQLite store query: `lib/fishystuff_store/src/sqlite.rs`

### Reusable parts

Reusable for `zone_profile_v2`:

- fish identity/name resolution from `fish_table`, `fish_names_*`, and `languagedata_en`
- zone metadata lookup from `zones_merged`
- ranking evidence freshness, ESS, and drift structures
- existing status thresholds in server config
- zone mask asset infrastructure and `ZoneMask` PNG loader
- boundary-QA terminology and JSD-based comparison concepts from existing docs

Not reusable as-is:

- `distribution[].p_mean` as a default public field
- the current single-layer response shape, which mixes assignment, support, confidence, and advanced ranking stats into one panel

## 2) Three-Way Semantics Comparison Table

| Dimension | Docs semantics | Live server semantics | Standalone analytics semantics |
| --- | --- | --- | --- |
| Primary input meaning | Ranking evidence distribution | Ranking evidence distribution | Ranking evidence distribution |
| Event weighting for displayed fish evidence | `w = w_time * w_eff * optional w_fish` | `w = w_time * optional w_fish` | `w = w_time * w_eff * optional w_fish` |
| Weight used for ESS | `u = w_time * w_eff` | `u = w_time` | `u = w_time * w_eff` |
| ESS formula | `ESS = W^2 / max(W2, eps)` | same formula | same formula |
| Effort debiasing active | documented as active | not active in live `zone_stats` | active |
| Per-fish normalization | optional | optional field exists, UI currently sends `false` | optional |
| Prior/posterior model | Dirichlet posterior over evidence share | implemented | implemented |
| Credible intervals | required / recommended | implemented via Monte Carlo Beta CI for top-K | implemented via Monte Carlo Beta CI for top-K |
| Drift behavior | patch-aware drift expected | implemented when `drift_boundary_ts_utc` is provided | implemented when `drift_boundary_ts` is provided |
| Source filtering behavior | ranking pipeline assumed | `zone_stats` SQL does not filter `events.source_kind` | SQLite analytics query also does not filter by source kind; assumes ranking-only store |
| Public percent rendering in current UI | docs say label as evidence share | yes, current UI renders `pMean` as a percent badge | not directly public; library only |
| Border ambiguity support | not part of zone signatures | none | none |
| Catch-rate support | explicitly not justified from ranking | not supported | not supported |

### Concrete mismatch summary

1. Docs vs live server:

- docs describe effort-debiased evidence and effort-aware ESS
- live server currently computes recency-only evidence and recency-only ESS

2. Live server vs standalone analytics:

- analytics uses the richer `w_eff` model and water-tile effort normalization
- live server does not

3. Docs/UI mismatch:

- docs explicitly warn that `p_hat` is not a drop probability
- current UI still renders the value as the primary percent in the default zone evidence list

4. Source-isolation mismatch:

- event snapshot queries explicitly filter `e.source_kind = ranking`
- live `zone_stats` SQL does not

## 3) Source-Family Audit

### Legacy fishing tables

Type:

- legacy reference

Concrete schema/runtime support:

- `fishing_table` provides zone RGB to slot-level group references and slot rates
- `fishing_zone_slots` flattens `DropRate1..5` and `DropID1..5`
- `item_main_group_options` flattens main-group to subgroup choices and option rates
- `fish_table` provides encyclopedia key to item key and icon/name identity joins
- the local workbook sheet `Fishing Data Translated` is broadly aligned with `fishing_table`, but not perfectly 1:1

What it can support:

- legacy zone-level support claims
- legacy zone-slot / group references
- item-level subgroup baselines where the local group tables are populated
- provenance that a zone was historically associated with certain group slots
- fish identity joins and icons

What it cannot reliably support today:

- current truth for newer or changed regions
- trustworthy freshness
- trustworthy subgroup item rates from the raw legacy XLSX files alone
- a blanket direct overwrite of `fishing_table` from auxiliary reference material

Important runtime finding:

- in the raw legacy import path, subgroup item expansion is structurally modeled but initially empty because the raw subgroup workbook does not carry usable `SelectRate_*` values
- the current local runtime now has subgroup baselines populated in the legacy group tables
- `item_sub_group_item_variants` now has `1330` rows across `260` subgroup keys
- `item_main_group_table` now has `405` rows and `item_main_group_options` now has `469` rows
- `fishing_table` remains at `276` rows
- in sample RGB checks, workbook `Harpoon ID` matched `fishing_table.DropIDHarpoon`
- workbook category percentages matched `fishing_zone_slots.slot_idx` `2..5` for the sampled zones
- workbook `Prize Catch` IDs did not cleanly match `fishing_table.DropID` in the sampled zones
- some workbook `Prize Catch` IDs exist in Dolt as group keys (`11057`, `11058`, `11060`), while others sampled from the workbook (`11056`, `11073`) were absent from the current runtime snapshot
- therefore the durable rule is:
  - treat `fishing_table` as the legacy RGB-to-slot layer
  - enrich `item_main_group_table` and `item_sub_group_table` for subgroup resolution
  - do not assume auxiliary workbook ids can safely overwrite `fishing_table` rows directly

Public label recommendation:

- `legacy reference`
- never present as “current confirmed rate”
- where workbook-derived wording is surfaced, prefer `legacy table reference`

### Community sheet

Type:

- curated hint / curated support layer

What it can support:

- zone-level fish presence hints
- optionally zone+group hints where explicitly curated
- support claims even where ranking evidence is sparse

What it cannot support by itself:

- freshness guarantees unless versioned locally
- true drop rates
- consistent denominators

Repo/runtime findings:

- no direct ingestion tooling for the provided remote sheet ids was found in the repo
- however, local workbook material exists under `data/fishing_tables_101/`
- `BDO - PRIZE FISHES.xlsx` already encodes source-like support semantics:
  - zone RGB
  - region
  - zone name
  - fish entries
  - remark/status values such as `CONFIRMED`, `UNCONFIRMED`, and `DATA INCOMPLETE`
- `Fishing Data.xlsx` provides a translated RGB-to-table-reference sheet with human descriptions and category-rate fields

Interpretation:

- for local ingestion design, the immediate practical input is not “remote Google Sheet only”
- the immediate practical input is “local manually curated workbook data, likely derived from the same community-maintained knowledge base”
- this is enough to design a first normalized import contract without blocking on online access

Recommended next step:

- define a local normalized ingestion contract for workbook-backed community support first
- keep direct Google Sheet ingestion as a later convenience layer or sync source
- do not block `zone_profile_v2` on live network access to the sheet

Public label recommendation:

- `community overlay`
- `community hint`
- for `CONFIRMED` workbook entries, `reference_supported` is a reasonable initial support-grade mapping
- for `UNCONFIRMED` or `DATA INCOMPLETE` workbook entries, `weak_hint` is the safer initial support-grade mapping

### Ranking events

Type:

- direct observation, but positive-only and time-bounded

What they can support:

- positive evidence that a fish has appeared in a zone
- recency/freshness
- ESS / stability of weighted evidence
- drift comparison across time windows
- advanced evidence-share summaries

What they cannot support:

- true catch/drop rates
- absence claims
- denominator-aware setup-specific rates
- border certainty

Repo/runtime findings:

- only `EventSourceKind::Ranking` exists in the public enum today
- runtime `events.source_kind` currently contains only `1`
- ranking snapshot and metadata APIs already expose ranking-only semantics
- `zone_stats` event-loading query does not filter `source_kind`, so the path is vulnerable to future contamination if additional event types are inserted

Public label recommendation:

- `observed in ranking data`
- `ranking evidence share` for advanced-only percentages

### Player logs

Type:

- direct observation with denominator potential

What they should support in future:

- positive presence claims
- setup- and location-scoped catch counts
- denominator-aware catch-rate summaries
- eventual subgroup/setup inference

What they cannot support unless collected carefully:

- global rates without bias controls
- generalized zone-wide rates from a tiny contributor set

Repo/runtime findings:

- no existing player-log table was found in runtime tables
- no player-log ingestion path was found in current code
- docs only mention direct catch logs as future work

Public label recommendation:

- when added, separate this family clearly from ranking evidence
- `player logs`
- `catch-rate summary`

## 4) Risk Register

### Risk: public misreads `p_mean` as drop rate

Why it exists:

- current default map panel renders `pMean` as a percent badge

Mitigation:

- remove percent as the default public primary signal in `zone_profile_v2`
- if preserved, nest it under `ranking_evidence` and rename it to `ranking evidence share`

### Risk: source-kind contamination

Why it exists:

- live `zone_stats` SQL joins `events` and `event_zone_assignment` without filtering `events.source_kind`

Impact:

- future non-ranking event types could silently change ranking evidence semantics

Mitigation:

- isolate ranking evidence loading by source family
- add typed source-family boundaries in the new model
- add tests around source-specific loading before multi-source ingestion lands

### Risk: outdated legacy data looks authoritative

Why it exists:

- `fishing_table` contains structured rates and slot/group keys

Impact:

- users may treat historical or stale tables as current truth

Mitigation:

- label as `legacy reference`
- keep legacy support in `presence_support`, not `catch_rates`
- attach freshness / provenance notes where available

### Risk: border ambiguity is confused with evidence uncertainty

Why it exists:

- current product has one mixed zone-evidence panel and no point-level border model

Impact:

- ESS or drift may be wrongly read as “confidence the click belongs to this zone”

Mitigation:

- create separate `assignment` and `ranking_evidence` sections
- reserve ESS for evidence quality only
- add explicit border class and neighboring-zone output

### Risk: missing evidence is misread as absence

Why it exists:

- sparse ranking evidence naturally produces empty distributions

Impact:

- users over-trust empty or weak panels as proof a fish is absent

Mitigation:

- preserve `unknown` / `insufficient_evidence`
- distinguish “no support observed” from “not present”

### Risk: newer-zone geometry reliability issues

Why it exists:

- community zone masks are updated manually and can lag content changes

Impact:

- assignment and presence support may disagree near boundaries or changed regions

Mitigation:

- explicit border ambiguity state
- border stress diagnostics
- boundary QA outputs linked to evidence divergence

### Risk: future player logs contaminate ranking evidence

Why it exists:

- current live `zone_stats` loading path is not source-filtered
- `EventSourceKind` does not yet model additional source families

Mitigation:

- make ranking evidence a source-scoped submodel
- treat logs as a separate source family and separate metric family
- never aggregate logs into ranking evidence share fields

## 5) Proposed Target Model

### Core response

```rust
ZoneProfileV2 {
    assignment: ZoneAssignment,
    presence_support: ZonePresenceSupport,
    ranking_evidence: Option<ZoneRankingEvidence>,
    catch_rates: Option<ZoneCatchRateSummary>,
    diagnostics: ZoneDiagnostics,
}
```

### Proposed request shape

`zone_profile_v2` should not reuse the exact `zone_stats` request unchanged.

It needs the current analysis window, but also the clicked point context needed for border ambiguity:

```rust
ZoneProfileV2Request {
    layer_revision_id: Option<String>,
    layer_id: Option<String>,
    patch_id: Option<String>,
    at_ts_utc: Option<Timestamp>,
    map_version_id: Option<MapVersionId>,
    rgb: RgbKey,
    map_px_x: Option<i32>,
    map_px_y: Option<i32>,
    from_ts_utc: Timestamp,
    to_ts_utc: Timestamp,
    tile_px: u32,
    sigma_tiles: f64,
    fish_norm: bool,
    alpha0: f64,
    top_k: usize,
    half_life_days: Option<f64>,
    drift_boundary_ts_utc: Option<Timestamp>,
    ref_id: Option<String>,
    lang: Option<String>,
}
```

If a caller only knows zone RGB and not click position, `assignment` should explicitly return `unavailable` for point-level border distance/classification rather than fabricating precision.

### Assignment

Purpose:

- answers where the click is, spatially
- does not speak for ranking evidence quality

Suggested shape:

```rust
ZoneAssignment {
    zone_rgb_u32: u32,
    zone_rgb: RgbKey,
    zone_name: Option<String>,
    point: Option<ZonePoint>,
    border: ZoneBorderAssessment,
    neighboring_zones: Vec<ZoneNeighborCandidate>,
}

ZoneBorderAssessment {
    class: ZoneBorderClass,        // core | near_border | ambiguous | unavailable
    nearest_border_distance_px: Option<f64>,
    method: ZoneBorderMethod,      // mask_distance | local_sample | unavailable
    warnings: Vec<String>,
}
```

### Presence support

Purpose:

- answers what support exists that a fish appears in this zone
- makes provenance explicit

Suggested shape:

```rust
ZonePresenceSupport {
    fish: Vec<ZoneFishSupport>,
}

ZoneFishSupport {
    fish_id: i32,
    item_id: i32,
    encyclopedia_key: Option<i32>,
    encyclopedia_id: Option<i32>,
    fish_name: Option<String>,
    support_grade: ZoneSupportGrade,
    source_badges: Vec<ZoneSourceFamily>,
    claims: Vec<ZoneSupportClaim>,
}
```

Support grades:

- `observed_recent`
- `observed_historical`
- `reference_supported`
- `weak_hint`
- `unknown`

Source families:

- `legacy`
- `community`
- `ranking`
- `logs`

### Ranking evidence

Purpose:

- advanced ranking-only diagnostics
- not the default public interpretation layer

Suggested shape:

```rust
ZoneRankingEvidence {
    source_family: ZoneSourceFamily,   // always ranking
    total_weight: f64,
    ess: f64,
    raw_event_count: Option<u64>,
    last_seen_ts_utc: Option<Timestamp>,
    age_days_last: Option<f64>,
    status: ZoneRankingStatus,
    drift: Option<ZoneRankingDrift>,
    fish: Vec<ZoneRankingFishEvidence>,
}

ZoneRankingFishEvidence {
    fish_id: i32,
    item_id: i32,
    encyclopedia_key: Option<i32>,
    encyclopedia_id: Option<i32>,
    fish_name: Option<String>,
    evidence_weight: f64,
    evidence_share_mean: Option<f64>,
    ci_low: Option<f64>,
    ci_high: Option<f64>,
}
```

Naming rule:

- do not carry forward raw `p_mean` as the public semantic name
- if kept, rename to `evidence_share_mean` and label it explicitly as ranking-only

### Catch rates

Purpose:

- future denominator-aware player-log statistics only

Suggested shape:

```rust
ZoneCatchRateSummary {
    source_family: ZoneSourceFamily,   // logs
    availability: ZoneMetricAvailability,
    fish: Vec<ZoneFishCatchRate>,
    notes: Vec<String>,
}
```

Important rule:

- never infer this from ranking events
- `None` or `availability=unavailable` is the correct result until logs exist

### Diagnostics

Purpose:

- explain warnings and future optimization hooks without blending semantics

Suggested shape:

```rust
ZoneDiagnostics {
    public_state: ZonePublicState,
    insufficient_evidence: bool,
    border_sensitive: Option<bool>,
    border_stress: Option<ZoneBorderStressSummary>,
    notes: Vec<String>,
}
```

`public_state` should cover user-facing availability, for example:

- `unknown`
- `insufficient_evidence`
- `supported`
- `stale`
- `drifting`

## 6) Border Ambiguity And Border Stress

### A) Point-level border ambiguity

This is a click-level assignment question, not an evidence-quality question.

Recommended classes:

- `core`
- `near_border`
- `ambiguous`
- `unavailable`

Recommended behavior:

- if click pixel is available, sample the zone mask at that pixel
- if a border-distance implementation exists, return distance and class
- if only local neighborhood sampling exists, return a class and plausible neighbors without fake exact distance
- if no point coordinates are provided, return `unavailable`

Current feasibility assessment:

- exact border distance is not currently implemented anywhere in the repo
- however it is feasible from currently available assets
- the repo has a full local zone mask PNG at `data/cdn/public/images/zones_mask_v1.png`
- the repo also has `ZoneMask::load_png` and exact RGB sampling in `lib/fishystuff_core/src/masks.rs`

Conclusion:

- point-level border distance is feasible with new code
- it is not currently supported
- first-pass API should therefore model availability explicitly

### B) Zone-level border sensitivity / stress

This is a zone-diagnostic and mask-QA concept, not the same thing as point-level ambiguity.

Recommended method:

1. Define a border band around each zone in mask space.
2. Partition evidence into:
   - core region
   - near-border band
   - neighbor-border bands
3. Compute ranking evidence distributions for each partition.
4. Compare:
   - zone core vs its border band
   - border band vs neighboring-zone border bands
5. Use divergence metrics such as JSD.

Recommended outputs:

- `border_sensitive: bool`
- `near_border_weight_fraction`
- `core_vs_border_jsd`
- per-neighbor stress summary:
  - `neighbor_zone_rgb`
  - `neighbor_zone_name`
  - `shared_border_weight`
  - `cross_border_jsd`
- future debug overlay:
  - raster or vector heatmap of stressed boundary segments

Fit with existing docs:

- this aligns with `docs/boundary-qa-and-updates.md`
- the existing “edge divergence map” should become one of the diagnostic inputs for `border_stress`

User-facing rule:

- border stress is a diagnostic about zone-mask quality and cross-boundary mixing
- it is not the same thing as freshness or ESS

## 7) Migration Plan

### Phase 1

- ship this RFC
- add typed `zone_profile_v2` request/response models
- add source-family enums and support-grade enums
- keep `/api/v1/zone_stats` intact

Status after first additive slice:

- completed
- additive endpoint now exists at `/api/v1/zone_profile_v2`
- `/api/v1/zone_stats` remains intact

### Phase 2

- add a ranking-heavy `zone_profile_v2` backend composition path
- split the payload into:
  - `assignment`
  - `presence_support`
  - `ranking_evidence`
  - `diagnostics`
- keep `catch_rates` unavailable / placeholder
- move ranking percentages into advanced-only `ranking_evidence`

Status after first additive slice:

- partially completed
- ranking-backed `presence_support`, `ranking_evidence`, and `diagnostics` are now wired
- `assignment.border` remains an explicit unavailable placeholder when point coordinates are absent or the zone mask runtime asset is unavailable
- `catch_rates` remains an explicit pending-source placeholder

### Phase 3

- add legacy/community support inputs into `presence_support`
- keep `fishing_table` as the zone-slot baseline and prefer legacy/community enrichment at the group-table layer
- add first point-level border classification
- keep exact distance optional until the mask-distance primitive exists

Status after the current additive slice:

- partially completed
- `presence_support` now merges ranking observations with legacy fishing-table support resolved through `fishing_table -> item_main_group_table -> item_sub_group_table`
- community-backed support is now wired through a dedicated `community_zone_fish_support` runtime table and merged into `presence_support` as source-scoped claims
- if the community support table is missing or present-but-empty, the API still reports the community layer as unavailable rather than implying absence
- point-level border classification now uses local zone-mask neighborhood sampling when click coordinates are provided
- exact border distance remains unimplemented and is intentionally reported as unavailable rather than estimated
- terrain-only mask neighbors are ignored for point-level border ambiguity so shoreline proximity does not masquerade as neighboring-zone ambiguity

### Phase 4

- add player-log schema and ingestion
- add denominator-aware `catch_rates`
- preserve strict separation from ranking evidence

### Phase 5

- add border stress analytics and debug outputs
- expose QA overlays or offline analytics outputs for mask maintenance

This sequence is preferred because it fixes semantics and model boundaries before adding more data sources.

## Concrete answers to required verification tasks

### How the live `zone_stats` path computes weights and ESS

Live server path:

- loads ranking-scoped zone-assigned in-window events with `load_ranking_events_with_zone_in_window`
- computes `w_time` from `half_life_days`
- if `fish_norm` is enabled, computes per-fish normalization from recency-weight sums
- uses `u = w_time` for zone total weight and ESS
- uses `w = u * fish_norm` only for displayed fish evidence when `fish_norm=true`
- computes `ESS = (w_sum * w_sum) / w2_sum`

### Whether live server semantics differ from docs semantics

Yes.

- docs say effort debiasing is part of zone evidence and ESS
- live server does not apply `w_eff`

### Whether standalone analytics semantics differ from live server

Yes.

- analytics computes blurred effort, inverse-effort clipping, and uses it in both fish evidence and ESS
- live server does not

### Whether live `zone_stats` SQL filters `events.source_kind`

Yes, now.

- the ranking-derived zone-evidence query now filters `e.source_kind = SOURCE_KIND_RANKING`
- this safeguard is shared by the ranking-backed `zone_profile_v2` path and the existing ranking-derived `zone_stats` / `effort_grid` loaders

### Whether that creates a concrete future contamination risk

Previously yes; this slice now mitigates that specific loading-path risk.

- the public source enum still only exposes `Ranking`
- future multi-source work still needs explicit new query paths rather than reusing ranking-only loaders

### Whether `EventSourceKind` supports anything beyond ranking

No.

- current public enum contains only `Ranking`
- current DB mapping helper also only maps code `1` to `Ranking`

### What `fishing_table` and related schema can concretely provide

Available today:

- zone RGB to slot rows
- slot-level legacy rates
- main-group references
- main-group option rows
- subgroup item variants in the current local runtime
- fish identity/icon mapping

Not available from the raw legacy import alone:

- trustworthy subgroup item variants without additional backfill or enrichment work

Additional local-source finding:

- the workbook `Fishing Data.xlsx` provides a human-readable translated RGB table that can supplement runtime SQL with descriptions and category-rate context during ingestion or auditing
- the workbook has `158` translated RGB rows, while runtime `fishing_table` currently has `276` rows, so the workbook should be treated as a dated reference subset rather than a full live mirror
- the raw legacy XLSX dump under `data/data/excel/` is the durable source-schema backbone for `fishing_table`, `item_main_group_table`, `item_sub_group_table`, and `item_table`
- maintained import work should enrich the legacy group tables first; it should not assume a direct `fishing_table` rewrite is safe

### Whether player-log schema already exists

No current schema or ingestion path was found.

### Whether border distance can be computed from currently available assets

Partially.

- assets exist locally
- exact mask RGB sampling code exists
- local-neighborhood border classification is now implemented from the cached zone mask asset
- no distance-transform or nearest-boundary primitive currently exists, so exact border distance is still unavailable

## Implemented first additive slice

This repo now includes the first additive `zone_profile_v2` slice:

1. typed request/response models under `lib/fishystuff_api/src/models/zone_profile_v2.rs`
2. additive backend route at `/api/v1/zone_profile_v2`
3. ranking-heavy composition path that maps current ranking evidence into separated sections:
   - `assignment`
   - `presence_support`
   - `ranking_evidence`
   - `catch_rates`
   - `diagnostics`
4. semantics-focused tests that lock:
   - ranking evidence share is nested and explicitly named
   - missing ranking evidence is not treated as absence
   - placeholder border state is explicit
   - catch rates remain a typed pending-source placeholder
5. ranking-event load path isolation via `source_kind`
6. legacy-backed `presence_support` claims resolved from the current Dolt runtime tables rather than from workbook parsing in the API path
7. community-backed `presence_support` claims resolved from a dedicated imported runtime table rather than from workbook parsing in the API path
8. store-side v2 profile composition moved under `api/fishystuff_server/src/store/dolt_mysql/zone_profile_v2/` so future work does not accumulate inside `dolt_mysql.rs`
9. point-level border classification now uses cached `zones_mask_v1.png` neighborhood sampling for `core | near_border | ambiguous | unavailable`

### Fully implemented in this slice

- additive `/api/v1/zone_profile_v2` route and store plumbing
- ranking-backed `presence_support`
- legacy-backed `presence_support` claims from the current Dolt fishing tables
- community-backed `presence_support` claims from `community_zone_fish_support` when that table is populated
- ranking-backed `ranking_evidence`
- explicit typed `catch_rates` placeholder
- explicit typed `assignment.border` classification from local mask-neighborhood sampling when point coordinates are available
- diagnostics notes that separate ranking evidence, assignment ambiguity, and catch-rate estimation

### Still placeholder in this slice

- exact point-level border distance
- live-populated community support data in the current runtime
- player-log catch-rate summaries
- border stress metrics and overlays

### Public wording for this slice

Use wording like:

- `observed in ranking data`
- `supported by legacy fishing tables`
- `supported by curated community zone data`
- `insufficient ranking evidence`
- `border ambiguity unavailable`
- `catch rates unavailable`

Avoid wording like:

- `drop rate`
- `chance to catch`
- `border confidence`

### Advanced wording for this slice

- `ranking evidence share`
- `effective sample size (ESS)`
- `fresh/stale/drifting ranking evidence`

Advanced wording must still avoid describing ranking evidence share as a drop rate.

### Migration note

- `/api/v1/zone_stats` is intentionally preserved for compatibility
- the old route still exposes `p_mean`/CI semantics that are easy to misread publicly
- the new route is the additive path for safer semantics and future multi-source composition
- the new route keeps ranking evidence and legacy support in separate sections/claims rather than blending them into one score
- the new route also keeps curated community support in `presence_support` rather than promoting it into ranking evidence or catch-rate semantics

### Current community-support runtime note

- the repo now defines a dedicated `community_zone_fish_support` table for curated zone/fish presence claims
- the importer for the maintained community prize-fish workbook writes into that table
- the API treats a missing table or an empty table as `unavailable`, not as `no fish supported`
- this is intentional so an unpopulated runtime does not silently turn missing community data into false absence

## Non-goals for this RFC pass

- no sweeping UI rewrite
- no replacement of `/api/v1/zone_stats`
- no fabricated border-distance number
- no fake catch-rate summaries
