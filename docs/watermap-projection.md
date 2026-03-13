# Watermap Map-space Projection

Water now follows the same map-space model as zone mask / region-groups overlays:

- water tiles are generated in canonical map pixels (`11560x10540`)
- runtime `layers.transform_kind` for `water` is `identity_map_space`
- no runtime affine fitting for water overlay rendering

## Inputs

- Raw watermap image: `data/imagery/watermap.png` (`7168x6144`)
- Projection coefficients (defaulted in script):
  - `map_x = a * water_x + tx`
  - `map_y = d * water_y + ty`
  - defaults:
    - `a=1.659485954446`
    - `d=1.662131049737`
    - `tx=2.028836685947`
    - `ty=-6.184779503586`

## One-command Script

Run from the repo root:

```bash
tools/scripts/rebuild_water_overlay.sh
```

Optional arguments:

```bash
tools/scripts/rebuild_water_overlay.sh <raw_watermap.png> <tiles_out_dir> <projected_mapspace_png>
```

What the script does:
1. Projects raw watermap into canonical map-space image (`11560x10540`)
2. Tiles projected image to `data/scratch/water/<map_version>/0`
3. Writes a level-0 full occupancy tileset manifest (`23x21`, `483` tiles)

## Validation Checklist

1. `data/scratch/water/v1/0` exists and contains `483` tiles (`23 x 21`).
2. `data/scratch/water/v1/tileset.json` exists.
3. `layers` row for `water` uses `identity_map_space` and affine columns are null.
4. In UI, water overlay stays locked with zone mask/region groups during pan/zoom.

## Commit Policy

- Do not commit generated water tiles unless explicitly requested.
