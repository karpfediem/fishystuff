#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
CDN_ROOT="${CDN_ROOT:-$ROOT_DIR/data/cdn/public}"
SITE_MAP_ASSETS_DIR="$ROOT_DIR/site/assets/map"

require_path() {
  local path="$1"
  if [ ! -e "$path" ]; then
    echo "required path missing: $path" >&2
    exit 1
  fi
}

require_path "$SITE_MAP_ASSETS_DIR/loader.js"
require_path "$SITE_MAP_ASSETS_DIR/map-host.js"
require_path "$SITE_MAP_ASSETS_DIR/fishystuff_ui_bevy.js"
require_path "$SITE_MAP_ASSETS_DIR/fishystuff_ui_bevy_bg.wasm"
require_path "$SITE_MAP_ASSETS_DIR/ui/fishystuff.css"
require_path "$CDN_ROOT/images"
require_path "$CDN_ROOT/region_groups"

mkdir -p "$CDN_ROOT/map/ui" "$CDN_ROOT/logs"

rsync -a \
  "$SITE_MAP_ASSETS_DIR/loader.js" \
  "$SITE_MAP_ASSETS_DIR/map-host.js" \
  "$SITE_MAP_ASSETS_DIR/fishystuff_ui_bevy.js" \
  "$SITE_MAP_ASSETS_DIR/fishystuff_ui_bevy_bg.wasm" \
  "$CDN_ROOT/map/"

rsync -a "$SITE_MAP_ASSETS_DIR/ui/fishystuff.css" "$CDN_ROOT/map/ui/"

cat > "$CDN_ROOT/.cdn-metadata.json" <<EOF
{
  "base_url": "https://cdn.fishystuff.fish",
  "generated_at_utc": "$(date -u +%Y-%m-%dT%H:%M:%SZ)",
  "source_roots": [
    "data/cdn/public/images",
    "data/cdn/public/region_groups",
    "site/assets/map"
  ],
  "paths": [
    "images/",
    "map/",
    "region_groups/"
  ]
}
EOF

echo "staged CDN payload in $CDN_ROOT"
