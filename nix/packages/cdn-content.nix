{ runCommand, cdnBaseContent, cdnMinimapVisual }:
runCommand "cdn-content" { } ''
  mkdir -p "$out/images/tiles"
  ln -sfn ${cdnBaseContent}/.cdn-metadata.json "$out/.cdn-metadata.json"
  ln -sfn ${cdnBaseContent}/fields "$out/fields"
  ln -sfn ${cdnBaseContent}/logs "$out/logs"
  ln -sfn ${cdnBaseContent}/map "$out/map"
  ln -sfn ${cdnBaseContent}/waypoints "$out/waypoints"
  mkdir -p "$out/images"
  ln -sfn ${cdnBaseContent}/images/items "$out/images/items"
  ln -sfn ${cdnMinimapVisual} "$out/images/tiles/minimap_visual"
''
