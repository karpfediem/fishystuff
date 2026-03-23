# PABR and World-Map Metadata Guide

This guide documents how to work with the original Black Desert world-map
files that `pazifista` currently understands.

Use this file as the stable reference.

Use the investigation log in
[worklog/pabr-region-map-investigation.md](/home/carp/code/fishystuff/data/spec/worklog/pabr-region-map-investigation.md)
for the step-by-step reverse-engineering history, dead ends, and narrower
findings that are not yet polished into a final spec.

Related reference:

- [world-map-sector-model.md](/home/carp/code/fishystuff/data/spec/world-map-sector-model.md)

## Purpose

The project is moving away from stale community-derived map artifacts and
toward original-game sources.

In practice that means:

- replacing smoothed external GeoJSON for `regions` and `region_groups`
- replacing stale external waypoint metadata where original waypoint XML exists
- understanding whether the current `zone mask` can also be replaced with an
  original source

## Source of Truth

Current recommended source-of-truth chain:

- region geometry
  - `*.bmp.rid` + `*.bmp.bkd`
- region ID metadata
  - `regioninfo.bss`
  - `regionclientdata_*.xml`
- region-group metadata
  - `regiongroupinfo.bss`
  - `mapdata_realexplore.xml` / `mapdata_realexplore2.xml`
- waypoint names and positions
  - `mapdata_realexplore.xml` / `mapdata_realexplore2.xml`

Files that are useful for investigation but are not the preferred naming source:

- `exploration.bss`
- `stringtable.bss`
- `mapdata_arraywaypoint.bin`

## File Families

### `*.bmp.rid`

Purpose:

- region-ID dictionary
- native map dimensions in the validated footer

Use it for:

- region ID lookup
- native raster size
- pairing with the matching `*.bmp.bkd`

### `*.bmp.bkd`

Purpose:

- wrapped and sheared breakpoint rows that reconstruct the original region-map
  raster

Use it for:

- rendering original unsmoothed region geometry
- exporting exact region and region-group polygons

Important:

- these are not generic Elasticsearch or Lucene BKD trees

### `regioninfo.bss`

Purpose:

- region-level metadata such as:
  - `tradeoriginregion`
  - `regiongroup`
  - `waypoint`

Use it for:

- linking region IDs to their group IDs
- linking region IDs to waypoint IDs
- checking whether current external `regioninfo.json` is stale

### `regiongroupinfo.bss`

Purpose:

- region-group table with:
  - `key`
  - `waypoint`
  - graph position

Use it for:

- linking a region-group ID to a waypoint ID
- locating the group on the world map

### `mapdata_realexplore.xml`

Purpose:

- original large waypoint graph
- canonical internal waypoint names and positions

Observed characteristics:

- `179203` waypoints
- `412122` links
- includes many hidden and road sub-waypoints

Use it for:

- authoritative waypoint naming
- direct lookup of world-map waypoint IDs
- validating graph-point alignment from `regiongroupinfo.bss`

### `mapdata_realexplore2.xml`

Purpose:

- second, smaller waypoint graph in the same schema

Observed characteristics:

- `1022` waypoints
- `2338` links

Use it for:

- cross-checking names and a smaller high-level graph
- comparing alternate waypoint placements

Important:

- `mapdata_realexplore.xml` and `mapdata_realexplore2.xml` share waypoint keys
  but can disagree on position, links, and `IsSubWaypoint`

### `exploration.bss`

Purpose:

- original binary table that contains live waypoint IDs and related fields

Current status:

- useful for reverse-engineering
- not the preferred naming source
- the previously explored `exploration.bss -> stringtable.bss` path turned out
  to lead to unrelated UI strings for the tested focus rows

### `stringtable.bss`

Purpose:

- general string index plus trailing UTF-16LE text entries

Current status:

- partially decoded
- useful for broader client text work
- not currently needed to name world-map waypoints when
  `mapdata_realexplore*.xml` is available

### `mapdata_arraywaypoint.bin`

Purpose:

- sector-native semantic raster

Current status:

- decoded as a sector-aligned `u16` grid
- not a direct `waypoint_id -> name/position` table
- not yet a drop-in replacement for the current zone-mask assets

## Core Decode Model

### Region Raster Reconstruction

