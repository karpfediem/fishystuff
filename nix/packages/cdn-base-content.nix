{ runCommandLocal, repoRoot }:
let
  cdnRoot = repoRoot + "/data/cdn/public";
  metadata = builtins.path {
    path = cdnRoot + "/.cdn-metadata.json";
    name = "cdn-metadata-json";
  };
  fields = builtins.path {
    path = cdnRoot + "/fields";
    name = "cdn-fields";
  };
  map = builtins.path {
    path = cdnRoot + "/map";
    name = "cdn-map";
  };
  hotspots = builtins.path {
    path = cdnRoot + "/hotspots";
    name = "cdn-hotspots";
  };
  waypoints = builtins.path {
    path = cdnRoot + "/waypoints";
    name = "cdn-waypoints";
  };
  itemImages = builtins.path {
    path = cdnRoot + "/images/items";
    name = "cdn-item-images";
  };
  petImages = builtins.path {
    path = cdnRoot + "/images/pets";
    name = "cdn-pet-images";
  };
in
runCommandLocal "cdn-base-content" { } ''
  mkdir -p "$out/images"
  mkdir -p "$out/logs"
  ln -sfn ${metadata} "$out/.cdn-metadata.json"
  ln -sfn ${fields} "$out/fields"
  ln -sfn ${hotspots} "$out/hotspots"
  ln -sfn ${map} "$out/map"
  ln -sfn ${waypoints} "$out/waypoints"
  ln -sfn ${itemImages} "$out/images/items"
  ln -sfn ${petImages} "$out/images/pets"
''
