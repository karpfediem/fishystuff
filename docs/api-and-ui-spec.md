# API and UI specification (local)

## UI goals

- Interactive map with:
  - base world image
  - zone mask overlay
  - hover: zone name, RGB, coords
  - click: copy bookmark using community zone name
- Side panel:
  - time window controls (patch start / end)
  - recency half-life slider
  - effort map toggle
  - zone distribution for hovered/selected zone
  - drift status and compare panel

## Required UI features

1) Zone search by name and by RGB.
2) Fish search by ID or name; show fish id/name in lists.
3) Display top-K fish with:
   - evidence share (posterior mean)
   - credible interval
   - raw weighted evidence mass
4) Confidence display:
   - ESS, last seen, age days
   - High/Medium/Low/Unknown badge
5) Outdatedness display:
   - Stale / Drifting labels
   - Compare pre vs post patch (if requested)
6) Map markers should render fish icons by joining zone evidence fish ids
   (`EncyclopediaKey`) to `/api/fish_table`. Prefer the `icon` file name
   (e.g. `00008201.png`) published under `/images/FishIcons/` from
   `site/assets/images/FishIcons/`, and fall back to `encyclopedia_icon`
   if needed.

## Minimal API endpoints

All endpoints accept:
- `ref` (dolt commit ref; optional, default HEAD)
- `map_version_id` (optional; default derived by time or latest)
- `from_ts`, `to_ts` (UTC seconds)
- `half_life_days` (optional)
- `tile_px`, `sigma_tiles` (optional)
- `per_fish_normalize` (optional)

### GET /api/meta
Returns:
- available dolt refs (optional)
- patch list
- map versions list
- default parameters

### GET /api/zones?ref=...
Returns:
- list of zones: RGB, name, flags, metadata from zones_merged

### GET /api/v1/fish?lang=en|ko
Returns:
- `revision`, `count`, `fish[]`
- each fish entry is a compact DTO:
  - `fish_id`
  - `name`
  - optional `grade`
  - optional `is_prize`
  - optional `icon_url`

### GET /api/fish_map?encyclopedia_key=... or /api/fish_map?item_key=...
Returns:
- mapping between encyclopedia key and item id, plus icon fields

### GET /api/fish_table
Returns:
- fish table entries with encyclopedia_key, item_key, name, icon, encyclopedia_icon

### GET /api/zone_stats?rgb=R,G,B&lang=en|ko&...
Returns:
- zone metadata (name, RGB)
- window params echoed
- confidence metrics: ESS, last_seen_ts
- status: Fresh/Stale/Drifting/Unknown
- distribution: top-K fish with CI and evidence

### GET /api/zone_compare?rgb=...&t0=patch_ts&...
Returns:
- OLD window distribution + metrics
- NEW window distribution + metrics
- D_mean, p_drift, p_thresh, N samples

### GET /api/effort_grid?...
Returns:
- tile grid effort values (optionally compressed)
- suitable for client heatmap

### GET /api/v1/events_snapshot_meta
Returns:
- snapshot revision/hash
- event_count
- source_kind
- optional last_updated_utc
- revisioned snapshot URL

### GET /api/v1/events_snapshot?revision=...
Returns:
- full compact ranking event snapshot for client-side rendering/filtering/clustering

### GET /api/boundary_qa?...
Optional:
- edge divergence overlay

## Response format

Use JSON. For large grids, allow:
- gzip compression
- or binary formats later (msgpack)
