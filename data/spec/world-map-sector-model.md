# World Map Sector Model

This note documents how the repo appears to handle world-map sectors in
general, across raster overlays, vector geometry, and original-game assets.

## Canonical Domain

The shared constants live in
`lib/fishystuff_core/src/constants.rs`:

- `MAP_WIDTH = 11560`
- `MAP_HEIGHT = 10540`
- `LEFT = -160`
- `RIGHT = 112`
- `BOTTOM = -88`
- `TOP = 160`
- `SECTOR_SCALE = 12800`
- `SECTOR_PER_PIXEL = 0.0235294122248888`
- `DEFAULT_PIXEL_CENTER_OFFSET = 1.0`

Interpretation:

- the canonical map spans `272` sectors horizontally
  - `RIGHT - LEFT = 112 - (-160) = 272`
- the canonical map spans `248` sectors vertically
  - `TOP - BOTTOM = 160 - (-88) = 248`
- one sector corresponds to `12800` world units
- the full canonical world bounds are:
  - `world_x in [-2048000, 1433600]`
  - `world_z in [-1126400, 2048000]`

The map behaves as if sector space is the primary coordinate system, and world
coordinates are just sector coordinates scaled by `12800`.

## Axes and Orientation

There are two main planar spaces in use:

1. map-pixel space
   - origin is top-left
   - `x` grows right
   - `y` grows down

2. world space
   - `x` grows right/east
   - `z` grows up/north

So the vertical axis is inverted between map pixels and world coordinates:

- larger map `y` means smaller world `z`
- top of the map is large positive `z`
- bottom of the map is large negative `z`

This is visible in the shared transforms in
`lib/fishystuff_core/src/coord.rs` and
`map/fishystuff_ui_bevy/src/map/spaces/world.rs`.

## Core Transform

The shared pixel-to-world transform is:

```text
world_x = (px * SECTOR_PER_PIXEL + LEFT) * SECTOR_SCALE
world_z = (-(py + pixel_center_offset) * SECTOR_PER_PIXEL + TOP) * SECTOR_SCALE
```

The inverse is:

```text
px = ((world_x / SECTOR_SCALE) - LEFT) / SECTOR_PER_PIXEL
py = ((TOP - (world_z / SECTOR_SCALE)) / SECTOR_PER_PIXEL) - pixel_center_offset
```

Default `pixel_center_offset = 1.0`.

The repo treats this as an affine transform, not as an ad hoc per-layer
special case. The same model is used in:

- `lib/fishystuff_core/src/coord.rs`
- `lib/fishystuff_core/src/transform.rs`
- `map/fishystuff_ui_bevy/src/map/spaces/world.rs`
- `tools/fishystuff_tilegen/src/bin/minimap_display_tiles.rs`

Equivalent affine form:

```text
world_x = map_x * (SECTOR_PER_PIXEL * SECTOR_SCALE) + LEFT * SECTOR_SCALE
world_z = map_y * -(SECTOR_PER_PIXEL * SECTOR_SCALE)
        + (TOP - pixel_center_offset * SECTOR_PER_PIXEL) * SECTOR_SCALE
```

With current constants:

- one map pixel is about `301.176476` world units
- one map pixel is about `0.0235294122` sectors

The exact sector span divided by pixel dimensions is
`272 / 11560 = 248 / 10540 = 0.0235294117647059`, which differs from the
stored `SECTOR_PER_PIXEL` constant by only about `4.6e-10`.

## Pixel-Center Convention

The repo does not use a `0.5` pixel-center convention here. It uses
`DEFAULT_PIXEL_CENTER_OFFSET = 1.0`.

Practical consequence:

- the world point associated with map pixel `(px, py)` is treated as the bottom
  edge of that pixel row, not the geometric center at `py + 0.5`
- all code using the shared transform stays internally consistent because both
  forward and inverse paths use the same offset

