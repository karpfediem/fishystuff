#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT_DIR"

: "${CARGO_HOME:=/tmp/cargo}"
export CARGO_HOME
export RUSTFLAGS='--cfg getrandom_backend="wasm_js"'

PROFILE="${FISHYSTUFF_WASM_PROFILE:-release}"
if [ "$PROFILE" = "release" ]; then
  cargo build --manifest-path "$ROOT_DIR/Cargo.toml" -p fishystuff_ui_bevy --target wasm32-unknown-unknown --release
  WASM_INPUT="target/wasm32-unknown-unknown/release/fishystuff_ui_bevy.wasm"
else
  cargo build --manifest-path "$ROOT_DIR/Cargo.toml" -p fishystuff_ui_bevy --target wasm32-unknown-unknown
  WASM_INPUT="target/wasm32-unknown-unknown/debug/fishystuff_ui_bevy.wasm"
fi

SITE_MAP_ASSET_DIR="$ROOT_DIR/site/assets/map"
SITE_IMAGE_ASSET_DIR="$ROOT_DIR/site/assets/images"
mkdir -p "$SITE_MAP_ASSET_DIR/ui"
mkdir -p "$SITE_IMAGE_ASSET_DIR"

wasm-bindgen --target web --no-typescript --out-dir "$SITE_MAP_ASSET_DIR" "$WASM_INPUT"
cp -f map/fishystuff_ui_bevy/assets/ui/fishystuff.css "$SITE_MAP_ASSET_DIR/ui/fishystuff.css"

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

prepare_terrain_source_tiles() {
  : "${TERRAIN_SOURCE_IMAGE:=$(first_existing_path \
    data/terrain/Karpfen/terraintiles/whole_fullres.png \
    zonegen/data/Karpfen/terraintiles/whole_fullres.png)}"
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
  : "${TERRAIN_HEIGHT_TILE_OUT_DIR:=site/assets/images/terrain_height/v1}"

  prepare_terrain_source_tiles
  rm -rf "$TERRAIN_HEIGHT_TILE_OUT_DIR"
  mkdir -p "$TERRAIN_HEIGHT_TILE_OUT_DIR"
  cp -f "$TERRAIN_SOURCE_TILE_DIR"/*.png "$TERRAIN_HEIGHT_TILE_OUT_DIR"/
}

: "${TERRAIN_HEIGHT_TILE_OUT_DIR:=site/assets/images/terrain_height/v1}"
if [ "${REBUILD_TERRAIN_HEIGHT_TILES:-0}" = "1" ] || [ ! -f "$TERRAIN_HEIGHT_TILE_OUT_DIR/0_0.png" ]; then
  ensure_terrain_height_tiles
fi

if [ "${REBUILD_TERRAIN_PYRAMID:-0}" = "1" ]; then
  : "${TERRAIN_PYRAMID_SOURCE_ROOT:=$(first_existing_path \
    data/terrain/Karpfen/terraintiles/7 \
    zonegen/data/Karpfen/terraintiles/7)}"
  : "${TERRAIN_PYRAMID_OUT_DIR:=site/assets/images/terrain/v1}"
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
  : "${TERRAIN_PYRAMID_OUT_DIR:=site/assets/images/terrain/v1}"
  : "${TERRAIN_DRAPE_OUT_DIR:=site/assets/images/terrain_drape/minimap/v1}"
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
if [ "${REBUILD_MINIMAP_PYRAMID:-0}" = "1" ]; then
  cargo run --manifest-path "$ROOT_DIR/Cargo.toml" -p fishystuff_tilegen --bin minimap_pyramid -- \
    --input-dir site/assets/images/tiles/minimap \
    --out-dir site/assets/images/tiles/minimap/v1 \
    --tile-px 128 \
    --max-level 8 \
    --root-url /images/tiles/minimap/v1 \
    --y-flip
fi
