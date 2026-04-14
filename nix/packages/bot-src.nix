{ runCommandLocal, botWaypoints }:

runCommandLocal "fishystuff-bot-src" { } ''
  mkdir -p $out/src
  cp ${../../bot/Cargo.toml} $out/Cargo.toml
  cp ${../../bot/Cargo.lock} $out/Cargo.lock
  cp ${../../bot/build.rs} $out/build.rs
  cp -r ${../../bot/src}/. $out/src
  cp -r ${botWaypoints}/bdo-fish-waypoints $out/bdo-fish-waypoints
''