The validated region-map pipeline is:

1. read native width and height from `*.bmp.rid`
2. read breakpoint rows from `*.bmp.bkd`
3. undo the wrapped-band storage model
4. undo the fixed per-row shear
5. map dictionary indices through the RID dictionary
6. reconstruct the unsmoothed raster by majority vote across wrapped bands

For the currently validated family:

- native size example: `11560 x 10540`
- row shear: `3824`
- wrapped bands derived from `max_bkd_x / native_width`

Implementation:

- [parse.rs](/home/carp/code/fishystuff/tools/pazifista/src/pabr/parse.rs)
- [render.rs](/home/carp/code/fishystuff/tools/pazifista/src/pabr/render.rs)
- [geojson.rs](/home/carp/code/fishystuff/tools/pazifista/src/pabr/geojson.rs)

### Region and Region-Group Metadata

Current reliable metadata chain:

- region geometry comes from `rid+bkd`
- region ID to group ID comes from `regioninfo.bss`
- group ID to waypoint comes from `regiongroupinfo.bss`
- waypoint name and position come from `mapdata_realexplore*.xml`

That means region-group labeling no longer depends on stale external
`deck_rg_graphs.json` or `waypoints.json`.

### Waypoint XML Schema

The validated `mapdata_realexplore*.xml` rows look like:

```xml
<Waypoint
  Key="2052"
  Name="town(olvia_academy)"
  PosX="-114942"
  PosY="-2674.33"
  PosZ="157114"
  Property="ground"
  IsSubWaypoint="True"
  IsEscape="False"/>
```

And graph edges look like:

```xml
<Link SourceWaypoint="205741" TargetWaypoint="2052"/>
```

So the XML is already sufficient to recover:

- waypoint key
- canonical internal name token
- world position
- local graph connectivity

## The `2052` Example

The Olvia redesign chain is the clean reference example.

From original data:

- `regiongroupinfo.bss`
  - `group 295`
  - `waypoint = 2052`
  - graph point `(-114535, -2674, 157512)`
- `mapdata_realexplore.xml`
  - `Waypoint Key="2052"`
  - `Name="town(olvia_academy)"`
  - `Pos=(-114942, -2674.33, 157114)`
- `mapdata_realexplore2.xml`
  - `Waypoint Key="2052"`
  - `Name="town(olvia_academy)"`
  - `Pos=(-125229, -2883.02, 146801)`

Important interpretation:

- the missing external waypoint was a stale downstream-data problem
- the original files do contain the live waypoint and its canonical internal
  name
- the `regiongroupinfo.bss` graph point is much closer to the
  `mapdata_realexplore.xml` position than to the `realexplore2` position, so
  `mapdata_realexplore.xml` is the better match for group linkage

## `pazifista` Commands

### Inspect a region map pair

```bash
devenv shell -- cargo run -q -p pazifista -- \
  pabr inspect data/scratch/ui_texture/minimap/area/regionmap_new.bmp.rid
```

### Render a debug BMP

```bash
devenv shell -- cargo run -q -p pazifista -- \
  pabr render \
  data/scratch/ui_texture/minimap/area/regionmap_new.bmp.rid \
  -o /tmp/regionmap_new.bmp
```

### Export exact unsmoothed regions GeoJSON

```bash
devenv shell -- cargo run -q -p pazifista -- \
  pabr export-regions-geojson \
  data/scratch/ui_texture/minimap/area/regionmap_new.bmp.rid \
  -o /tmp/regions.geojson
```

### Export exact unsmoothed region-groups GeoJSON

```bash
devenv shell -- cargo run -q -p pazifista -- \
  pabr export-region-groups-geojson \
  data/scratch/ui_texture/minimap/area/regionmap_new.bmp.rid \
  --regioninfo data/scratch/gamecommondata/binary/regioninfo.bss \
  -o /tmp/region-groups.geojson
```

### Inspect `regioninfo.bss`

```bash
devenv shell -- cargo run -q -p pazifista -- \
  gcdata inspect-regioninfo-bss \
  data/scratch/gamecommondata/binary/regioninfo.bss \
  --id 1677 --id 1688 -o /tmp/regioninfo-focus.json
```

### Inspect `regiongroupinfo.bss`

