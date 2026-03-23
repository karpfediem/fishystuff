# PABR Region Map Investigation Worklog

This note documents the `PABR`-family `*.bmp.rid` and `*.bmp.bkd` files used
for Black Desert minimap region maps.

Status:

- this is a reverse-engineered format note, not an official vendor spec
- the geometry and reconstruction path below are validated against the current
  `pazifista` implementation
- some trailer fields are still unidentified

Current implementation:

- PABR module root: [tools/pazifista/src/pabr/mod.rs](/home/carp/code/fishystuff/tools/pazifista/src/pabr/mod.rs)
- parsing and format validation: [tools/pazifista/src/pabr/parse.rs](/home/carp/code/fishystuff/tools/pazifista/src/pabr/parse.rs)
- rendering: [tools/pazifista/src/pabr/render.rs](/home/carp/code/fishystuff/tools/pazifista/src/pabr/render.rs)
- GeoJSON export: [tools/pazifista/src/pabr/geojson.rs](/home/carp/code/fishystuff/tools/pazifista/src/pabr/geojson.rs)
- region matching: [tools/pazifista/src/pabr/matching.rs](/home/carp/code/fishystuff/tools/pazifista/src/pabr/matching.rs)
- CLI entrypoint: [tools/pazifista/src/lib.rs](/home/carp/code/fishystuff/tools/pazifista/src/lib.rs)

## Scope

These files are not generic Elasticsearch/CrateDB `BKD` files and not generic
`RID` image payloads.

They encode original region-map geometry:

- `*.rid` stores the region-ID dictionary and footer metadata
- `*.bkd` stores the run/breakpoint data used to reconstruct the raster map

The colors shown by `pazifista pabr render` are synthetic and carry no game
data. The meaningful information is the region geometry and the region numbers.

## Samples Used

The current decoding was validated primarily against:

- `regionmap_new.bmp.rid` + `regionmap_new.bmp.bkd`
- `regionmap_morning.bmp.rid` + `regionmap_morning.bmp.bkd`
- `siegemap.bmp.rid` + `siegemap.bmp.bkd`

Observed from `regionmap_new`:

- native size: `11560 x 10540`
- wrapped bands: `6`
- dictionary entries: `1264`
- BKD rows: `1860`
- max BKD x: `65535`

`regionmap_new.bmp.rid` contains all `1252` region IDs from the current
smoothed `regions.v1.geojson` plus `12` additional IDs, which is a strong
sanity check that the RID dictionary is a region-ID table rather than a color
palette.

## RID Layout

High-level structure:

```text
offset  size  meaning
0x00    4     magic = "PABR"
0x04    4     u32 dictionary_entry_count
0x08    ...   dictionary_entry_count * u16 region IDs
...     var   small per-file trailer prefix
EOF-47  47    fixed footer block
```

### RID Dictionary

The dictionary is a flat array of little-endian `u16` values.

For `regionmap_new`, the dictionary values are region IDs such as:

- min: `4`
- max: `1688`
- count: `1264`

For other maps the values may represent a smaller map-specific ID space, but
the decoding model is the same: BKD entries reference RID dictionary indices.

### RID Footer

The last `47` bytes form a stable footer signature.

Known bytes:

```text
00 00 60 FF FF FF 78 87 00 00 28 2D 00 00 2C 29 ...
```

Known fields inside that footer:

- width at footer offset `10`: little-endian `u16`
- height at footer offset `14`: little-endian `u16`

For the known region maps:

- width = `0x2D28 = 11560`
- height = `0x292C = 10540`

Unknown fields:

- the small trailer prefix immediately before the fixed 47-byte footer
- the remaining footer fields after width/height

The current parser treats only the validated signature and the width/height
fields as format requirements.

## BKD Layout

High-level structure:

```text
offset  size  meaning
0x00    4     magic = "PABR"
0x04    4     u32 row_count
0x08    ...   repeated row payloads
EOF-12  12    footer/trailer words
```

Row payloads:

```text
u32 breakpoint_count
breakpoint_count * (
    u16 x
    u16 dictionary_index
)
```

Observed invariants:

- x values are sorted within each row
- `dictionary_index == 65535` acts as a transparent/sentinel value
- trailing footer is three `u32`s
- for all validated samples the BKD footer is:
  - first word: `0`
  - second word: byte offset of the parsed row payload end
  - third word: `0`

Example:

```text
BKD trailer words: [0, payload_end_offset, 0]
```

## Decoding Model

The naive interpretation, "each BKD row is a direct scanline with x normalized
into 0..65535", is wrong and produces repeated diagonal artifacts.

The currently validated model is:

1. The native map width comes from the RID footer: `11560`
2. BKD x coordinates are stored in wrapped width-sized bands
3. For the validated samples, the number of bands is:

```text
wrapped_bands = floor(max_x / native_width) + 1
```

For `regionmap_new`:

```text
floor(65535 / 11560) + 1 = 6
```

4. Each BKD row is sheared horizontally by a constant per-row shift
5. For the validated samples, the shear step is:

```text
row_shift = 3824 = 0x0EF0
```

