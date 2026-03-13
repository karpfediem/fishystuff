# Terrain Pyramid Pipeline

## Rationale

- Authoritative terrain input is the full-resolution source tile set under:
  - `data/terrain/Karpfen/terraintiles/whole_fullres.png`
- That image is tiled offline before chunk-pyramid generation; the derived tile directory is an implementation detail, not the source of truth.
- `whole.webp` is obsolete for terrain runtime and bake inputs.
- Runtime Terrain3D must stream chunked LOD assets, not decode source RGB24 tiles in-browser.

## Geometry assets

- Offline bake command: `build-terrain-pyramid` (`fishystuff_tilegen --bin terrain_pyramid`)
- Output:
  - `site/assets/images/terrain/<revision>/manifest.json`
  - `site/assets/images/terrain/<revision>/levels/<level>/<x>_<y>.thc`
- Chunk encoding:
  - fixed grid (`grid_size`, default 65x65)
  - `u16` normalized heights (`u16_norm`)
  - world Y reconstruction:
    - `h = bbox_y_min + (u16 / 65535.0) * (bbox_y_max - bbox_y_min)`
- Manifest carries occupancy bitsets per level to avoid blind 404 probes.

## Source decode

- Source tiles are packed RGB24 scalar heights:
  - `packed = (R << 16) | (G << 8) | B`
  - `hNorm = packed / 16777215.0`
- Resampling is performed in scalar space with bilinear interpolation.
- Coarser levels are generated from finer scalar chunk data, not from RGB bytes.

## Drape assets

- Offline bake command: `build-terrain-drape-pyramid`
- Output:
  - `site/assets/images/terrain_drape/<layer>/<revision>/manifest.json`
  - `site/assets/images/terrain_drape/<layer>/<revision>/levels/<level>/<x>_<y>.png`
- Drape chunks are chunk-aligned to terrain chunk boundaries and level hierarchy.

## Runtime behavior

- Terrain3D loads a terrain manifest, then streams chunk files incrementally.
- Visible LOD selection uses distance-based target level plus ancestor fallback.
- Coarse levels are pinned in cache to prevent gaps during fast zoom-out.
- 2D map mode remains isolated from Terrain3D chunk caches/transforms.