```bash
devenv shell -- cargo run -q -p pazifista -- \
  gcdata inspect-regiongroupinfo-bss \
  data/scratch/gamecommondata/binary/regiongroupinfo.bss \
  --id 295 -o /tmp/regiongroup-focus.json
```

### Inspect original waypoint XML

```bash
devenv shell -- cargo run -q -p pazifista -- \
  gcdata inspect-waypoint-xml \
  data/scratch/gamecommondata/waypoint/mapdata_realexplore.xml \
  --id 1739 --id 1746 --id 2052 \
  -o /tmp/realexplore-focus.json
```

### Inspect `mapdata_arraywaypoint.bin`

```bash
devenv shell -- cargo run -q -p pazifista -- \
  gcdata inspect-arraywaypoint-bin \
  data/scratch/gamecommondata/waypoint_binary/mapdata_arraywaypoint.bin \
  --preview-bmp /tmp/arraywaypoint.bmp \
  -o /tmp/arraywaypoint.json
```

## Recommended Workflow

When adding or rebuilding map layers, use this order:

1. geometry
   - derive from `rid+bkd`
2. region-to-group linkage
   - derive from `regioninfo.bss`
3. group-to-waypoint linkage
   - derive from `regiongroupinfo.bss`
4. waypoint naming and placement
   - derive from `mapdata_realexplore*.xml`
5. only then compare against external JSON or GeoJSON
   - treat external artifacts as compatibility outputs, not authoritative

## Replacement Status

### `regions`

Status:

- geometry: original source is available and decoded
- metadata: mostly original-source backed
- remaining work: reconcile the last ID mismatches and wire the original chain
  cleanly into the production layer build

### `region_groups`

Status:

- geometry: original source is available and decoded
- metadata: original-source backed
- naming path: `regiongroupinfo.bss -> waypoint -> mapdata_realexplore*.xml`

This is the cleanest layer to move fully off external GeoJSON.

### `waypoints`

Status:

- original source exists
- `mapdata_realexplore*.xml` should be preferred over stale external
  `waypoints.json`

Open design decision:

- whether the production output should favor the denser `realexplore` graph,
  the smaller `realexplore2` graph, or a derived merged view

### `zone mask`

Status:

- not replaced yet
- `mapdata_arraywaypoint.bin` is original and decoded
- but it is a coarse sector-native semantic raster, not yet a proven
  substitute for the current PNG-plus-bin pair

## Known Unknowns

Still not fully resolved:

- the exact meaning of the remaining RID trailer fields
- the full structure of every `regioninfo.bss` row family
- the exact relationship between `mapdata_realexplore.xml` and
  `mapdata_realexplore2.xml`
- whether there is a fully original source for the current zone-mask semantics
- whether `exploration.bss` still carries useful metadata that is not already
  easier to recover from the waypoint XML

## Implementation Pointers

Region-map decode modules:

- [tools/pazifista/src/pabr/mod.rs](/home/carp/code/fishystuff/tools/pazifista/src/pabr/mod.rs)
- [tools/pazifista/src/pabr/parse.rs](/home/carp/code/fishystuff/tools/pazifista/src/pabr/parse.rs)
- [tools/pazifista/src/pabr/render.rs](/home/carp/code/fishystuff/tools/pazifista/src/pabr/render.rs)
- [tools/pazifista/src/pabr/geojson.rs](/home/carp/code/fishystuff/tools/pazifista/src/pabr/geojson.rs)
- [tools/pazifista/src/pabr/matching.rs](/home/carp/code/fishystuff/tools/pazifista/src/pabr/matching.rs)

Metadata and waypoint inspection:

- [tools/pazifista/src/gcdata.rs](/home/carp/code/fishystuff/tools/pazifista/src/gcdata.rs)
- [tools/pazifista/src/gcdata/array_waypoint.rs](/home/carp/code/fishystuff/tools/pazifista/src/gcdata/array_waypoint.rs)
- [tools/pazifista/src/gcdata/stringtable.rs](/home/carp/code/fishystuff/tools/pazifista/src/gcdata/stringtable.rs)
- [tools/pazifista/src/gcdata/waypoint_xml.rs](/home/carp/code/fishystuff/tools/pazifista/src/gcdata/waypoint_xml.rs)