6. To reconstruct a pixel at local output x:

```text
row_offset = (row_index * row_shift) % native_width
global_x(band) = local_x + row_offset + band * native_width
```

7. Evaluate the BKD breakpoint state at each valid `global_x`
8. Map each non-sentinel dictionary index through the RID dictionary to get a
   region ID
9. Fold the bands back together by majority vote on region ID

This produces a plausible unsmoothed region map for all currently tested
samples.

### Why Majority Vote Works

On sampled nonempty pixels, band agreement is very high:

- `regionmap_new`: `98.46%`
- `regionmap_morning`: `99.86%`
- `siegemap`: `99.97%`

When bands disagree, they are almost always split across only two region IDs,
which typically happens near true region boundaries.

## Rendering Notes

The current `pazifista` renderer:

- uses the original geometry from `rid+bkd`
- uses synthetic stable colors derived from region ID
- fills missing/transparent pixels with a fixed blue background

That means `render` is suitable for:

- validating the decoded geometry
- generating debug previews
- comparing original unsmoothed region boundaries against smoothed GeoJSON

It is not yet intended as a canonical in-game color reproduction.

## CLI

Inspect a pair:

```bash
devenv shell -- cargo run -q -p pazifista --bin pazifista -- \
  pabr inspect data/scratch/ui_texture/minimap/area/regionmap_new.bmp.rid
```

Render a preview:

```bash
devenv shell -- cargo run -q -p pazifista --bin pazifista -- \
  pabr render data/scratch/ui_texture/minimap/area/regionmap_new.bmp.rid \
  --width 2048 \
  -o data/scratch/ui_texture/minimap/area/regionmap_new.tool.preview.bmp
```

Override the inferred row shear if needed for future variants:

```bash
... pabr render ... --row-shift 3824
```

Export unsmoothed regions GeoJSON directly from the original PABR pair:

```bash
devenv shell -- cargo run -q -p pazifista --bin pazifista -- \
  pabr export-regions-geojson \
  data/scratch/ui_texture/minimap/area/regionmap_new.bmp.rid \
  -o /tmp/pazifista-regions.raw.geojson
```

Export unsmoothed region-groups GeoJSON by mapping region IDs through
`regioninfo.json`:

```bash
devenv shell -- cargo run -q -p pazifista --bin pazifista -- \
  pabr export-region-groups-geojson \
  data/scratch/ui_texture/minimap/area/regionmap_new.bmp.rid \
  --regioninfo /home/carp/code/clones/shrddr.github.io/workerman/data/regioninfo.json \
  -o /tmp/pazifista-region-groups.raw.geojson
```

Match PABR-derived regions against the current shipped `regions.v1.geojson` by
pixel overlap:

```bash
devenv shell -- cargo run -q -p pazifista --bin pazifista -- \
  pabr match-regions \
  data/scratch/ui_texture/minimap/area/regionmap_new.bmp.rid \
  --current-regions data/cdn/public/region_groups/regions.v1.geojson \
  -o /tmp/pazifista-region-matches.json
```

Inspect which original `regionclientdata_*.xml` variants know about specific
region IDs:

```bash
devenv shell -- cargo run -q -p pazifista --bin pazifista -- \
  gcdata inspect-regionclientdata \
  /tmp/paz-region-meta/gamecommondata/regionclientdata_ps_.xml \
  /tmp/paz-region-meta/gamecommondata/regionclientdata_sa_.xml \
  /tmp/paz-region-meta-more/gamecommondata/regionclientdata_en_.xml \
  /tmp/paz-region-meta-more/gamecommondata/regionclientdata_na_.xml \
  /tmp/paz-region-meta-more/gamecommondata/regionclientdata_dv_.xml \
  --id 1070 --id 1150 --id 1677 --id 1678 --id 1679 --id 1680 \
  --id 1681 --id 1682 --id 1683 --id 1684 --id 1685 --id 1686 \
  --id 1687 --id 1688
```

Compare the current region layer against the original PAZ-side metadata chain:

```bash
devenv shell -- cargo run -q -p pazifista --bin pazifista -- \
  gcdata compare-region-sources \
  --rid data/scratch/ui_texture/minimap/area/regionmap_new.bmp.rid \
  --current-regions data/cdn/public/region_groups/regions.v1.geojson \
  --row-shift 3824 \
  --current-regioninfo /home/carp/code/clones/shrddr.github.io/workerman/data/regioninfo.json \
  --regioninfo-bss /tmp/paz-region-meta/gamecommondata/binary/regioninfo.bss \
  --regionclientdata /tmp/paz-region-meta/gamecommondata/regionclientdata_ps_.xml \
  --regionclientdata /tmp/paz-region-meta/gamecommondata/regionclientdata_sa_.xml \
  --regionclientdata /tmp/paz-region-meta-more/gamecommondata/regionclientdata_en_.xml \
  --regionclientdata /tmp/paz-region-meta-more/gamecommondata/regionclientdata_na_.xml \
  --regionclientdata /tmp/paz-region-meta-more/gamecommondata/regionclientdata_dv_.xml \
  -o /tmp/pazifista-region-source-compare.json
```

