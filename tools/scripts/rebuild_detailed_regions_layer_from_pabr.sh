#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT_DIR"

MAP_VERSION="${MAP_VERSION:-v1}"
PABR_INPUT="${1:-$ROOT_DIR/data/scratch/ui_texture/minimap/area/regionmap_new.bmp.rid}"
REGIONINFO_BSS="${2:-$ROOT_DIR/data/scratch/gamecommondata/binary/regioninfo.bss}"
REGIONGROUPINFO_BSS="${3:-$ROOT_DIR/data/scratch/gamecommondata/binary/regiongroupinfo.bss}"
LOC_PATH="${4:-$ROOT_DIR/data/data/languagedata_en.loc}"
WAYPOINT_XML_PRIMARY="${5:-$ROOT_DIR/data/scratch/gamecommondata/waypoint/mapdata_realexplore.xml}"
WAYPOINT_XML_SECONDARY="${6:-$ROOT_DIR/data/scratch/gamecommondata/waypoint/mapdata_realexplore2.xml}"
CDN_ROOT="${CDN_ROOT:-$ROOT_DIR/data/cdn/public}"
OUT_GEOJSON="${7:-$CDN_ROOT/region_groups/regions.${MAP_VERSION}.geojson}"

for required in \
  "${PABR_INPUT}" \
  "${REGIONINFO_BSS}" \
  "${REGIONGROUPINFO_BSS}" \
  "${LOC_PATH}" \
  "${WAYPOINT_XML_PRIMARY}" \
  "${WAYPOINT_XML_SECONDARY}"
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
  --regioninfo-bss "${REGIONINFO_BSS}" \
  --regiongroupinfo-bss "${REGIONGROUPINFO_BSS}" \
  --loc "${LOC_PATH}" \
  --waypoint-xml "${WAYPOINT_XML_PRIMARY}" \
  --waypoint-xml "${WAYPOINT_XML_SECONDARY}" \
  --out "${OUT_GEOJSON}"

echo "Done."
echo "Detailed regions GeoJSON: ${OUT_GEOJSON}"
