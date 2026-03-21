#!/usr/bin/env bash
set -euo pipefail

SITE_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
TMP_DIR="$(mktemp -d)"
trap 'rm -rf "$TMP_DIR"' EXIT

CHARSET_FILE="$TMP_DIR/site-text.txt"

cd "$SITE_DIR"

cat \
  tailwind.input.css \
  assets/css/style.css \
  assets/config.ziggy \
  $(find content i18n layouts -type f \( -name '*.smd' -o -name '*.ziggy' -o -name '*.shtml' -o -name '*.html' -o -name '*.css' -o -name '*.js' -o -name '*.mjs' \) | sort) \
  > "$CHARSET_FILE"

build_subset() {
  local source_file="$1"
  local subset_ttf="$2"
  local output_woff2="$3"

  pyftsubset "$source_file" \
    --text-file="$CHARSET_FILE" \
    --output-file="$subset_ttf" \
    --layout-features='*' \
    --glyph-names \
    --symbol-cmap \
    --legacy-cmap \
    --name-IDs='*' \
    --name-legacy \
    --name-languages='*'

  rm -f "${subset_ttf%.ttf}.woff2"
  woff2_compress "$subset_ttf" >/dev/null 2>&1
  mv -f "${subset_ttf%.ttf}.woff2" "$output_woff2"
}

build_subset \
  "assets/css/fonts/Comfortaa/Comfortaa-VariableFont_wght.ttf" \
  "$TMP_DIR/Comfortaa-VariableFont_wght.site.ttf" \
  "assets/css/fonts/Comfortaa/Comfortaa-VariableFont_wght.site.woff2"

build_subset \
  "assets/css/fonts/Itim/Itim-Regular.ttf" \
  "$TMP_DIR/Itim-Regular.site.ttf" \
  "assets/css/fonts/Itim/Itim-Regular.site.woff2"

build_subset \
  "assets/css/fonts/Pacifico/Pacifico-Regular.ttf" \
  "$TMP_DIR/Pacifico-Regular.site.ttf" \
  "assets/css/fonts/Pacifico/Pacifico-Regular.site.woff2"