Decode the validated `regioninfo.bss` row family directly and compare focus IDs
against the current external `regioninfo.json`:

```bash
devenv shell -- cargo run -q -p pazifista --bin pazifista -- \
  gcdata inspect-regioninfo-bss \
  /tmp/paz-region-meta/gamecommondata/binary/regioninfo.bss \
  --loc /home/carp/code/clones/shrddr.github.io/workerman/data/loc.json \
  --current-regioninfo /home/carp/code/clones/shrddr.github.io/workerman/data/regioninfo.json \
  --id 78 --id 226 --id 323 --id 820 --id 880 --id 1070 --id 1111 \
  --id 1144 --id 1150 --id 1211 --id 1406 \
  --id 1677 --id 1678 --id 1679 --id 1680 --id 1681 --id 1682 \
  --id 1683 --id 1684 --id 1685 --id 1686 --id 1687 --id 1688 \
  -o /tmp/pazifista-regioninfo-bss-focus.json
```

Decode `regiongroupinfo.bss` directly and compare it against the current
external `deck_rg_graphs.json`:

```bash
devenv shell -- cargo run -q -p pazifista --bin pazifista -- \
  gcdata inspect-regiongroupinfo-bss \
  /tmp/paz-region-meta/gamecommondata/binary/regiongroupinfo.bss \
  --current-deck-rg-graphs /home/carp/code/clones/shrddr.github.io/workerman/data/deck_rg_graphs.json \
  --id 177 --id 179 --id 295 \
  -o /tmp/pazifista-regiongroupinfo-bss-focus.json
```

Decode `mapdata_arraywaypoint.bin` and optionally sample the current external
waypoint coordinates against the decoded grid:

```bash
devenv shell -- cargo run -q -p pazifista --bin pazifista -- \
  gcdata inspect-arraywaypoint-bin \
  data/scratch/gamecommondata/waypoint_binary/mapdata_arraywaypoint.bin \
  --waypoints /home/carp/code/clones/shrddr.github.io/workerman/data/waypoints.json \
  --id 1739 --id 1746 --id 2051 --id 2052 \
  -o /tmp/pazifista-arraywaypoint.json \
  --preview-bmp /tmp/pazifista-arraywaypoint.bmp
```

Wrapper scripts under `tools/scripts/` chain that raw export through the
existing `fishystuff_ingest` enrichment step so the current map-facing GeoJSON
shape stays stable:

- `tools/scripts/rebuild_detailed_regions_layer_from_pabr.sh`
- `tools/scripts/rebuild_region_groups_vector_layer_from_pabr.sh`

## Map Integration Status

Current validated status for `regionmap_new.bmp.rid`:

- `region_groups` replacement is viable now
- exported `region_groups` GeoJSON matches the current layer's feature-count and
  region-group ID set: `240` features with the same `rg` values, including the
  existing `rg=0` catch-all feature
- exported `regions` GeoJSON does not match the current smoothed layer exactly:
  - RID dictionary IDs: `1264`
  - BKD-referenced IDs: `1264`
  - active positive-area PABR regions: `1253`
  - current smoothed regions: `1252`
  - current-only IDs: `78`, `226`, `323`, `820`, `880`, `1070`, `1111`,
    `1144`, `1150`, `1211`, `1406`
  - PABR-only IDs: `1677` through `1688`

Observed cause of the `regions` mismatch:

- the extra `1677..1688` IDs are active positive-area PABR regions and do not
  exist in the current `regioninfo.json`
- the `11` current-only IDs are not foreign to PAZ; they still exist in the
  RID dictionary and are still referenced by BKD breakpoints, but they collapse
  to zero positive area in the reconstructed native raster
- that means the real distinction is not "missing from PAZ" versus "present in
  GeoJSON", but "still active on the original raster" versus "retained as
  degenerate legacy IDs"

## Zone Mask Implication

No original asset equivalent to the current full-resolution community fish-zone
mask has been identified yet.

The current shipped zone-mask assets are both native map resolution:

- `data/cdn/public/images/zones_mask_v1.png`
  - PNG IHDR width `11560`, height `10540`
- `data/cdn/public/images/exact_lookup/zone_mask.v1.bin`
  - header magic `FSZLKP01`
  - width `11560`, height `10540`

`mapdata_arraywaypoint.bin` is structurally different:

- decoded size `2160 x 1968` microcells
- this is a sector-space grid, not a native map-space pixel image
- every microcell covers `1600 x 1600` world units
- it is therefore much coarser than the current `11560 x 10540` exact zone
  lookup

So the practical replacement path is now:

- `regions` / `region_groups`: can move toward original-data-derived geometry
- `zone_mask`: still needs either
  - discovery of the original fish-zone source asset, or
  - a repo-native semantic-mask format whose canonical source is not the
    current community `png + bin` pair
- `mapdata_arraywaypoint.bin` is still relevant, but as a separate original
  semantic raster rather than a drop-in replacement for the current zone-mask
  exact lookup

## Region ID Metadata Linkage

