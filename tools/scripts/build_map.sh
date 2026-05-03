#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT_DIR"

: "${CARGO_HOME:=/tmp/cargo}"
export CARGO_HOME

wasm_rustflags='--cfg getrandom_backend="wasm_js"'
if [ -n "${RUSTFLAGS:-}" ]; then
  wasm_rustflags="${RUSTFLAGS} ${wasm_rustflags}"
fi

resolve_map_runtime_manifest_cache_key() {
  local cache_key="${FISHYSTUFF_RUNTIME_MAP_ASSET_CACHE_KEY:-}"
  if [ -z "$cache_key" ]; then
    if git -C "$ROOT_DIR" rev-parse HEAD >/dev/null 2>&1; then
      cache_key="$("$ROOT_DIR/tools/scripts/resolve_map_runtime_cache_key.sh")"
    else
      cache_key="$(date -u +%Y%m%dT%H%M%SZ)"
    fi
  fi

  cache_key="$(printf '%s' "$cache_key" | tr -cs 'A-Za-z0-9._-' '-' | sed -E 's/^-+//; s/-+$//')"
  if [ -z "$cache_key" ]; then
    cache_key="$(date -u +%Y%m%dT%H%M%SZ)"
  fi

  printf '%s\n' "$cache_key"
}

PROFILE="${FISHYSTUFF_WASM_PROFILE:-release}"
MAP_RUNTIME_MANIFEST_CACHE_KEY="$(resolve_map_runtime_manifest_cache_key)"
MAP_RUNTIME_MANIFEST_FILE="runtime-manifest.${MAP_RUNTIME_MANIFEST_CACHE_KEY}.json"
if [ "$PROFILE" = "release" ]; then
  RUSTFLAGS="$wasm_rustflags" cargo build --manifest-path "$ROOT_DIR/Cargo.toml" -p fishystuff_ui_bevy --target wasm32-unknown-unknown --release
  WASM_INPUT="target/wasm32-unknown-unknown/release/fishystuff_ui_bevy.wasm"
else
  RUSTFLAGS="$wasm_rustflags" cargo build --manifest-path "$ROOT_DIR/Cargo.toml" -p fishystuff_ui_bevy --target wasm32-unknown-unknown
  WASM_INPUT="target/wasm32-unknown-unknown/debug/fishystuff_ui_bevy.wasm"
fi

SITE_MAP_ASSET_DIR="$ROOT_DIR/site/assets/map"
CDN_ROOT="${CDN_ROOT:-$ROOT_DIR/data/cdn/public}"
CDN_MAP_ASSET_DIR="$CDN_ROOT/map"
CDN_IMAGE_ASSET_DIR="$CDN_ROOT/images"
CDN_FIELD_ASSET_DIR="$CDN_ROOT/fields"
CDN_WAYPOINT_ASSET_DIR="$CDN_ROOT/waypoints"
CDN_HOTSPOT_ASSET_DIR="$CDN_ROOT/hotspots"
MINIMAP_SOURCE_TILE_DIR="${MINIMAP_SOURCE_TILE_DIR:-$ROOT_DIR/data/scratch/minimap/source_tiles}"
mkdir -p "$SITE_MAP_ASSET_DIR/ui"
mkdir -p "$CDN_MAP_ASSET_DIR"
mkdir -p "$CDN_IMAGE_ASSET_DIR"
mkdir -p "$CDN_FIELD_ASSET_DIR"
mkdir -p "$CDN_WAYPOINT_ASSET_DIR"
mkdir -p "$CDN_HOTSPOT_ASSET_DIR"

cp -f map/fishystuff_ui_bevy/assets/ui/fishystuff.css "$SITE_MAP_ASSET_DIR/ui/fishystuff.css"
rm -f \
  "$SITE_MAP_ASSET_DIR/fishystuff_ui_bevy.js" \
  "$SITE_MAP_ASSET_DIR/fishystuff_ui_bevy_bg.wasm"

WASM_BINDGEN_TMP_DIR="$(mktemp -d)"
HOTSPOT_SOURCE_LOOT_JSON=""
cleanup_temp_files() {
  rm -rf "$WASM_BINDGEN_TMP_DIR"
  if [ -n "$HOTSPOT_SOURCE_LOOT_JSON" ]; then
    rm -f "$HOTSPOT_SOURCE_LOOT_JSON"
  fi
}
trap cleanup_temp_files EXIT

wasm-bindgen --target web --no-typescript --out-dir "$WASM_BINDGEN_TMP_DIR" "$WASM_INPUT"

WASM_BUNDLE_INPUT="$WASM_BINDGEN_TMP_DIR/fishystuff_ui_bevy_bg.wasm"
JS_BUNDLE_INPUT="$WASM_BINDGEN_TMP_DIR/fishystuff_ui_bevy.js"

