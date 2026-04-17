# Ingestion and indexing

This document defines the current Dolt/MySQL evidence pipeline for ranking samples.

## Core constraint

- There is currently no trusted canonical watermap for ingestion-time validation.
- Ranking samples are accepted directly as valid evidence points.
- Ingestion must not depend on water snapping.

## Ranking import semantics

`fishystuff_ingest import-ranking` writes directly into `events` and `ingest_runs`:

- events are revision-agnostic facts (no map/layer revision binding at ingest time)
- canonical map pixel coordinates come from authoritative world->map transform
- `snap_px_x = map_px_x`
- `snap_px_y = map_px_y`
- `snap_dist_px = 0`
- `water_ok = 1` (accepted evidence; not raster-validated water)
- no player/family identifiers are stored
- deterministic `event_uid` enables idempotent dedupe
- when a zone mask source is configured, import also backfills
  `event_zone_assignment` and `event_zone_ring_support` for the current
  `zone_mask` layer revision

## Zone indexing

`fishystuff_ingest build-event-zone-assignment` writes `event_zone_assignment` and
`event_zone_ring_support` rows per `layer_revision_id`:

- samples zone mask at `(sample_px_x, sample_px_y) = (snap_px_x, snap_px_y)`
- records all zone RGBs touched by the fixed-radius evidence ring
- inserts only missing `(layer_revision_id, event_id)` rows
- rerunnable and idempotent
- supports additional revisions without reimporting events
- zone-mask-backed revisions use a unique `layer_revision_id` derived from the
  mask content hash
- `layer_revisions.map_version_id` links that unique revision back to the
  logical map version
- `layer_revisions.revision_hash` stores the full source image hash for
  provenance

### Table semantics

`event_zone_assignment` now contains the canonical single-zone assignment for an
event under a specific zone-mask revision:

- one row per `(layer_revision_id, event_id)`
- `layer_revision_id` identifies the exact zone-mask bitmap revision used for
  the lookup
- for `zone_mask` revisions, the steady-state shape is a hash-backed id like
  `zone_mask:v1:<hash-prefix>`, not a bare map version id
- `zone_rgb` and `zone_r/g/b` are the zone-mask color sampled at the event's
  snapped evidence point
- `sample_px_x` and `sample_px_y` record the exact pixel used for that lookup;
  today this is `(snap_px_x, snap_px_y)`
- this table answers "which zone contains the event's canonical sampled
  position for this mask revision?"

`event_zone_ring_support` now contains the zone-overlap footprint of the fixed
radius ranking ring for the same mask revision:

- zero or more rows per `(layer_revision_id, event_id)`, keyed by
  `(layer_revision_id, event_id, zone_rgb)`
- `layer_revision_id` matches the same hash-backed zone-mask revision used by
  `event_zone_assignment`
- `zone_rgb` and `zone_r/g/b` identify every zone whose mask color is touched by
  the sampled ring perimeter
- `ring_center_px_x` and `ring_center_px_y` record the ring center used for the
  overlap test; today this is `(map_px_x, map_px_y)`
- `ring_fully_contained = 1` means the ring touched exactly one zone RGB for
  that revision
- `ring_fully_contained = 0` means the ring crossed into multiple zone colors,
  so the event is only partial support for each touched zone
- this table answers "which zones could this event support once the fixed-radius
  evidence uncertainty ring is taken seriously?"

### Current expected state

After a fresh zone-mask backfill:

- zone support rows should live under the hash-backed `zone_mask` layer revision
  recorded in `layer_revisions`
- `layer_revisions.revision_hash` is the full SHA-256 of the source mask image
- legacy bare-map-version rows such as `layer_revision_id = 'v1'` are obsolete
  compatibility leftovers and should be removed once the hash-backed revision is
  populated

## Deferred water normalization

- true water-normalized effort and `water_tiles` are deferred until a trustworthy water source exists
- `events_snapshot` and `zone_stats` must work without depending on water tile ingestion

## Notes

- `data/**/*.csv` and `data/**/*.xlsx` are local tooling inputs; do not modify them during API/frontend work.