The current map pipeline does not attach a human-readable label directly from
the region ID `r`.

The linkage path in the existing enrichment pipeline is:

1. start with region ID `r`
2. look up `regioninfo.json[r]`
3. look up `deck_r_origins.json` row where `r == region_id`
4. resolve origin metadata in this order:
   - origin region from `deck_r_origins.o`
   - else origin region from `regioninfo.tradeoriginregion`
   - origin waypoint from `deck_r_origins.owp`
   - world position from `deck_r_origins.(x,z)`
5. resolve display name in this order:
   - `loc.en.node[origin_waypoint_id]`
   - else `loc.en.town[origin_region_id]`
   - else `loc.en.node[origin_region_id]`

This is implemented in:

- `tools/fishystuff_ingest/src/region_layers.rs`
  - `resolve_region_origin_info`
  - `resolve_origin_name`

Practical consequence:

- the `regions` layer property `on` is an origin/town/node label
- it is not a unique label for region ID `r`
- many distinct region IDs legitimately share the same `on` value

Examples from the current `regions.v1.geojson`:

- `"Ross Sea"` appears on `20` different region IDs
- `"Velia"` appears on `9` different region IDs
- `"Tarif"` appears on `9` different region IDs
- `"Duvencrune"` appears on `10` different region IDs
- `"Nampo's Moodle Village"` appears on `11` different region IDs

So the question "same label, different ID" must be interpreted carefully:

- it can mean a real ID rename
- but it can also mean multiple subregions share one origin label
- therefore label equality alone is not strong evidence of identity

## Current Mismatch Interpretation

For the current `regionmap_new` comparison:

- current-only region IDs:
  - `78`, `226`, `323`, `820`, `880`, `1070`, `1111`, `1144`, `1150`,
    `1211`, `1406`
- PABR-only region IDs:
  - `1677` through `1688`

`pazifista gcdata compare-region-sources` makes the metadata split explicit:

- RID dictionary IDs: `1264`
- BKD-referenced IDs: `1264`
- active positive-area PABR IDs: `1253`
- current `regions.v1.geojson` IDs: `1252`
- current external `regioninfo.json` entries: `1503`
- original `regioninfo.bss` header count: `1515`

Observed metadata status:

- all current-only IDs above exist in `regioninfo.json`
- all current-only IDs above also exist in the original RID dictionary and are
  still referenced by BKD rows
- all current-only IDs above disappear only at the last step, where the native
  PABR reconstruction yields zero positive area for them
- the PABR-only IDs `1677..1688` are active positive-area regions
- the PABR-only IDs `1677..1688` do not exist in the current `regioninfo.json`
- the `1515 - 1503 = 12` entry gap between original `regioninfo.bss` and the
  current external `regioninfo.json` matches the count of those active
  PABR-only IDs exactly

That is the strongest current evidence that the external `regioninfo.json` is
missing precisely the new active region IDs now visible in the original PAZ
assets.

## Partial `regioninfo.bss` Decode

`pazifista gcdata inspect-regioninfo-bss` now decodes a validated
signature-based row family directly from the original `regioninfo.bss`.

Current validated status:

- `regioninfo.bss` header entry count: `1515`
- decoded signature-family rows: `1297`
- undecoded remainder: `218`

For that decoded row family, the following fields are now validated:

- `key`
- `tradeoriginregion`
- `regiongroup`
- `waypoint` candidate
- origin label resolved from `loc.json` through `tradeoriginregion`

The warehouse and worker character-key fields have plausible candidate offsets
for some rows, but they are not treated as fully validated yet.

Most importantly for the current mismatch set:

- `1677..1688` all decode cleanly
- every one of those rows has:
  - `tradeoriginregion = 88`
  - `regiongroup = 295`
  - `waypoint = 2052`
  - origin label resolved from `loc.json` as `Olvia`
- `295` is a brand-new region-group ID from the original PAZ data
- current external `regioninfo.json` and current `region_groups/v1.geojson`
  both stop at max `regiongroup = 294`

That means the active new regions are not just unnamed region IDs. They also
introduce a new original region-group bucket that the current external metadata
does not know about.

## `regiongroupinfo.bss` Decode

`pazifista gcdata inspect-regiongroupinfo-bss` now decodes the original region
group table directly.

Current validated structure:

- `PABR` header with `u32` entry count
- fixed-size payload rows: `51` bytes each
- trailing `12`-byte PABR footer
- validated row fields:
  - `key` at row offset `0` as `u16`
  - `waypoint` at row offset `5` as unaligned `u32`
  - flag bytes at row offsets `9..11`
  - `graphx`, `graphy`, `graphz` at row offsets `12`, `16`, `20` as `f32`

Current validated counts:

- `regiongroupinfo.bss` header entry count: `243`
- decoded nonzero group rows: `242`
- blank placeholder row count: `1`
- current external `deck_rg_graphs.json` rows: `222`
- current-only external group IDs: none
- original-only group IDs: `16`, `25`, `113`, `134`, `177`, `179`, `180`,
  `221`, `242`, `243`, `244`, `245`, `255`, `283`, `284`, `285`, `286`,
  `287`, `288`, `295`