patch_wasm_bindgen_mouse_offsets() {
  local js_path="$1"

  # winit's web event path reads MouseEvent.offsetX/offsetY through generated
  # wasm-bindgen imports. In Chromium those accessors can force layout when
  # unrelated DOM has pending style work, so compute canvas-relative pointer
  # coordinates from clientX/clientY and a cached target rect instead.
  if ! grep -q 'function __wbg_get_imports() {' "$js_path"; then
    echo "wasm-bindgen JS changed: missing __wbg_get_imports hook point" >&2
    exit 1
  fi
  if ! grep -q 'const ret = arg0\.offsetX;' "$js_path" || ! grep -q 'const ret = arg0\.offsetY;' "$js_path"; then
    echo "wasm-bindgen JS changed: missing mouse offset imports" >&2
    exit 1
  fi

  local pointer_helper
  pointer_helper="$(cat <<'JS'
let __fishymapPointerRectCache = new WeakMap();
let __fishymapPointerRectListenersAttached = false;

function __fishymapInvalidatePointerRectCache() {
    __fishymapPointerRectCache = new WeakMap();
}

function __fishymapEnsurePointerRectListeners() {
    if (__fishymapPointerRectListenersAttached || typeof globalThis.addEventListener !== "function") {
        return;
    }
    __fishymapPointerRectListenersAttached = true;
    globalThis.addEventListener("resize", __fishymapInvalidatePointerRectCache, { passive: true });
    globalThis.addEventListener("scroll", __fishymapInvalidatePointerRectCache, { passive: true, capture: true });
}

function __fishymapCachedPointerTargetRect(event) {
    __fishymapEnsurePointerRectListeners();
    const target = event?.target ?? event?.currentTarget;
    if (!target || typeof target.getBoundingClientRect !== "function") {
        return null;
    }
    const cached = __fishymapPointerRectCache.get(target);
    if (cached) {
        return cached;
    }
    const rect = target.getBoundingClientRect();
    const next = { left: rect.left, top: rect.top };
    __fishymapPointerRectCache.set(target, next);
    return next;
}

function __fishymapPointerOffset(event, axis) {
    const rect = __fishymapCachedPointerTargetRect(event);
    const client = axis === "x" ? Number(event?.clientX) : Number(event?.clientY);
    if (rect && Number.isFinite(client)) {
        return client - (axis === "x" ? rect.left : rect.top);
    }
    return 0;
}

function __wbg_get_imports() {
JS
)"
  FISHYMAP_POINTER_HELPER="$pointer_helper" perl -0pi -e 's/function __wbg_get_imports\(\) \{/$ENV{FISHYMAP_POINTER_HELPER}/s' "$js_path"
  perl -0pi -e 's/const ret = arg0\.offsetX;\n\s*return ret;/return __fishymapPointerOffset(arg0, "x");/g' "$js_path"
  perl -0pi -e 's/const ret = arg0\.offsetY;\n\s*return ret;/return __fishymapPointerOffset(arg0, "y");/g' "$js_path"
  if grep -q 'const ret = arg0\.offsetX;' "$js_path" || grep -q 'const ret = arg0\.offsetY;' "$js_path"; then
    echo "wasm-bindgen JS changed: failed to patch mouse offset imports" >&2
    exit 1
  fi
}

patch_wasm_bindgen_mouse_offsets "$JS_BUNDLE_INPUT"

WASM_BUNDLE_HASH="$(sha256sum "$WASM_BUNDLE_INPUT" | cut -c1-16)"
WASM_BUNDLE_FILE="fishystuff_ui_bevy_bg.${WASM_BUNDLE_HASH}.wasm"
WASM_BUNDLE_PATH="$CDN_MAP_ASSET_DIR/$WASM_BUNDLE_FILE"

rm -f \
  "$CDN_MAP_ASSET_DIR/fishystuff_ui_bevy.js" \
  "$CDN_MAP_ASSET_DIR/fishystuff_ui_bevy_bg.wasm" \
  "$CDN_MAP_ASSET_DIR/runtime-manifest.json" \
  "$CDN_MAP_ASSET_DIR/$MAP_RUNTIME_MANIFEST_FILE"

cp -f "$WASM_BUNDLE_INPUT" "$WASM_BUNDLE_PATH"

sed \
  "s/fishystuff_ui_bevy_bg\\.wasm/${WASM_BUNDLE_FILE}/g" \
  "$JS_BUNDLE_INPUT" > "$WASM_BINDGEN_TMP_DIR/fishystuff_ui_bevy.patched.js"

JS_BUNDLE_PATCHED_INPUT="$WASM_BINDGEN_TMP_DIR/fishystuff_ui_bevy.patched.js"
JS_BUNDLE_HASH="$(sha256sum "$JS_BUNDLE_PATCHED_INPUT" | cut -c1-16)"
JS_BUNDLE_FILE="fishystuff_ui_bevy.${JS_BUNDLE_HASH}.js"
JS_BUNDLE_PATH="$CDN_MAP_ASSET_DIR/$JS_BUNDLE_FILE"

