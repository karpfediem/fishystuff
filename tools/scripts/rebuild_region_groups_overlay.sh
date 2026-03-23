#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT_DIR"

MAP_VERSION="${MAP_VERSION:-v1}"
GEOJSON="${1:-$ROOT_DIR/data/cdn/public/region_groups/${MAP_VERSION}.geojson}"
CDN_ROOT="${CDN_ROOT:-$ROOT_DIR/data/cdn/public}"
TILES_OUT_DIR="${2:-$CDN_ROOT/images/tiles/region_groups/${MAP_VERSION}/0}"
TILESET_JSON="${3:-$CDN_ROOT/images/tiles/region_groups/${MAP_VERSION}/tileset.json}"
TILE_SIZE="${TILE_SIZE:-512}"

if [[ ! -f "${GEOJSON}" ]]; then
  echo "Missing GeoJSON: ${GEOJSON}" >&2
  exit 1
fi

echo "Rasterizing region groups from: ${GEOJSON}"
echo "Regenerating region-group tiles in: ${TILES_OUT_DIR}"
rm -rf "${TILES_OUT_DIR}"
mkdir -p "$(dirname "${TILES_OUT_DIR}")"

cargo run --manifest-path "$ROOT_DIR/Cargo.toml" -p fishystuff_tilegen --bin region_groups_raster -- \
  --geojson "${GEOJSON}" \
  --out-dir "${TILES_OUT_DIR}" \
  --tileset-out "${TILESET_JSON}" \
  --tile-size "${TILE_SIZE}" \
  --map-width 11560 \
  --map-height 10540 \
  --alpha 255

echo "Done."
echo "Tiles directory: ${TILES_OUT_DIR}"
echo "Tileset manifest: ${TILESET_JSON}"
