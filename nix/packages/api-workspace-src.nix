{ runCommandLocal, apiWorkspaceCargoToml, apiWorkspaceCargoLock }:
let
  scopedCargoLock = builtins.path {
    path = apiWorkspaceCargoLock;
    name = "fishystuff-api-Cargo.lock";
  };
  fishystuffServerSrc = builtins.path {
    path = ../../api/fishystuff_server;
    name = "fishystuff-api-fishystuff_server-src";
  };
  fishystuffApiLibSrc = builtins.path {
    path = ../../lib/fishystuff_api;
    name = "fishystuff-lib-fishystuff_api-src";
  };
  fishystuffConfigLibSrc = builtins.path {
    path = ../../lib/fishystuff_config;
    name = "fishystuff-lib-fishystuff_config-src";
  };
  fishystuffCoreLibSrc = builtins.path {
    path = ../../lib/fishystuff_core;
    name = "fishystuff-lib-fishystuff_core-src";
  };
  fishystuffSiteI18nSrc = builtins.path {
    path = ../../site/i18n;
    name = "fishystuff-site-i18n-src";
  };
in
runCommandLocal "fishystuff-api-src" { } ''
  mkdir -p $out/api $out/lib $out/site
  cp ${apiWorkspaceCargoToml} $out/Cargo.toml
  cp ${scopedCargoLock} $out/Cargo.lock
  cp -r ${fishystuffServerSrc} $out/api/fishystuff_server
  cp -r ${fishystuffApiLibSrc} $out/lib/fishystuff_api
  cp -r ${fishystuffConfigLibSrc} $out/lib/fishystuff_config
  cp -r ${fishystuffCoreLibSrc} $out/lib/fishystuff_core
  cp -r ${fishystuffSiteI18nSrc} $out/site/i18n
''
