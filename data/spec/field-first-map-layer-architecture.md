# Field-First Map Layer Architecture

This note documents the desired direction for map layers that carry discrete
semantic data such as:

- `zone_mask`
- `regions`
- `region_groups`
- future `rid+bkd`-derived semantic overlays

The goal is to stop treating these as unrelated pipelines and instead use one
shared model for:

- exact lookup
- hover and selection metadata
- clipping
- visual rendering

Related references:

- [pabr-region-map-format.md](/home/carp/code/fishystuff/data/spec/pabr-region-map-format.md)
- [world-map-sector-model.md](/home/carp/code/fishystuff/data/spec/world-map-sector-model.md)

## Motivation

The current runtime splits the problem into different one-off paths:

- `zone_mask`
  - exact lookup comes from a compact custom `.bin`
  - visible imagery comes from a separate bitmap/tile pipeline
  - naming comes from a separate zone metadata table
- `regions` and `region_groups`
  - visible geometry comes from vector GeoJSON
  - hover metadata comes from hard-coded GeoJSON properties like `on`, `owp`,
    `rgwp`, `ox`, `oz`, `rgx`, `rgz`

That works, but it has two structural problems:

1. it duplicates semantics and visuals across multiple artifacts
2. it does not scale cleanly to additional original-game semantic layers

The desired end state is:

- original files remain the source of truth
- the runtime consumes one canonical semantic field per layer
- hover, clipping, and display all derive from that same field

## Core Idea

For semantic layers, the canonical runtime artifact should be a compact field
asset, not a pre-rendered bitmap and not a vector overlay.

In this model:

- the field is the authoritative runtime representation
- metadata is stored separately by cell ID
- visible textures are generated from the field on demand

Examples:

- `zone_mask`
  - field cell ID: zone ID or current canonical zone RGB
- `regions`
  - field cell ID: region ID
- `region_groups`
  - field cell ID: region-group ID

This means the same layer can answer:

- "what cell am I hovering?"
- "what label should be shown?"
- "does this point pass the clip mask?"
- "what color should be rendered here?"

without needing separate, duplicated source artifacts.

## Design Goals

- Prefer original files over community-derived or stale external artifacts.
- Keep one canonical semantic source per layer.
- Make clipping native for any layer that exposes image/coverage data.
- Allow hover and selection UI to be driven by layer semantics, not hard-coded
  per-layer JSON property names.
- Avoid storing giant full-resolution rendered textures when the source field is
  already compact.
- Make future streaming possible, but do not require streaming for the first
  implementation.

## Proposed Runtime Model

Each semantic layer should provide three capabilities.

### 1. Field Sampling

The layer exposes a discrete field over canonical map pixels.

Conceptually:

```rust
trait LayerField {
    type CellId: Copy + Eq;

    fn sample_id_at_map_px(&self, x: i32, y: i32) -> Option<Self::CellId>;
    fn contains_at_map_px(&self, x: i32, y: i32) -> bool;
}
```

This is the core primitive for:

- hover
- selection
- exact lookup
- clipping

### 2. Semantics

The layer maps a sampled cell ID to user-facing information.

Conceptually:

```rust
trait LayerSemantics {
    type CellId: Copy + Eq;

    fn hover_rows(&self, id: Self::CellId, ctx: &HoverContext) -> Vec<HoverRow>;
    fn hover_targets(&self, id: Self::CellId, ctx: &HoverContext) -> Vec<HoverTarget>;
}
```

Examples:

- `regions`
  - `Origin: Grana`
- `region_groups`
  - `Resources: Southern Kamasylvia`
- `zone_mask`
  - `Zone: Coastal Shelf`

The important change is that the UI should consume generic rows and markers,
not fields hard-coded specifically for current region GeoJSON properties.

### 3. Visual Rendering

The layer can render visible pixels from the field plus a style table.

Conceptually:

```rust
trait LayerVisual {
    fn sample_rgba_at_map_px(&self, x: i32, y: i32) -> Option<[u8; 4]>;
}
```

This does not mean a full rendered image must be stored on disk.

Instead:

- the runtime keeps the semantic field in memory
- the runtime generates only the currently visible texture chunks

## Canonical Asset Split

For a semantic layer, the desired asset split is:

- original source files
  - for example `rid+bkd`, `regioninfo.bss`, `regiongroupinfo.bss`,
    `mapdata_realexplore.xml`, `.loc`
- canonical semantic field
  - one compact runtime field asset
- canonical metadata table
  - cell ID to labels, waypoints, graph positions, palette, and related data
- optional derived artifacts
  - vector polygons
  - pre-rendered visual tiles

The key point is:

- the field is canonical
- the visuals are derived

## Why Not Store Only a Bitmap

Because the bitmap duplicates information we already have in the field and is a
poor canonical representation for exact semantics.

For this repo's current map size:

- canonical map dimensions: `11560 x 10540`
- total pixels: `121,842,400`
- full raw RGB image at native size:
  - `365,527,200` bytes
