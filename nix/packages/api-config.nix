{ runCommandLocal }:

runCommandLocal "fishystuff-api-config" { } ''
  mkdir -p $out/etc/fishystuff
  cp ${../../api/config.toml} $out/etc/fishystuff/config.toml
  cp ${../../secretspec.toml} $out/etc/fishystuff/secretspec.toml
''
