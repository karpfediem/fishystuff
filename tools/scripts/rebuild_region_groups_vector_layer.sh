#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT_DIR"

MAP_VERSION="${MAP_VERSION:-v1}"
REGION_GROUPS_GEOJSON="${1:-/home/carp/code/clones/shrddr.github.io/workerman/data/rg_latest_1_5.geojson}"
REGIONINFO_JSON="${2:-/home/carp/code/clones/shrddr.github.io/workerman/data/regioninfo.json}"
LOC_JSON="${3:-/home/carp/code/clones/shrddr.github.io/workerman/data/loc.json}"
DECK_R_ORIGINS_JSON="${4:-/home/carp/code/clones/shrddr.github.io/workerman/data/deck_r_origins.json}"
DECK_RG_GRAPHS_JSON="${5:-/home/carp/code/clones/shrddr.github.io/workerman/data/deck_rg_graphs.json}"
CDN_ROOT="${CDN_ROOT:-$ROOT_DIR/data/cdn/public}"
OUT_GEOJSON="${6:-$CDN_ROOT/region_groups/${MAP_VERSION}.geojson}"

for required in \
  "${REGION_GROUPS_GEOJSON}" \
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

echo "Building region-group vector layer from: ${REGION_GROUPS_GEOJSON}"
echo "Writing region-group vector layer to: ${OUT_GEOJSON}"

cargo run --manifest-path "$ROOT_DIR/Cargo.toml" -p fishystuff_ingest -- \
  build-region-groups-geojson \
  --region-groups-geojson "${REGION_GROUPS_GEOJSON}" \
  --regioninfo "${REGIONINFO_JSON}" \
  --loc "${LOC_JSON}" \
  --deck-r-origins "${DECK_R_ORIGINS_JSON}" \
  --deck-rg-graphs "${DECK_RG_GRAPHS_JSON}" \
  --out "${OUT_GEOJSON}"

echo "Done."
echo "Region groups GeoJSON: ${OUT_GEOJSON}"