cp -f "$JS_BUNDLE_PATCHED_INPUT" "$JS_BUNDLE_PATH"

manifest_payload="$(cat <<EOF
{
  "generated_at_utc": "$(date -u +%Y-%m-%dT%H:%M:%SZ)",
  "module": "${JS_BUNDLE_FILE}",
  "wasm": "${WASM_BUNDLE_FILE}"
}
EOF
)"

printf '%s\n' "$manifest_payload" > "$CDN_MAP_ASSET_DIR/runtime-manifest.json"
printf '%s\n' "$manifest_payload" > "$CDN_MAP_ASSET_DIR/$MAP_RUNTIME_MANIFEST_FILE"

find "$CDN_MAP_ASSET_DIR" -maxdepth 1 -type f \
  \( -name 'fishystuff_ui_bevy.*.js' -o -name 'fishystuff_ui_bevy_bg.*.wasm' -o -name 'runtime-manifest.*.json' \) \
  ! -name "$JS_BUNDLE_FILE" \
  ! -name "$WASM_BUNDLE_FILE" \
  ! -name "$MAP_RUNTIME_MANIFEST_FILE" \
  -delete

first_existing_path() {
  local candidate
  for candidate in "$@"; do
    if [ -e "$candidate" ]; then
      printf '%s\n' "$candidate"
      return 0
    fi
  done
  printf '%s\n' "$1"
}

resolve_existing_path() {
  local candidate
  for candidate in "$@"; do
    if [ -e "$candidate" ]; then
      printf '%s\n' "$candidate"
      return 0
    fi
  done
  return 1
}

read_json_u32_field() {
  local json_path="$1"
  local field_name="$2"
  python3 - "$json_path" "$field_name" <<'PY'
import json
import sys
from pathlib import Path

json_path = Path(sys.argv[1])
field_name = sys.argv[2]
with json_path.open("r", encoding="utf-8") as handle:
    payload = json.load(handle)
value = payload.get(field_name)
if isinstance(value, int):
    print(value)
PY
}

prune_legacy_map_assets() {
  rm -rf \
    "$CDN_ROOT/images/exact_lookup" \
    "$CDN_ROOT/images/tiles/minimap" \
    "$CDN_ROOT/images/tiles/region_groups" \
    "$CDN_ROOT/images/tiles/zone_mask_visual" \
    "$CDN_ROOT/images/zones_mask_v1.png" \
    "$CDN_ROOT/region_groups"
}

prune_legacy_map_assets

