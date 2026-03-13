#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
CDN_ROOT="${CDN_ROOT:-$ROOT_DIR/data/cdn/public}"
SITE_ASSETS_DIR="$ROOT_DIR/site/assets"

require_path() {
  local path="$1"
  if [ ! -e "$path" ]; then
    echo "required path missing: $path" >&2
    exit 1
  fi
}

require_path "$SITE_ASSETS_DIR/map/loader.js"
require_path "$SITE_ASSETS_DIR/map/map-host.js"
require_path "$SITE_ASSETS_DIR/map/fishystuff_ui_bevy.js"
require_path "$SITE_ASSETS_DIR/map/fishystuff_ui_bevy_bg.wasm"
require_path "$SITE_ASSETS_DIR/map/ui/fishystuff.css"
require_path "$SITE_ASSETS_DIR/images"
require_path "$SITE_ASSETS_DIR/region_groups"

mkdir -p "$CDN_ROOT/map/ui" "$CDN_ROOT/logs"

rsync -a --delete "$SITE_ASSETS_DIR/images/" "$CDN_ROOT/images/"
rsync -a --delete "$SITE_ASSETS_DIR/region_groups/" "$CDN_ROOT/region_groups/"

rsync -a \
  "$SITE_ASSETS_DIR/map/loader.js" \
  "$SITE_ASSETS_DIR/map/map-host.js" \
  "$SITE_ASSETS_DIR/map/fishystuff_ui_bevy.js" \
  "$SITE_ASSETS_DIR/map/fishystuff_ui_bevy_bg.wasm" \
  "$CDN_ROOT/map/"

rsync -a "$SITE_ASSETS_DIR/map/ui/fishystuff.css" "$CDN_ROOT/map/ui/"

cat > "$CDN_ROOT/.cdn-metadata.json" <<EOF
{
  "base_url": "https://cdn.fishystuff.fish",
  "generated_at_utc": "$(date -u +%Y-%m-%dT%H:%M:%SZ)",
  "source_root": "site/assets",
  "paths": [
    "images/",
    "map/",
    "region_groups/"
  ]
}
EOF

echo "staged CDN payload in $CDN_ROOT"
