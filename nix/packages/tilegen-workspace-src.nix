{ runCommandLocal, tilegenWorkspaceCargoToml }:
let
  workspaceCargoLock = builtins.path {
    path = ../../Cargo.lock;
    name = "tilegen-workspace-Cargo.lock";
  };
  coreSrc = builtins.path {
    path = ../../lib/fishystuff_core;
    name = "tilegen-lib-fishystuff_core-src";
  };
  tilegenSrc = builtins.path {
    path = ../../tools/fishystuff_tilegen;
    name = "tilegen-tools-fishystuff_tilegen-src";
  };
  pazifistaSrc = builtins.path {
    path = ../../tools/pazifista;
    name = "tilegen-tools-pazifista-src";
  };
in
runCommandLocal "tilegen-workspace-src" { } ''
  mkdir -p "$out/lib" "$out/tools"
  cp ${tilegenWorkspaceCargoToml} "$out/Cargo.toml"
  cp ${workspaceCargoLock} "$out/Cargo.lock"
  cp -r ${coreSrc} "$out/lib/fishystuff_core"
  cp -r ${tilegenSrc} "$out/tools/fishystuff_tilegen"
  cp -r ${pazifistaSrc} "$out/tools/pazifista"
''