prepare_terrain_source_tiles() {
  : "${TERRAIN_SOURCE_IMAGE:?set TERRAIN_SOURCE_IMAGE to the canonical terrain source image path}"
  : "${TERRAIN_SOURCE_TILE_DIR:=/tmp/fishystuff-terrain-whole_fullres-tiles}"
  : "${TERRAIN_SOURCE_TILE_SIZE:=512}"
  : "${TERRAIN_SOURCE_EXPECT_WIDTH:=32000}"
  : "${TERRAIN_SOURCE_EXPECT_HEIGHT:=27904}"

  if [ "${REBUILD_TERRAIN_SOURCE_TILES:-0}" = "1" ] || [ ! -f "$TERRAIN_SOURCE_TILE_DIR/0_0.png" ]; then
    rm -rf "$TERRAIN_SOURCE_TILE_DIR"
    mkdir -p "$TERRAIN_SOURCE_TILE_DIR"
    read -r SOURCE_WIDTH SOURCE_HEIGHT < <(magick identify -format "%w %h" "$TERRAIN_SOURCE_IMAGE")
    if [ "$SOURCE_WIDTH" != "$TERRAIN_SOURCE_EXPECT_WIDTH" ] || [ "$SOURCE_HEIGHT" != "$TERRAIN_SOURCE_EXPECT_HEIGHT" ]; then
      echo "unexpected terrain source dimensions: ${SOURCE_WIDTH}x${SOURCE_HEIGHT} (expected ${TERRAIN_SOURCE_EXPECT_WIDTH}x${TERRAIN_SOURCE_EXPECT_HEIGHT})" >&2
      exit 1
    fi
    magick "$TERRAIN_SOURCE_IMAGE" \
      -crop "${TERRAIN_SOURCE_TILE_SIZE}x${TERRAIN_SOURCE_TILE_SIZE}" \
      +repage \
      +adjoin \
      "$TERRAIN_SOURCE_TILE_DIR/tile_%05d.png"
    tiles_x=$(( (SOURCE_WIDTH + TERRAIN_SOURCE_TILE_SIZE - 1) / TERRAIN_SOURCE_TILE_SIZE ))
    for tile_path in "$TERRAIN_SOURCE_TILE_DIR"/tile_*.png; do
      tile_name="$(basename "$tile_path")"
      tile_index="${tile_name#tile_}"
      tile_index="${tile_index%.png}"
      tile_index=$((10#$tile_index))
      tile_x=$(( tile_index % tiles_x ))
      tile_y=$(( tile_index / tiles_x ))
      mv "$tile_path" "$TERRAIN_SOURCE_TILE_DIR/${tile_x}_${tile_y}.png"
    done
  fi
}

ensure_terrain_height_tiles() {
  : "${TERRAIN_HEIGHT_TILE_OUT_DIR:=$CDN_IMAGE_ASSET_DIR/terrain_height/v1}"

  prepare_terrain_source_tiles
  rm -rf "$TERRAIN_HEIGHT_TILE_OUT_DIR"
  mkdir -p "$TERRAIN_HEIGHT_TILE_OUT_DIR"
  cp -f "$TERRAIN_SOURCE_TILE_DIR"/*.png "$TERRAIN_HEIGHT_TILE_OUT_DIR"/
}

: "${TERRAIN_HEIGHT_TILE_OUT_DIR:=$CDN_IMAGE_ASSET_DIR/terrain_height/v1}"
terrain_source_image="${TERRAIN_SOURCE_IMAGE:-}"
if [ -z "$terrain_source_image" ]; then
  terrain_source_image="$(resolve_existing_path \
    data/terrain/Karpfen/terraintiles/whole_fullres.png \
    zonegen/data/Karpfen/terraintiles/whole_fullres.png || true)"
fi
if [ "${REBUILD_TERRAIN_HEIGHT_TILES:-0}" = "1" ] || [ ! -f "$TERRAIN_HEIGHT_TILE_OUT_DIR/0_0.png" ]; then
  if [ -n "$terrain_source_image" ] && [ -f "$terrain_source_image" ]; then
    TERRAIN_SOURCE_IMAGE="$terrain_source_image"
    ensure_terrain_height_tiles
  else
    echo "warning: terrain source image not found; skipping terrain_height/v1 build" >&2
  fi
fi

if [ "${REBUILD_TERRAIN_PYRAMID:-0}" = "1" ]; then
  : "${TERRAIN_PYRAMID_SOURCE_ROOT:=$(resolve_existing_path \
    data/terrain/Karpfen/terraintiles/7 \
    zonegen/data/Karpfen/terraintiles/7)}"
  : "${TERRAIN_PYRAMID_OUT_DIR:=$CDN_IMAGE_ASSET_DIR/terrain/v1}"
  rm -rf "$TERRAIN_PYRAMID_OUT_DIR"
  cargo run --manifest-path "$ROOT_DIR/Cargo.toml" --release -p fishystuff_tilegen --bin terrain_pyramid -- build-terrain-pyramid \
    --source-root "$TERRAIN_PYRAMID_SOURCE_ROOT" \
    --out-dir "$TERRAIN_PYRAMID_OUT_DIR" \
    --revision v1 \
    --root-url /images/terrain/v1 \
    --chunk-path "levels/{level}/{x}_{y}.thc" \
    --map-width 11560 \
    --map-height 10540 \
    --chunk-map-px 256 \
    --grid-size 65 \
    --max-level 7 \
    --bbox-y-min=-9500 \
    --bbox-y-max=24000
fi
if [ "${REBUILD_TERRAIN_DRAPE_MINIMAP:-0}" = "1" ]; then
  : "${TERRAIN_DRAPE_SOURCE_IMAGE:?set TERRAIN_DRAPE_SOURCE_IMAGE to the canonical minimap source image path}"
  : "${TERRAIN_PYRAMID_OUT_DIR:=$CDN_IMAGE_ASSET_DIR/terrain/v1}"
  : "${TERRAIN_DRAPE_OUT_DIR:=$CDN_IMAGE_ASSET_DIR/terrain_drape/minimap/v1}"
  rm -rf "$TERRAIN_DRAPE_OUT_DIR"
  cargo run --manifest-path "$ROOT_DIR/Cargo.toml" --release -p fishystuff_tilegen --bin terrain_pyramid -- build-terrain-drape-pyramid \
    --terrain-manifest "$TERRAIN_PYRAMID_OUT_DIR/manifest.json" \
    --source-image "$TERRAIN_DRAPE_SOURCE_IMAGE" \
    --out-dir "$TERRAIN_DRAPE_OUT_DIR" \
    --layer minimap \
    --revision v1 \
    --root-url /images/terrain_drape/minimap/v1 \
    --chunk-path "levels/{level}/{x}_{y}.png" \
    --texture-px 256 \
    --kind raster-visual
fi

MINIMAP_DISPLAY_TILE_PX=512
MINIMAP_DISPLAY_MAX_LEVEL=2
minimap_source_archive="${MINIMAP_SOURCE_ARCHIVE:-$ROOT_DIR/data/scratch/paz}"
minimap_display_source_dir="$MINIMAP_SOURCE_TILE_DIR"
minimap_display_root="$CDN_IMAGE_ASSET_DIR/tiles/minimap_visual/v1"
minimap_display_manifest="$minimap_display_root/tileset.json"
minimap_display_manifest_tile_px=""
minimap_display_manifest_max_level=""
if [ -e "$minimap_source_archive" ]; then
  minimap_build_args=(
    "$ROOT_DIR/tools/scripts/build_minimap_tiles_from_source.mjs"
    --source-archive "$minimap_source_archive"
    --raw-output-dir "$minimap_display_source_dir"
    --visual-output-dir "$minimap_display_root"
  )
  if [ "${REBUILD_MINIMAP_DISPLAY_TILES:-0}" = "1" ]; then
    minimap_build_args+=(--force-visual)
  fi
  node "${minimap_build_args[@]}"
elif [ -f "$minimap_display_manifest" ]; then
  minimap_display_manifest_tile_px="$(
    read_json_u32_field "$minimap_display_manifest" "tile_size_px" || true
  )"
  minimap_display_manifest_max_level="$(
    python3 - "$minimap_display_manifest" <<'PY'
import json, sys
path = sys.argv[1]
with open(path, 'r', encoding='utf-8') as fh:
    data = json.load(fh)
levels = data.get("levels") or []
if not levels:
    print("")
else:
    print(max(int(level.get("z", 0)) for level in levels))
PY
  )"
fi
if [ ! -e "$minimap_source_archive" ] && [ -d "$minimap_display_source_dir" ] && {
  [ "${REBUILD_MINIMAP_DISPLAY_TILES:-0}" = "1" ] ||
  [ ! -f "$minimap_display_manifest" ] ||
  [ "$minimap_display_manifest_tile_px" != "$MINIMAP_DISPLAY_TILE_PX" ] ||
  [ "$minimap_display_manifest_max_level" != "$MINIMAP_DISPLAY_MAX_LEVEL" ] ||
  find "$minimap_display_source_dir" -maxdepth 1 -name 'rader_*.png' -newer "$minimap_display_manifest" -print -quit | grep -q .
}; then
  find "$(dirname "$minimap_display_root")" -maxdepth 1 -type d \
    -name "$(basename "$minimap_display_root").tmp.*" \
    -exec rm -rf {} +
  minimap_display_tmp_root="${minimap_display_root}.tmp.$$"
  rm -rf "$minimap_display_tmp_root"
  cargo run --manifest-path "$ROOT_DIR/Cargo.toml" --release -p fishystuff_tilegen --bin minimap_display_tiles -- \
    --input-dir "$minimap_display_source_dir" \
    --out-dir "$minimap_display_tmp_root" \
    --tile-px "$MINIMAP_DISPLAY_TILE_PX" \
    --max-level "$MINIMAP_DISPLAY_MAX_LEVEL" \
    --root-url /images/tiles/minimap_visual/v1
  rm -rf "$minimap_display_root"
  mv "$minimap_display_tmp_root" "$minimap_display_root"
elif [ ! -e "$minimap_source_archive" ] && [ ! -f "$minimap_display_manifest" ]; then
  echo "warning: minimap source archive not found; skipping minimap_visual/v1 build" >&2
fi

zone_mask_source_image="${ZONE_MASK_SOURCE_IMAGE:-$(first_existing_path \
  data/imagery/zones_mask_2025_12.png)}"
zone_lookup_source_image="${ZONE_LOOKUP_SOURCE_IMAGE:-$zone_mask_source_image}"
zone_lookup_output="$CDN_FIELD_ASSET_DIR/zone_mask.v1.bin"
if [ -f "$zone_lookup_source_image" ] && { [ "${REBUILD_ZONE_LOOKUP:-0}" = "1" ] || [ ! -f "$zone_lookup_output" ] || [ "$zone_lookup_source_image" -nt "$zone_lookup_output" ]; }; then
  mkdir -p "$CDN_FIELD_ASSET_DIR"
  cargo run --manifest-path "$ROOT_DIR/Cargo.toml" -p fishystuff_tilegen --bin zone_lookup -- \
    --input "$zone_lookup_source_image" \
    --output "$zone_lookup_output"
fi

zone_mask_field_metadata_output="$CDN_FIELD_ASSET_DIR/zone_mask.v1.meta.json"
if [ -f "$zone_lookup_output" ] && {
  [ "${REBUILD_ZONE_MASK_FIELD_METADATA:-0}" = "1" ] ||
  [ ! -f "$zone_mask_field_metadata_output" ] ||
  [ "$zone_lookup_output" -nt "$zone_mask_field_metadata_output" ];
}; then
  cargo run --manifest-path "$ROOT_DIR/Cargo.toml" -p fishystuff_ingest -- \
    build-zone-mask-field-metadata \
    --field "$zone_lookup_output" \
    --out "$zone_mask_field_metadata_output"
fi

regions_field_input="${REGIONS_FIELD_INPUT:-$(first_existing_path \
  data/scratch/ui_texture/minimap/area/regionmap_new.bmp.rid)}"
regioninfo_bss_input="${REGIONINFO_BSS_INPUT:-$(first_existing_path \
  data/scratch/gamecommondata/binary/regioninfo.bss)}"
regiongroupinfo_bss_input="${REGIONGROUPINFO_BSS_INPUT:-$(first_existing_path \
  data/scratch/gamecommondata/binary/regiongroupinfo.bss)}"
region_loc_input="${REGION_LOC_INPUT:-$(first_existing_path \
  data/data/languagedata_en.loc)}"
character_function_xlsx_input="${CHARACTER_FUNCTION_XLSX_INPUT:-$(first_existing_path \
  data/data/excel/CharacterFunction_Table.xlsx)}"
character_table_xlsx_input="${CHARACTER_TABLE_XLSX_INPUT:-$(first_existing_path \
  data/data/excel/Character_Table.xlsx)}"
selling_to_npc_xlsx_input="${SELLING_TO_NPC_XLSX_INPUT:-$(first_existing_path \
  data/data/excel/SellingToNpc_Table.xlsx)}"
float_fishing_point_xlsx_input="${FLOAT_FISHING_POINT_XLSX_INPUT:-$(first_existing_path \
  data/data/excel/FloatFishingPoint_Table.xlsx)}"
float_fishing_xlsx_input="${FLOAT_FISHING_XLSX_INPUT:-$(first_existing_path \
  data/data/excel/FloatFishing_Table.xlsx)}"
bdolytics_hotspots_json_input="${BDOLYTICS_HOTSPOTS_JSON_INPUT:-$(first_existing_path \
  data/data/hotspots_bdolytics.json)}"
regionclientdata_input="${REGIONCLIENTDATA_INPUT:-$(first_existing_path \
  data/scratch/gamecommondata/regionclientdata_en_.xml)}"
waypoint_xml_primary="${WAYPOINT_XML_PRIMARY:-$(first_existing_path \
  data/scratch/gamecommondata/waypoint/mapdata_realexplore.xml)}"
waypoint_xml_secondary="${WAYPOINT_XML_SECONDARY:-$(first_existing_path \
  data/scratch/gamecommondata/waypoint/mapdata_realexplore2.xml)}"
regions_field_output="$CDN_FIELD_ASSET_DIR/regions.v1.bin"
region_groups_field_output="$CDN_FIELD_ASSET_DIR/region_groups.v1.bin"
regions_field_metadata_output="$CDN_FIELD_ASSET_DIR/regions.v1.meta.json"
region_groups_field_metadata_output="$CDN_FIELD_ASSET_DIR/region_groups.v1.meta.json"
region_nodes_output="$CDN_WAYPOINT_ASSET_DIR/region_nodes.v1.geojson"
hotspots_output="$CDN_HOTSPOT_ASSET_DIR/hotspots.v1.json"
waypoint_xml_args=()
if [ -f "$waypoint_xml_primary" ]; then
  waypoint_xml_args+=(--waypoint-xml "$waypoint_xml_primary")
fi
if [ -f "$waypoint_xml_secondary" ]; then
  waypoint_xml_args+=(--waypoint-xml "$waypoint_xml_secondary")
fi
build_hotspot_source_loot_json() {
  local out_path="$1"
  dolt sql -r json -q "
    SELECT *
    FROM (
    SELECT
      CAST(m.item_main_group_key AS SIGNED) AS item_main_group_key,
      CAST(m.option_idx AS SIGNED) AS option_idx,
      CAST(m.select_rate AS SIGNED) AS option_select_rate,
      NULLIF(TRIM(m.condition_raw), '') AS condition_raw,
      CAST(m.item_sub_group_key AS SIGNED) AS item_sub_group_key,
      CAST(v.item_key AS SIGNED) AS item_key,
      CAST(v.select_rate AS SIGNED) AS item_select_rate,
      NULLIF(TRIM(item_name.\`text\`), '') AS item_name,
      NULLIF(TRIM(it.IconImageFile), '') AS icon_image,
      it.GradeType AS grade_type,
      CASE WHEN ft.item_key IS NULL THEN 0 ELSE 1 END AS is_fish,
      NULL AS item_source_tooltip
    FROM item_main_group_options m
    LEFT JOIN item_sub_group_item_variants v
      ON v.item_sub_group_key = m.item_sub_group_key
    LEFT JOIN item_table it
      ON CAST(it.\`Index\` AS SIGNED) = CAST(v.item_key AS SIGNED)
    LEFT JOIN fish_table ft
      ON CAST(ft.item_key AS SIGNED) = CAST(v.item_key AS SIGNED)
    LEFT JOIN languagedata item_name
      ON item_name.lang = 'en'
     AND item_name.id = CAST(v.item_key AS SIGNED)
     AND item_name.format = 'A'
     AND item_name.category = ''
    UNION ALL
    SELECT
      CAST(m.item_main_group_key AS SIGNED) AS item_main_group_key,
      CAST(m.option_idx AS SIGNED) AS option_idx,
      CAST(m.select_rate AS SIGNED) AS option_select_rate,
      NULLIF(TRIM(m.condition_raw), '') AS condition_raw,
      CAST(m.item_sub_group_key AS SIGNED) AS item_sub_group_key,
      CAST(it.\`Index\` AS SIGNED) AS item_key,
      CAST(500000 AS SIGNED) AS item_select_rate,
      NULLIF(TRIM(item_name.\`text\`), '') AS item_name,
      NULLIF(TRIM(it.IconImageFile), '') AS icon_image,
      it.GradeType AS grade_type,
      CASE WHEN ft.item_key IS NULL THEN 0 ELSE 1 END AS is_fish,
      'Contents group 689 open branch: item_main_group_options subgroup 11185 plus item_table item 800108' AS item_source_tooltip
    FROM item_main_group_options m
    JOIN item_table it
      ON CAST(it.\`Index\` AS SIGNED) = 800108
    LEFT JOIN fish_table ft
      ON CAST(ft.item_key AS SIGNED) = CAST(it.\`Index\` AS SIGNED)
    LEFT JOIN languagedata item_name
      ON item_name.lang = 'en'
     AND item_name.id = CAST(it.\`Index\` AS SIGNED)
     AND item_name.format = 'A'
     AND item_name.category = ''
    WHERE CAST(m.item_sub_group_key AS SIGNED) = 11185
      AND NULLIF(TRIM(m.condition_raw), '') = 'isContentsGroupOpen(0,689);'
    UNION ALL
    SELECT
      CAST(m.item_main_group_key AS SIGNED) AS item_main_group_key,
      CAST(m.option_idx AS SIGNED) AS option_idx,
      CAST(m.select_rate AS SIGNED) AS option_select_rate,
      NULLIF(TRIM(m.condition_raw), '') AS condition_raw,
      CAST(m.item_sub_group_key AS SIGNED) AS item_sub_group_key,
      CAST(fallback.item_key AS SIGNED) AS item_key,
      CAST(500000 AS SIGNED) AS item_select_rate,
      NULLIF(TRIM(item_name.\`text\`), '') AS item_name,
      NULLIF(TRIM(it.IconImageFile), '') AS icon_image,
      it.GradeType AS grade_type,
      CASE WHEN ft.item_key IS NULL THEN 0 ELSE 1 END AS is_fish,
      'Contents group 689 open branch: fallback fish from sibling closed subgroup for item_main_group_options subgroup 11185' AS item_source_tooltip
    FROM item_main_group_options m
    JOIN item_main_group_options sibling
      ON sibling.item_main_group_key = m.item_main_group_key
     AND NULLIF(TRIM(sibling.condition_raw), '') = CONCAT('!', NULLIF(TRIM(m.condition_raw), ''))
    JOIN (
      SELECT DISTINCT item_sub_group_key, item_key
      FROM item_sub_group_item_variants
    ) fallback
      ON fallback.item_sub_group_key = sibling.item_sub_group_key
    LEFT JOIN item_table it
      ON CAST(it.\`Index\` AS SIGNED) = CAST(fallback.item_key AS SIGNED)
    LEFT JOIN fish_table ft
      ON CAST(ft.item_key AS SIGNED) = CAST(fallback.item_key AS SIGNED)
    LEFT JOIN languagedata item_name
      ON item_name.lang = 'en'
     AND item_name.id = CAST(fallback.item_key AS SIGNED)
     AND item_name.format = 'A'
     AND item_name.category = ''
    WHERE CAST(m.item_sub_group_key AS SIGNED) = 11185
      AND NULLIF(TRIM(m.condition_raw), '') = 'isContentsGroupOpen(0,689);'
    ) hotspot_loot_rows
    ORDER BY
      item_main_group_key,
      option_idx,
      item_sub_group_key,
      item_key
  " > "$out_path"
}

if [ -f "$regions_field_input" ] && { [ "${REBUILD_REGIONS_FIELD:-0}" = "1" ] || [ ! -f "$regions_field_output" ] || [ "$regions_field_input" -nt "$regions_field_output" ]; }; then
  cargo run --manifest-path "$ROOT_DIR/Cargo.toml" -p pazifista --bin pazifista -- \
    pabr export-regions-field \
    "$regions_field_input" \
    --output "$regions_field_output"
fi

if [ -f "$regions_field_input" ] && [ -f "$regioninfo_bss_input" ] && {
  [ "${REBUILD_REGION_GROUPS_FIELD:-0}" = "1" ] ||
  [ ! -f "$region_groups_field_output" ] ||
  [ "$regions_field_input" -nt "$region_groups_field_output" ] ||
  [ "$regioninfo_bss_input" -nt "$region_groups_field_output" ];
}; then
  cargo run --manifest-path "$ROOT_DIR/Cargo.toml" -p pazifista --bin pazifista -- \
    pabr export-region-groups-field \
    "$regions_field_input" \
    --regioninfo-bss "$regioninfo_bss_input" \
    --output "$region_groups_field_output"
fi

if [ -f "$regions_field_output" ] && [ -f "$regioninfo_bss_input" ] && [ -f "$regiongroupinfo_bss_input" ] && [ -f "$region_loc_input" ] && [ "${#waypoint_xml_args[@]}" -gt 0 ] && {
  [ "${REBUILD_REGIONS_FIELD_METADATA:-0}" = "1" ] ||
  [ ! -f "$regions_field_metadata_output" ] ||
  [ "$regions_field_output" -nt "$regions_field_metadata_output" ] ||
  [ "$regioninfo_bss_input" -nt "$regions_field_metadata_output" ] ||
  [ "$regiongroupinfo_bss_input" -nt "$regions_field_metadata_output" ] ||
  [ "$region_loc_input" -nt "$regions_field_metadata_output" ];
}; then
  cargo run --manifest-path "$ROOT_DIR/Cargo.toml" -p fishystuff_ingest -- \
    build-regions-field-metadata \
    --field "$regions_field_output" \
    --regioninfo-bss "$regioninfo_bss_input" \
    --regiongroupinfo-bss "$regiongroupinfo_bss_input" \
    --loc "$region_loc_input" \
    "${waypoint_xml_args[@]}" \
    --out "$regions_field_metadata_output"
fi

if [ -f "$region_groups_field_output" ] && [ -f "$regioninfo_bss_input" ] && [ -f "$regiongroupinfo_bss_input" ] && [ -f "$region_loc_input" ] && [ "${#waypoint_xml_args[@]}" -gt 0 ] && {
  [ "${REBUILD_REGION_GROUPS_FIELD_METADATA:-0}" = "1" ] ||
  [ ! -f "$region_groups_field_metadata_output" ] ||
  [ "$region_groups_field_output" -nt "$region_groups_field_metadata_output" ] ||
  [ "$regions_field_output" -nt "$region_groups_field_metadata_output" ] ||
  [ "$regioninfo_bss_input" -nt "$region_groups_field_metadata_output" ] ||
  [ "$regiongroupinfo_bss_input" -nt "$region_groups_field_metadata_output" ] ||
  [ "$region_loc_input" -nt "$region_groups_field_metadata_output" ];
}; then
  cargo run --manifest-path "$ROOT_DIR/Cargo.toml" -p fishystuff_ingest -- \
    build-region-groups-field-metadata \
    --field "$region_groups_field_output" \
    --regions-field "$regions_field_output" \
    --regioninfo-bss "$regioninfo_bss_input" \
    --regiongroupinfo-bss "$regiongroupinfo_bss_input" \
    --loc "$region_loc_input" \
    "${waypoint_xml_args[@]}" \
    --out "$region_groups_field_metadata_output"
fi

if [ -f "$regioninfo_bss_input" ] && [ -f "$regiongroupinfo_bss_input" ] && [ -f "$region_loc_input" ] && [ "${#waypoint_xml_args[@]}" -gt 0 ] && {
  [ "${REBUILD_REGION_NODE_WAYPOINTS:-0}" = "1" ] ||
  [ ! -f "$region_nodes_output" ] ||
  [ "$regioninfo_bss_input" -nt "$region_nodes_output" ] ||
  [ "$regiongroupinfo_bss_input" -nt "$region_nodes_output" ] ||
  [ "$region_loc_input" -nt "$region_nodes_output" ];
}; then
  cargo run --manifest-path "$ROOT_DIR/Cargo.toml" -p fishystuff_ingest -- \
    build-region-nodes-geojson \
    --regioninfo-bss "$regioninfo_bss_input" \
    --regiongroupinfo-bss "$regiongroupinfo_bss_input" \
    --loc "$region_loc_input" \
    "${waypoint_xml_args[@]}" \
    --out "$region_nodes_output"
fi

if [ -f "$float_fishing_point_xlsx_input" ] && [ -f "$float_fishing_xlsx_input" ]; then
  HOTSPOT_SOURCE_LOOT_JSON="$(mktemp)"
  build_hotspot_source_loot_json "$HOTSPOT_SOURCE_LOOT_JSON"
  hotspot_metadata_args=()
  if [ -f "$bdolytics_hotspots_json_input" ]; then
    hotspot_metadata_args+=(--bdolytics-hotspots-json "$bdolytics_hotspots_json_input")
  fi
  cargo run --manifest-path "$ROOT_DIR/Cargo.toml" -p fishystuff_ingest -- \
    build-hotspots-asset \
    --float-fishing-point-xlsx "$float_fishing_point_xlsx_input" \
    --float-fishing-xlsx "$float_fishing_xlsx_input" \
    --source-loot-groups-json "$HOTSPOT_SOURCE_LOOT_JSON" \
    "${hotspot_metadata_args[@]}" \
    --out "$hotspots_output"
fi