- full raw RGBA image at native size:
  - `487,369,600` bytes

That is acceptable only as transient generated texture data, not as the
canonical layer representation.

## Size Notes From Current Assets

Current real files in this repo:

- [zone_mask.v1.bin](/home/carp/code/fishystuff/data/cdn/public/images/exact_lookup/zone_mask.v1.bin)
  - `1,790,476` bytes
- [zones_mask_v1.png](/home/carp/code/fishystuff/data/cdn/public/images/zones_mask_v1.png)
  - `1,472,702` bytes
- current `zone_mask` visual tile set under
  [zone_mask_visual/v1](/home/carp/code/fishystuff/data/cdn/public/images/tiles/zone_mask_visual/v1)
  - `3,476,127` bytes total PNG payload
- [regionmap_new.bmp.bkd](/home/carp/code/fishystuff/data/scratch/ui_texture/minimap/area/regionmap_new.bmp.bkd)
  - `1,808,420` bytes
- [regionmap_new.bmp.rid](/home/carp/code/fishystuff/data/scratch/ui_texture/minimap/area/regionmap_new.bmp.rid)
  - `2,587` bytes
- [regions.v1.geojson](/home/carp/code/fishystuff/data/cdn/public/region_groups/regions.v1.geojson)
  - `10,106,053` bytes
- [v1.geojson](/home/carp/code/fishystuff/data/cdn/public/region_groups/v1.geojson)
  - `3,412,400` bytes

Practical conclusion:

- loading the full semantic field for `zone_mask`, `regions`, or
  `region_groups` is cheap enough
- loading a full rendered image for those layers is not

So the first implementation should assume:

- full semantic field in memory
- generated visible texture chunks
- no full monolithic texture upload

## Visual Strategy

For semantic layers, the desired visual path is:

1. load the canonical field asset
2. sample visible chunk bounds in canonical map pixels
3. rasterize those chunks into RGBA buffers using the style table
4. upload only those generated chunks as textures

This avoids storing a second canonical bitmap while still giving the renderer
the textures it needs.

Important distinction:

- runtime-generated texture chunks are necessary for display
- pre-baked visual PNG tiles are optional

## Clipping Strategy

Clipping should become a native feature of any layer that exposes coverage or
image data.

Default clip semantics should be:

- coverage clip
  - `contains_at_map_px`
- exact discrete-ID clip
  - `sample_id_at_map_px` plus predicate
- alpha clip
  - `sample_rgba_at_map_px(...)[3] > 0`

This means:

- `zone_mask` can clip other layers
- `regions` can clip other layers
- `region_groups` can clip other layers
- future semantic layers can clip other layers without custom one-off code

## Layer Expectations

### `minimap`

`minimap` should remain a tiled raster layer.

It is a large continuous image overlay and already fits the raster-tile model
well.

### `zone_mask`

`zone_mask` should move toward the field-first model:

- canonical semantic source
  - current bitmap-derived field for now
  - original source later, if discovered
- naming
  - metadata table
- visuals
  - generated from the field on demand

### `regions`

`regions` should use:

- canonical field
  - derived from original `rid+bkd`
- metadata
  - derived from original region metadata files
- visuals
  - generated from field cell IDs and palette rules

Vector GeoJSON can remain as a derived export for:

- debugging
- analysis
- alternate presentation modes

but it should not remain the canonical hover/clip source.

### `region_groups`

`region_groups` should follow the same pattern as `regions`:

- canonical field
  - derived from original `rid+bkd` plus original region-group mapping
- metadata
  - derived from original group metadata
- visuals
  - generated from field cell IDs and palette rules

## Streaming

Streaming is a plausible later optimization, but should not block the first
field-first implementation.

Recommended implementation order:

1. non-streamed field asset
   - load the full semantic field into memory
2. generated visible texture chunks
3. chunk-addressable streamed field format if bandwidth becomes worth
   optimizing

If streaming is added later, the preferred streamed object is the field itself,
not pre-rendered visual PNG tiles.

That would allow:

- fetching only the needed semantic chunks
- deriving both hover/clipping and visuals from the same fetched data
- avoiding a second semantically redundant transport format

## Recommended Rollout

1. Generalize the current exact-lookup row-span model into a reusable semantic
   field type.
2. Replace hard-coded region/region-group hover-property extraction with
   generic hover rows and hover targets derived from semantics.
3. Move clip-mask evaluation to the same field-sampling model.
4. Build canonical field plus metadata assets for:
   - `zone_mask`
   - `regions`
   - `region_groups`
5. Add runtime-generated texture chunks for semantic layers.
6. Keep pre-rendered PNG tiles optional and minimap-specific unless later
   measurements justify them elsewhere.

## Non-Goals

This note does not require:

- immediate removal of all vector GeoJSON artifacts
- immediate streaming support
- reconstructing fake `rid+bkd` for assets that did not originally use that
  format

The immediate target is a shared runtime architecture that can treat semantic
layers consistently, regardless of which original file family they came from.
