#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
CDN_ROOT="${CDN_ROOT:-$ROOT_DIR/data/cdn/public}"
BROTLI_QUALITY="${FISHYSTUFF_CDN_PRECOMPRESS_BROTLI_QUALITY:-11}"
GZIP_LEVEL="${FISHYSTUFF_CDN_PRECOMPRESS_GZIP_LEVEL:-9}"

if [ "${FISHYSTUFF_CDN_PRECOMPRESS:-1}" = "0" ]; then
  exit 0
fi

if ! command -v brotli >/dev/null 2>&1; then
  echo "brotli is required to precompress CDN assets; enter the devenv shell or install brotli" >&2
  exit 1
fi

if ! command -v gzip >/dev/null 2>&1; then
  echo "gzip is required to precompress CDN assets; enter the devenv shell or install gzip" >&2
  exit 1
fi

is_precompressible_file() {
  local path="$1"
  case "$path" in
    *.br|*.gz)
      return 1
      ;;
    *.bin|*.css|*.geojson|*.js|*.json|*.map|*.mjs|*.svg|*.txt|*.wasm|*.xml|*.ziggy)
      return 0
      ;;
    *)
      return 1
      ;;
  esac
}

write_brotli_sidecar() {
  local source_path="$1"
  local sidecar_path="${source_path}.br"
  local tmp_path="${sidecar_path}.tmp.$$"

  if [ -f "$sidecar_path" ] && [ "$sidecar_path" -nt "$source_path" ]; then
    keep_sidecar_if_smaller "$source_path" "$sidecar_path"
    return 0
  fi

  brotli \
    --quality="$BROTLI_QUALITY" \
    --no-copy-stat \
    --force \
    --output="$tmp_path" \
    "$source_path"
  mv -f "$tmp_path" "$sidecar_path"
  keep_sidecar_if_smaller "$source_path" "$sidecar_path"
}

write_gzip_sidecar() {
  local source_path="$1"
  local sidecar_path="${source_path}.gz"
  local tmp_path="${sidecar_path}.tmp.$$"

  if [ -f "$sidecar_path" ] && [ "$sidecar_path" -nt "$source_path" ]; then
    keep_sidecar_if_smaller "$source_path" "$sidecar_path"
    return 0
  fi

  gzip \
    "-${GZIP_LEVEL}" \
    --no-name \
    --stdout \
    "$source_path" > "$tmp_path"
  mv -f "$tmp_path" "$sidecar_path"
  keep_sidecar_if_smaller "$source_path" "$sidecar_path"
}

file_size_bytes() {
  wc -c < "$1" | tr -d '[:space:]'
}

keep_sidecar_if_smaller() {
  local source_path="$1"
  local sidecar_path="$2"
  local source_size
  local sidecar_size

  source_size="$(file_size_bytes "$source_path")"
  sidecar_size="$(file_size_bytes "$sidecar_path")"
  if [ "$sidecar_size" -ge "$source_size" ]; then
    rm -f "$sidecar_path"
  fi
}

precompress_file() {
  local source_path="$1"

  if ! is_precompressible_file "$source_path"; then
    return 0
  fi

  write_brotli_sidecar "$source_path"
  write_gzip_sidecar "$source_path"
}

precompress_target() {
  local target="$1"

  if [ -f "$target" ]; then
    precompress_file "$target"
    return 0
  fi

  if [ -d "$target" ]; then
    while IFS= read -r -d "" file_path; do
      precompress_file "$file_path"
    done < <(find "$target" -type f -print0 | sort -z)
    return 0
  fi

  echo "precompression target does not exist: $target" >&2
  exit 1
}

if [ "$#" -eq 0 ]; then
  set -- \
    "$CDN_ROOT/map" \
    "$CDN_ROOT/fields" \
    "$CDN_ROOT/waypoints" \
    "$CDN_ROOT/hotspots" \
    "$CDN_ROOT/images/tiles" \
    "$CDN_ROOT/images/terrain" \
    "$CDN_ROOT/images/terrain_drape" \
    "$CDN_ROOT/images/terrain_height" \
    "$CDN_ROOT/images/terrain_fullres"
fi

for target in "$@"; do
  if [ -e "$target" ]; then
    precompress_target "$target"
  fi
done
