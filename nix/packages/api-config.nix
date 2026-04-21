{ runCommandLocal }:
let
  apiConfigToml = builtins.path {
    path = ../../api/config.toml;
    name = "fishystuff-api-config.toml";
  };
  secretSpecToml = builtins.path {
    path = ../../secretspec.toml;
    name = "fishystuff-secretspec.toml";
  };
in
runCommandLocal "fishystuff-api-config" { } ''
  mkdir -p $out/etc/fishystuff
  cp ${apiConfigToml} $out/etc/fishystuff/config.toml
  cp ${secretSpecToml} $out/etc/fishystuff/secretspec.toml
''
