# Events Snapshot Architecture (Bevy/WASM)

## Decision

Ranking events are treated as a bounded interactive dataset.

- Server provides an authoritative revisioned snapshot.
- Bevy (Rust+WASM) loads the snapshot once per revision.
- Interactive derivations are local:
  - viewport filtering
  - fish/time filtering
  - visible-tile scoping
  - deterministic clustering/grouping

## API shape

- `GET /api/v1/events_snapshot_meta`
  - returns: `revision`, `event_count`, `source_kind`, optional `last_updated_utc`, `snapshot_url`
- `GET /api/v1/events_snapshot?revision=<revision>`
  - returns all compact ranking events needed for point rendering

## Caching and transport

- Snapshot revision is explicit and stable for unchanged payloads.
- Client checks metadata and skips snapshot download when revision is unchanged.
- Snapshot responses are revision-cacheable (`immutable` semantics) and compressed (`gzip`/`br`).

## Why this split

Per-viewport querying was too chatty during pan/zoom/filter interactions.

Snapshot-first flow improves:
- UI responsiveness (local query latency)
- bandwidth usage (single download per revision)
- server load (no repeated viewport query fanout for interactive state changes)

## When to use server-side querying instead

Prefer server-side query endpoints for:
- unbounded/very large datasets that cannot be held in client memory
- strict server-enforced filtering/authorization boundaries
- heavy backend joins/aggregations that are not feasible in client runtime
