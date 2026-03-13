# Implementation plan (Rust-first, local)

This plan assumes:
- local execution
- Rust backend + static web UI (deck.gl)
- Dolt repository available locally

## Crate layout

### 1) `fishystuff_core`
- PNG decoding helpers (watermask, zonemask)
- coordinate transforms
- water snapping
- tiling utilities
- Gaussian blur on 2D grids
- Dirichlet/Beta math utilities (sampling, JS divergence)

### 2) `fishystuff_ingest` (CLI)
- input: ranking CSV
- output: canonical event store (SQLite or binary)
- outputs optional QA stats (snapping failure rate, etc.)

### 3) `fishystuff_analytics`
- loads event store + masks
- executes query:
  - filters events by time
  - builds effort grid
  - computes inverse-effort weights
  - assigns to zones via zone mask pixel reads
  - computes posterior distributions
  - computes drift metrics (optional)
- caches results keyed by (ref, map_version_id, params)

### 4) `fishystuff_server` (local HTTP)
- serves UI static files
- exposes API endpoints from spec
- handles dolt exports (either via CLI wrapper or preloaded CSV)

## PNG handling

- Use `png` crate or `image` crate.
- Convert to raw RGB arrays.
- For watermask: only check exact (0,0,255).

## Numeric considerations

- Use f32 for grids to reduce memory.
- Keep sums in f64 if needed, but current scale is small.

## Determinism

- All random sampling (Dirichlet for drift probability) uses a fixed seed:
  - e.g. `seed = hash(zone_key, t0, params)`
- Ensure the same request yields identical output.

## Testing

1) Golden tests:
- fixed small subset of events + tiny masks
- verify effort grid and zone stats exactly

2) Property tests:
- weights clipping bounds
- ESS monotonicity with duplicated events

3) Integration test:
- run server and query known zone, compare to expected JSON schema

## Milestones

M1 — Map + zone picking:
- render base + zone mask
- show name from zones_merged

M2 — Zone distributions:
- ingest ranking events
- compute effort map and zone signatures (no drift)

M3 — Confidence + recency:
- ESS, last seen, badges
- patch window filter

M4 — Drift detection:
- zone_compare endpoint
- UI compare panel

M5 — Boundary QA (optional):
- edge divergence overlay
- suspicious edge report

M6 — Direct catch logs integration (future):
- separate data source with higher weight
- context bucketing