Important qualifier:

- most original-only group IDs above are blank placeholder rows with
  `waypoint = 0` and zero graph coordinates
- the current external `deck_rg_graphs.json` appears to have normalized those
  away instead of preserving the full original key space

But not all original-only groups are blank:

- `177` is waypoint-only in the original table:
  - `waypoint = 1746`
  - `loc.en.node[1746] = "Crow's Nest"`
  - current external `regioninfo.json` still references `regiongroup = 177`
    from live region IDs `983`, `985`, and `1071`
  - current `region_groups/v1.geojson` already contains an `rg = 177` polygon
- `179` is a fully populated original row:
  - `waypoint = 1739`
  - `graph = (-670204, -7838, -179627)`
  - `loc.en.node[1739] = "Papua Crinea"`
  - current external `regioninfo.json` still references `regiongroup = 179`
    from live region IDs `986`, `987`, `1015`, `1016`, `1017`, `1018`,
    `1019`
  - current `region_groups/v1.geojson` already contains an `rg = 179` polygon
- `295` is the new Olvia redesign group:
  - `waypoint = 2052`
  - `graph = (-114535, -2674, 157512)`
  - current external `deck_rg_graphs.json` does not contain `295`
  - current external `waypoints.json` stops at max key `2051`, so it also does
    not know waypoint `2052`
  - the decoded original `regioninfo.bss` rows for `1677..1688` all point to
    `regiongroup = 295`

This changes the interpretation of the external data gap:

- the external chain is stale for the new Olvia split, because it is missing
  both `regiongroup = 295` and `waypoint = 2052`
- the external chain is also already incomplete for existing live content,
  because it omits populated deck rows for `177` and `179` even though current
  region metadata and current group polygons still use those group IDs

## `mapdata_arraywaypoint.bin` Decode

The archive search found an original waypoint-adjacent asset:

- path: `gamecommondata/waypoint_binary/mapdata_arraywaypoint.bin`
- source archive: `pad06779.paz`
- extracted size: `8501784` bytes

`pazifista gcdata inspect-arraywaypoint-bin` now decodes its validated outer
structure.

### Header and Bounds

The first 24 bytes are 6 little-endian `i32` values:

- `min_x_sector = -159`
- `min_y_sector = -3`
- `min_z_sector = -87`
- `max_x_sector = 111`
- `max_y_sector = 6`
- `max_z_sector = 159`

These bounds match the repo world constants as a one-sector inset:

- repo map sector bounds: `x=[-160,112)`, `z=[-88,160)`
- arraywaypoint bounds: `x=[-159,111)`, `z=[-87,159)`
- alignment result: exactly `1` inset sector on every side

### Payload Layout

The remaining `8501760` payload bytes decode exactly as:

- `270 x 246` sector blocks
- `128` bytes per sector block
- `64` big-endian `u16` values per block
- interpreted as `8 x 8` microcells per sector

That yields a decoded grid of:

- width `2160`
- height `1968`
- microcell size `1600` world units, given `SECTOR_SCALE = 12800`

Validated storage order:

```text
for z_sector {
  for x_sector {
    for sub_x {
      for sub_z {
        u16be
      }
    }
  }
}
```

The decoder flips the reconstructed `z` axis into map-style top-down row order
for preview rendering and waypoint sampling.

### Value Distribution

The decoded grid has:

- `451` unique `u16` values
- `233` unique high bytes
- `28` unique low bytes

Dominant full values:

- `0023` -> `1670340` cells
- `ff22` -> `1019091`
- `0001` -> `674254`
- `0013` -> `514212`
- `0021` -> `114696`
- `ff20` -> `78604`

Dominant low bytes:

- `23`, `22`, `01`, `13`, `21`, `20`, `11`, `a1`, `41`, `51`, `71`

This strongly suggests a spatial classification or flag field with recurring
class codes, not a direct table of waypoint IDs.

### Block Uniformity

The grid is highly coarse-grained:

- total sector blocks: `66420`
- completely uniform `8 x 8` blocks: `48319`

Top uniform block values:

- `0023` -> `21662` whole blocks
- `ff22` -> `12317`
- `0001` -> `7981`
- `0013` -> `5868`

So much of the map is constant at sector-block granularity, which fits a
semantic mask interpretation better than a dense waypoint metadata table.

### Waypoint Sampling

Sampling the current external `waypoints.json` against the decoded grid gives:

- total current external waypoints: `1021`
- inside decoded bounds: `1021`
- unique sampled cell values across all waypoints: `21`

Most common sampled values:

- `0001` -> `346` waypoints
- `ff22` -> `239`
- `0023` -> `204`
- `0013` -> `130`
- `0021` -> `35`
- `ff20` -> `35`

Focus examples:

- `1739` (`Papua Crinea`) samples `0023`
- `1746` (`Crow's Nest`) samples `ff22`
- `2051` samples `0001`
- `2052` is absent from the current external `waypoints.json`, so no direct
  sampling is possible there

This is the decisive negative result:

