{ craneLib, tilegenWorkspaceSrc }:
let
  cargoSrc = craneLib.cleanCargoSource tilegenWorkspaceSrc;
in
craneLib.buildPackage {
  pname = "minimap-display-tiles";
  version = "0.1.0";
  src = cargoSrc;
  cargoExtraArgs = "-p fishystuff_tilegen --bin minimap_display_tiles";
  doCheck = false;
}
