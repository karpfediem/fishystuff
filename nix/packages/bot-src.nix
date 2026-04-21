{ runCommandLocal, botWaypoints }:
let
  botCargoToml = builtins.path {
    path = ../../bot/Cargo.toml;
    name = "fishystuff-bot-Cargo.toml";
  };
  botCargoLock = builtins.path {
    path = ../../bot/Cargo.lock;
    name = "fishystuff-bot-Cargo.lock";
  };
  botBuildRs = builtins.path {
    path = ../../bot/build.rs;
    name = "fishystuff-bot-build.rs";
  };
  botSrcDir = builtins.path {
    path = ../../bot/src;
    name = "fishystuff-bot-src";
  };
in
runCommandLocal "fishystuff-bot-src" { } ''
  mkdir -p $out/src
  cp ${botCargoToml} $out/Cargo.toml
  cp ${botCargoLock} $out/Cargo.lock
  cp ${botBuildRs} $out/build.rs
  cp -r ${botSrcDir}/. $out/src
  cp -r ${botWaypoints}/bdo-fish-waypoints $out/bdo-fish-waypoints
''
