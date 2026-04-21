{ runCommand, minimapDisplayTiles, minimapSourceTiles }:
runCommand "cdn-minimap-visual" { } ''
  mkdir -p "$out"
  ${minimapDisplayTiles}/bin/minimap_display_tiles \
    --input-dir ${minimapSourceTiles} \
    --out-dir "$out/v1" \
    --tile-px 512 \
    --max-level 2 \
    --root-url /images/tiles/minimap_visual/v1
''
