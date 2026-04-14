{ runCommandLocal, apiWorkspaceCargoToml, apiWorkspaceCargoLock }:

runCommandLocal "fishystuff-api-src" { } ''
  mkdir -p $out/api $out/lib
  cp ${apiWorkspaceCargoToml} $out/Cargo.toml
  cp ${apiWorkspaceCargoLock} $out/Cargo.lock
  cp -r ${../../api/fishystuff_server} $out/api/fishystuff_server
  cp -r ${../../lib/fishystuff_api} $out/lib/fishystuff_api
  cp -r ${../../lib/fishystuff_config} $out/lib/fishystuff_config
  cp -r ${../../lib/fishystuff_core} $out/lib/fishystuff_core
''