- the file does cover the whole live waypoint footprint
- but many unrelated waypoints share the same cell values
- so the decoded values are not acting as direct waypoint keys

### Current Interpretation

`mapdata_arraywaypoint.bin` is no longer opaque, but it is also not the missing
`waypoint_id -> world position` table.

Current best interpretation:

- it is an original world-aligned semantic raster in sector space
- it is related to waypoint or world navigation semantics
- it may become useful for replacing or augmenting coarse map-semantic layers
- it does not by itself resolve the missing waypoint `2052` metadata chain

So the missing original-data hop remains:

- `regiongroup = 295`
- `waypoint = 2052`
- exact upstream name/position source still not decoded from PAZ-side data

For the legacy zero-area IDs that did decode in this family:

- `78 -> tradeoriginregion 202 -> Altinova`
- `226 -> 221 -> Tarif`
- `323 -> 5 -> Velia`
- `880 -> 873 -> Duvencrune`
- `1111 -> 1124 -> Eilton`
- `1144 -> 1124 -> Eilton`
- `1150 -> 1131 -> Mediah Northern Gateway`
- `1211 -> 1219 -> Nampo's Moodle Village`
- `1406 -> 1210 -> Dalbeol Village`

Two focus IDs were not found in the currently decoded signature family:

- `820`
- `1070`

So there is at least one additional `regioninfo.bss` row layout still to
decode. That second family is now the main blocker for turning the partial
decoder into a full replacement for the external `regioninfo.json`.

Observed `regionclientdata_*.xml` coverage from the extracted game install:

- `regionclientdata_dv_.xml`, `regionclientdata_en_.xml`, and
  `regionclientdata_sa_.xml` all contain `1070`, `1150`, and `1677..1688`
- `regionclientdata_na_.xml` and `regionclientdata_ps_.xml` contain none of
  those `14` IDs
- the remaining current-only legacy IDs
  `78`, `226`, `323`, `820`, `880`, `1111`, `1144`, `1211`, `1406` were not
  found in the tested `ps`, `sa`, `en`, `na`, or `dv` clientdata variants

Examples of current-only IDs and their resolved origin labels in the current
GeoJSON:

- `323 -> Velia`
- `226 -> Tarif`
- `820 -> Ross Sea`
- `1070 -> Ross Sea`
- `880 -> Duvencrune`
- `1111 -> Eilton`
- `1144 -> Eilton`
- `1211 -> Nampo's Moodle Village`
- `1406 -> Dalbeol Village`

These examples reinforce the ambiguity:

- the label comes from the resolved origin chain, not directly from the region
  ID
- multiple region IDs can share the same label while remaining distinct regions

Current working conclusion:

- `region_groups` can already be reconciled directly
- `regions` cannot be safely interpreted by label matching alone
- the original PAZ metadata now shows that the `11` current-only IDs are not
  missing from source assets; they are retained degenerate IDs with zero active
  raster area
- the next reliable step for naming `1677..1688` is decoding the original
  `regioninfo.bss` row layout rather than relying on the incomplete external
  `regioninfo.json`

## Geometry Matching Findings

`pazifista pabr match-regions` now rasterizes the current smoothed
`regions.v1.geojson` into the native PABR pixel grid and measures per-ID
overlap.

For `regionmap_new.bmp.rid` versus the current shipped
`data/cdn/public/region_groups/regions.v1.geojson`:

- PABR regions: `1253`
- current regions: `1252`
- overlap pairs recorded: `7032`
- mutual best matches: `1172`
- PABR-only IDs remain `1677..1688`
- current-only IDs remain `78`, `226`, `323`, `820`, `880`, `1070`, `1111`,
  `1144`, `1150`, `1211`, `1406`

### What the Matcher Resolves

The matcher is strong enough to separate "simple tiny legacy artifact" from
"real split/merge cluster".

Tiny current-only artifacts that map completely into another surviving PABR
region:

- `78 -> 81` with shared label `Altinova`, area `2`
- `1070 -> 950` with shared label `Ross Sea`, area `76`
- `1150 -> 1131` with shared label `Camp Balacs`, area `8`

These look like smoothed-layer crumbs rather than meaningful distinct regions.

### Zero-Area Current Features

Several current-only IDs cannot be geometry-matched because the current
smoothed GeoJSON collapses them to degenerate line or point shapes with zero
native-pixel area:

- `226` (`Tarif`)
- `323` (`Velia`)
- `820` (`Ross Sea`)
- `880` (`Duvencrune`)
- `1111` (`Eilton`)
- `1144` (`Eilton`)
- `1406` (`Dalbeol Village`)

These features still exist as IDs in the current GeoJSON, but geometric
matching alone cannot resolve them because there is no filled area left after
rasterization.

`1565` is an additional important edge case:

- it exists in both the current GeoJSON and the PABR source
- the current smoothed geometry collapses to zero raster area
- so it must not be treated as a PABR-only ID

### Olvia Split/Merge Cluster

The strongest new finding is that the `1677..1688` PABR-only IDs are not a
simple one-to-one rename against an old current ID.

Observed overlap:

- total area of PABR-only `1677..1688`: `21922`
- of that, `19754` pixels land inside current region `92` (`Olvia Coast`)
- `325` pixels land inside current region `88` (`Olvia`)

Current region `92` is therefore best described as a merged legacy polygon, not
as the direct successor of one PABR region:

- current `92` area: `26566`
- top PABR overlaps:
  - `1677`: `15581` pixels, `58.65%` of current `92`
  - `92`: `5990` pixels, `22.55%`
  - `1688`: `2019` pixels, `7.60%`

The only mutual best ID-change candidate found by the current matcher is:

- `current 92 (Olvia Coast) <-> PABR 1677`

Even that should be interpreted as "largest component inside a merged old
polygon", not as a confirmed rename.

Current practical interpretation:

- `1677..1688` are real PABR regions that the current smoothed GeoJSON mostly
  folds into older `Olvia Coast` and `Olvia` shapes
- the `11` current-only IDs are better understood as legacy IDs that survive in
  the PAZ-side tables and breakpoint stream but no longer own positive-area
  raster geometry
- `1070` and `1150` are confirmed by original `regionclientdata` variants,
  while the other nine legacy IDs still need a fuller PAZ-side row decode to
  explain exactly why they remain referenced

## Known Unknowns

Still not fully identified:

- the exact semantics of the unknown RID footer fields
- the exact meaning of the variable RID trailer prefix before the fixed footer
- why the source BKD row count is `1860` while the rendered native height is
  `10540`
- whether `3824` is universal for all PABR region-map assets or only for the
  currently validated family
- whether a reconstruction stricter than majority vote exists at band
  disagreement boundaries

## Practical Conclusion

For the validated region-map files, the current working interpretation is:

- RID = dictionary of region IDs plus native map dimensions
- BKD = sheared, wrapped breakpoint rows referencing that dictionary
- original region geometry can be reconstructed directly from `rid+bkd`

That is enough to replace GeoJSON as the source of truth for raster
reconstruction and to continue toward direct polygon extraction from the
original files.

## `2052` Chain Progress

The original-data chain for the new Olvia split is now closed far enough to
label the new region-group from original files without the stale external
`waypoints.json`.

### DBSS Extraction Finding

The earlier Rust extractor assumption for non-mobile archives was wrong for at
least part of the `*.dbss` family.

Validated with `pazifista archive inspect ... --raw-output`:

- `textbind.dbss`
- `textbindoffset.dbss`
- `dialogtext.dbss`
- `dialogtextoffset.dbss`
- `teleportoffset.dbss`

All of those are stored as raw on-disk payloads and should not be ICE-decrypted
by the normal extractor. `pazifista` now treats `*.dbss` as passthrough payloads
so these files extract byte-for-byte identical to the raw archive payload.

Useful structural notes:

- `textbindoffset.dbss` starts with a `u32` count and then `12`-byte records
  that behave like `hash, offset, length`
- `dialogtextoffset.dbss` follows the same count-plus-offset-table pattern
- `teleportoffset.dbss` starts with count `383` and then fixed `12`-byte
  records that behave like `key, offset, length`
- `teleport.dbss` starts with `u32 6`, `u32 383`, then `383` fixed-size rows,
  but that row layout is still undecoded

### `exploration.bss` Finding

`gamecommondata/binary/exploration.bss` is the first original table found so
far that directly contains the live waypoint IDs:

- `1739`
- `1746`
- `2051`
- `2052`

The focused rows for known waypoints strongly suggest the table stores both the
waypoint key and one or more shifted string IDs:

- waypoint `1739` carries repeated shifted ID `0x000c5a00`, which decodes to
  string ID `3162`
- waypoint `2052` carries repeated shifted ID `0x000cda00`, which decodes to
  string ID `3290`

That row family is real, but it is not the shortest route to names.

### `stringtable.bss` Finding

`stringtable.bss` now has a partially validated index interpretation.

At least one repeated record family is:

- `u32 hash`
- `u32 id_a`
- `u32 id_b`
- `u32 zero`

For the IDs reached from `exploration.bss`:

- string ID `3162` is in a record with paired ID `3163`
- string ID `3290` is in a record with paired ID `3291`

However, the focused strings reached that way are unrelated UI text:

- `3162/3163` resolve to `LUA_DONTUSE_PAINTINGPOINT`
- `3290/3291` resolve to `LUA_WORKERMANAGER_TOOLTIP_GO_WORLDMAP`

So the earlier `exploration.bss -> stringtable.bss` hop was a false lead for
the display name.

### `mapdata_realexplore*.xml` Finding

The authoritative waypoint-name source for this chain is the original waypoint
XML, not `stringtable.bss`.

Validated by extracting and inspecting:

- `gamecommondata/waypoint/mapdata_realexplore.xml`
- `gamecommondata/waypoint/mapdata_realexplore2.xml`

Focused original rows:

- `1739`
  - `mapdata_realexplore.xml`: `Name="field(papuacriny_island)"`
  - `mapdata_realexplore2.xml`: `Name="field(papuacriny_island)"`
