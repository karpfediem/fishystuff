# PABR Region Map Format

This note documents the `PABR`-family `*.bmp.rid` and `*.bmp.bkd` files used
for Black Desert minimap region maps.

Status:

- this is a reverse-engineered format note, not an official vendor spec
- the geometry and reconstruction path below are validated against the current
  `pazifista` implementation
- some trailer fields are still unidentified

Current implementation:

- parser and renderer: [tools/pazifista/src/pabr.rs](/home/carp/code/fishystuff/tools/pazifista/src/pabr.rs)
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