This matters when comparing repo-native assets against external imagery or
game-native grids. If something looks vertically shifted by about one pixel,
this offset is one of the first things to check.

## Geometry Spaces

The API/runtime distinguishes between two geometry spaces in
`lib/fishystuff_api/src/models/layers.rs`:

- `map_pixels`
  - feature coordinates are canonical map pixels and must be projected through
    the shared `MapToWorld` transform
- `world`
  - feature coordinates are already in world coordinates
  - GeoJSON stores them as `(x = world_x, y = world_z)`

In practice:

- many raster assets are produced or reprojected into canonical map pixels
  (`11560x10540`)
- region and region-group vector layers are treated as world-space geometry
- hover, camera, and selection logic in the Bevy map runtime use world-space
  coordinates directly

So there is one canonical planar world model, but assets can enter either as
map pixels or already-projected world coordinates.

## Raster Implications

Canonical repo-native raster overlays are generally expected to align to the
full canonical map image:

- `11560 x 10540`
- same pixel-to-world transform as above

Examples:

- community zone mask PNG
- exact zone lookup rows derived from that PNG
- projected water overlay
- minimap display tiling pipeline

This is why many tools validate exact dimensions against `MAP_WIDTH` and
`MAP_HEIGHT`.

## Sector-Space Assets Versus Map-Space Assets

Not every original-game raster is stored at canonical map resolution.

The new `mapdata_arraywaypoint.bin` decoder shows a second pattern:

- bounds stored directly in sector coordinates
- content stored as sector blocks and sub-sector microcells
- no native `11560 x 10540` map image involved

Validated current example:

- sector bounds: `x=[-159,111)`, `z=[-87,159)`
- this is a one-sector inset from the repo full bounds
- block grid: `270 x 246` sectors
- sub-grid: `8 x 8` microcells per sector
- resulting decoded grid: `2160 x 1968`
- microcell size: `12800 / 8 = 1600` world units

So the repo needs to handle at least two kinds of spatial assets:

1. full map-space rasters
   - canonical pixel grid
   - usually `11560 x 10540`

2. sector-native grids
   - explicit sector bounds
   - coarser sub-sector sampling
   - require a separate sector-grid decoder before they can be compared to map
     pixels

## Half-Open Bounds

The code consistently behaves as if sector bounds are half-open:

- horizontal extent: `x in [LEFT, RIGHT)`
- vertical extent: `z in [BOTTOM, TOP)`

That matches:

- map width from `RIGHT - LEFT`
- map height from `TOP - BOTTOM`
- sector-grid handling in the `arraywaypoint` decoder

This also matches how integer sector indices are derived:

- `sector_x = floor(world_x / SECTOR_SCALE)`
- `sector_z = floor(world_z / SECTOR_SCALE)`

Points at the upper boundary belong to the next excluded sector, not the last
included one.

## Practical Rules

When working on map assets in this repo, the safest assumptions are:

- sector space is the canonical logical domain
- world space is sector space multiplied by `12800`
- map pixels are just a fixed affine view of that domain
- top-left pixel origin implies inverted vertical axis relative to world `z`
- vector geometry in `world` space should stay there unless there is a real
  need to rasterize it
- full-resolution overlay assets should match `11560 x 10540`
- original-game assets may instead declare explicit sector bounds and need
  dedicated decoders rather than being forced into the PNG-sized model

## Current Confidence

This sector model is strongly supported by:

- shared core constants and transform helpers
- Bevy runtime `MapToWorld`
- vector layer geometry-space handling
- tile generation affine setup
- zone-mask and water overlay pipelines
- the decoded `mapdata_arraywaypoint.bin` sector bounds and block layout

What is still unknown:

- whether every original-game map-adjacent asset uses exactly the same sector
  bounds and pixel-center convention
- whether the `SECTOR_PER_PIXEL` constant originated from a source more precise
  than the simple span divided by image dimensions

But the general repo-level model is clear enough to treat the above as the
current canonical interpretation.
