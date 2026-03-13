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

## Zone indexing

`fishystuff_ingest build-event-zone-assignment` writes `event_zone_assignment` rows per `layer_revision_id`:

- samples zone mask at `(sample_px_x, sample_px_y) = (snap_px_x, snap_px_y)`
- inserts only missing `(layer_revision_id, event_id)` rows
- rerunnable and idempotent
- supports additional revisions without reimporting events

## Deferred water normalization

- true water-normalized effort and `water_tiles` are deferred until a trustworthy water source exists
- `events_snapshot` and `zone_stats` must work without depending on water tile ingestion

## Notes

- `data/**/*.csv` and `data/**/*.xlsx` are local tooling inputs; do not modify them during API/frontend work.