- `1746`
  - `mapdata_realexplore.xml`: `Name="field(partrizio_island)"`
  - `mapdata_realexplore2.xml`: `Name="field(partrizio_island)"`
- `2052`
  - `mapdata_realexplore.xml`: `Name="town(olvia_academy)"`, `Pos=(-114942,-2674.33,157114)`
  - `mapdata_realexplore2.xml`: `Name="town(olvia_academy)"`, `Pos=(-125229,-2883.02,146801)`

That closes the original chain directly:

- `regiongroupinfo.bss`
  - `group 295`
  - `waypoint = 2052`
- `mapdata_realexplore.xml`
  - `Waypoint Key="2052"`
  - `Name="town(olvia_academy)"`

The `regiongroup 295` graph point from `regiongroupinfo.bss`
`(-114535,-2674,157512)` aligns closely with the `mapdata_realexplore.xml`
variant of `2052`, which strongly suggests that the group table is referencing
that variant rather than `mapdata_realexplore2.xml`.

### Practical Consequence

For the live Olvia redesign content:

- the missing external waypoint is not mysterious anymore
- the original canonical internal name is `town(olvia_academy)`
- the external chain is stale because it omitted a real original waypoint row,
  not because the original files lacked a label

This also makes `mapdata_realexplore*.xml` the best current PAZ-side candidate
for replacing the external `waypoints.json` layer entirely.

What has been ruled out so far:

- current external `waypoints.json`
- current external `loc.json`
- `mapdata_arraywaypoint.bin`
- `textbind*.dbss`
- `dialogtext*.dbss`
- `teleport*.dbss` as a direct carrier of waypoint IDs

## Localization Boundary Findings

The remaining gap is no longer structural linkage. It is only the last
human-facing localization hop from the original waypoint token to the final
display label.

Validated additional findings:

- `regionclientdata_en_.xml` and `regionclientdata_kr_.xml` do contain the new
  region IDs `1677..1688`, but the rows are just:
  - `<RegionInfo Key="...">`
  - repeated `<SpawnInfo ... dialogIndex="..." position="..."/>`
  - no region-name or town-name attribute was found in those files
- `regionmapinfo.bss` is a tiny `PABR` table with only `13` top-level rows and
  does not look like a region-name table
- `tooltiptable.dbss` and `tooltiptableoffset.dbss` were extracted as another
  original `PABR` family, but a quick scan did not reveal plain UTF-16
  occurrences of `Olvia`, `Papua`, or `Crow`
- `textbindoffset.dbss` is exactly `4 + count * 12` bytes and behaves like
  `hash, offset, length`, but sampled `textbind.dbss` chunks did not decode as
  zlib, raw-deflate, or gzip streams
- archive-name searches did not reveal any obvious original
  `*waypoint*name*`, `*region*name*`, `*town*name*`, or `*node*name*` table

One useful positive signal from original files:

- `gamecommondata/dialogscene/wharf/62500_wharf_olviaacademy.xml` starts with
  a Korean comment `올비아 아카데미 나루터 화면구성`
- so original assets do contain human-facing localized labels for the new
  content, but not yet through one decoded generic table that can replace the
  external localization JSONs end-to-end

Practical conclusion at this stage:

- original-file linkage is closed through
  `region -> regiongroup -> waypoint -> canonical token -> graph position`
- original-file localization is not yet fully closed through
  `canonical token -> final localized display label`

## `.loc` Breakthrough

The existing Python reader at
[read_loc.py](/home/carp/code/fishystuff/data/data/read_loc.py) was correct and
turned out to be the missing last hop for English display labels.

Validated `.loc` structure:

- `u32 expected_uncompressed_size`
- zlib-compressed payload
- repeated UTF-16LE records
- layout `A`
  - `u64 char_len`
  - `u64 key`
  - `utf16le text`
  - `u32 zero`
- layout `B`
  - `u32 char_len`
  - `u32 namespace`
  - `u64 key`
  - `utf16le text`
  - `u32 zero`

The reader is now ported into
[loc.rs](/home/carp/code/fishystuff/tools/pazifista/src/gcdata/loc.rs) and
available through:

```bash
devenv shell -- cargo run -q -p pazifista -- \
  gcdata inspect-loc data/data/languagedata_en.loc
```

Focused validated mappings from the real file:

- namespace `29`
  - `2052 -> Olvia Academy`
  - `1739 -> Papua Crinea`
  - `1746 -> Crow's Nest`
- namespace `17`
  - `88 -> Olvia`
  - `92 -> Olvia Coast`

That means the tested English original-data chain is now fully closed:

- region geometry
  - `rid+bkd`
- containing region or origin region linkage
  - `regioninfo.bss`
- region-group resource bar waypoint and graph position
  - `regiongroupinfo.bss`
- canonical waypoint token and position
  - `mapdata_realexplore*.xml`
- final English display label
  - `.loc`

This also explains why the canonical token strings such as
`town(olvia_academy)` or `field(papuacriny_island)` do not need to appear in
`.loc`: the `.loc` file is keyed numerically by namespace and ID, not by the
raw token text.
