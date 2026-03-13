#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT_DIR"

MAP_VERSION="${MAP_VERSION:-v1}"
RAW_WATERMAP="${1:-zonegen/images/watermap.png}"
TILES_OUT_DIR="${2:-zonegen/images/tiles/water/${MAP_VERSION}/0}"
PROJECTED_WATERMAP="${3:-zonegen/images/watermap_projected_${MAP_VERSION}.png}"

MAP_EXPECTED_WIDTH=11560
MAP_EXPECTED_HEIGHT=10540
TILE_SIZE=512
TILESET_JSON="$(dirname "${TILES_OUT_DIR}")/tileset.json"

# water source pixel -> canonical map pixel (from DB affine fit)
MAP_FROM_WATER_A="${MAP_FROM_WATER_A:-1.659485954446}"
MAP_FROM_WATER_D="${MAP_FROM_WATER_D:-1.662131049737}"
MAP_FROM_WATER_TX="${MAP_FROM_WATER_TX:-2.028836685947}"
MAP_FROM_WATER_TY="${MAP_FROM_WATER_TY:--6.184779503586}"

if [[ ! -f "${RAW_WATERMAP}" ]]; then
  echo "Missing raw watermap: ${RAW_WATERMAP}" >&2
  exit 1
fi

INV_SX="$(python - <<PY
a = float("${MAP_FROM_WATER_A}")
print(f"{1.0/a:.15f}")
PY
)"
INV_SY="$(python - <<PY
d = float("${MAP_FROM_WATER_D}")
print(f"{1.0/d:.15f}")
PY
)"
INV_OX="$(python - <<PY
a = float("${MAP_FROM_WATER_A}")
tx = float("${MAP_FROM_WATER_TX}")
print(f"{(-tx/a):.15f}")
PY
)"
INV_OY="$(python - <<PY
d = float("${MAP_FROM_WATER_D}")
ty = float("${MAP_FROM_WATER_TY}")
print(f"{(-ty/d):.15f}")
PY
)"

echo "Projecting watermap to canonical map-space: ${PROJECTED_WATERMAP}"
cargo run --manifest-path "$ROOT_DIR/Cargo.toml" -p fishystuff_ingest -- debug-watermap-projection \
  --watermap="${RAW_WATERMAP}" \
  --out="${PROJECTED_WATERMAP}" \
  --projection-mode=rgb \
  --watermap-transform=scale_offset \
  --watermap-sx="${INV_SX}" \
  --watermap-sy="${INV_SY}" \
  --watermap-ox="${INV_OX}" \
  --watermap-oy="${INV_OY}"

echo "Tiling projected watermap from: ${PROJECTED_WATERMAP}"
echo "Regenerating water tiles in: ${TILES_OUT_DIR}"
rm -rf "${TILES_OUT_DIR}"
cargo run --manifest-path "$ROOT_DIR/Cargo.toml" -p fishystuff_tilegen --bin fishystuff_tilegen -- \
  --input "${PROJECTED_WATERMAP}" \
  --out-dir "${TILES_OUT_DIR}" \
  --tile-size "${TILE_SIZE}" \
  --expect-width "${MAP_EXPECTED_WIDTH}" \
  --expect-height "${MAP_EXPECTED_HEIGHT}"

cat > "${TILESET_JSON}" <<'JSON'
{
  "tile_size_px": 512,
  "levels": [
    {
      "z": 0,
      "min_x": 0,
      "min_y": 0,
      "width": 23,
      "height": 21,
      "tile_count": 483,
      "occupancy_b64": "////////////////////////////////////////////////////////////////////////////////Bw=="
    }
  ]
}
JSON

echo "Done."
echo "Raw watermap: ${RAW_WATERMAP}"
echo "Projected watermap: ${PROJECTED_WATERMAP}"
echo "Tiles directory: ${TILES_OUT_DIR}"
echo "Tileset manifest: ${TILESET_JSON}"
