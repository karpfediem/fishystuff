#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT_DIR"

MAP_VERSION="${MAP_VERSION:-v1}"
PABR_INPUT="${1:-$ROOT_DIR/data/scratch/ui_texture/minimap/area/regionmap_new.bmp.rid}"
REGIONINFO_JSON="${2:-/home/carp/code/clones/shrddr.github.io/workerman/data/regioninfo.json}"
LOC_JSON="${3:-/home/carp/code/clones/shrddr.github.io/workerman/data/loc.json}"
DECK_R_ORIGINS_JSON="${4:-/home/carp/code/clones/shrddr.github.io/workerman/data/deck_r_origins.json}"
DECK_RG_GRAPHS_JSON="${5:-/home/carp/code/clones/shrddr.github.io/workerman/data/deck_rg_graphs.json}"
CDN_ROOT="${CDN_ROOT:-$ROOT_DIR/data/cdn/public}"
OUT_GEOJSON="${6:-$CDN_ROOT/region_groups/regions.${MAP_VERSION}.geojson}"

for required in \
  "${PABR_INPUT}" \
  "${REGIONINFO_JSON}" \
  "${LOC_JSON}" \
  "${DECK_R_ORIGINS_JSON}" \
  "${DECK_RG_GRAPHS_JSON}"
do
  if [[ ! -f "${required}" ]]; then
    echo "Missing input: ${required}" >&2
    exit 1
  fi
done

mkdir -p "$(dirname "${OUT_GEOJSON}")"
TMP_GEOJSON="$(mktemp)"
cleanup() {
  rm -f "${TMP_GEOJSON}"
}
trap cleanup EXIT

echo "Exporting raw regions from PABR: ${PABR_INPUT}"
cargo run --manifest-path "$ROOT_DIR/Cargo.toml" -p pazifista --bin pazifista -- \
  pabr export-regions-geojson \
  "${PABR_INPUT}" \
  --output "${TMP_GEOJSON}"

echo "Enriching detailed regions into: ${OUT_GEOJSON}"
cargo run --manifest-path "$ROOT_DIR/Cargo.toml" -p fishystuff_ingest -- \
  build-detailed-regions-geojson \
  --regions-geojson "${TMP_GEOJSON}" \
  --regioninfo "${REGIONINFO_JSON}" \
  --loc "${LOC_JSON}" \
  --deck-r-origins "${DECK_R_ORIGINS_JSON}" \
  --deck-rg-graphs "${DECK_RG_GRAPHS_JSON}" \
  --out "${OUT_GEOJSON}"

echo "Done."
echo "Detailed regions GeoJSON: ${OUT_GEOJSON}"
