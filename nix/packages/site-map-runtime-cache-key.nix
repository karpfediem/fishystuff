let
  cargoToml = builtins.path {
    path = ../../Cargo.toml;
    name = "fishystuff-site-map-cache-Cargo.toml";
  };
  cargoLock = builtins.path {
    path = ../../Cargo.lock;
    name = "fishystuff-site-map-cache-Cargo.lock";
  };
  mapRuntimeSrc = builtins.path {
    path = ../../map/fishystuff_ui_bevy;
    name = "fishystuff-site-map-cache-map-runtime-src";
  };
  apiLibSrc = builtins.path {
    path = ../../lib/fishystuff_api;
    name = "fishystuff-site-map-cache-api-lib-src";
  };
  clientLibSrc = builtins.path {
    path = ../../lib/fishystuff_client;
    name = "fishystuff-site-map-cache-client-lib-src";
  };
  coreLibSrc = builtins.path {
    path = ../../lib/fishystuff_core;
    name = "fishystuff-site-map-cache-core-lib-src";
  };
  keyMaterial = builtins.concatStringsSep "\n" [
    (toString cargoToml)
    (toString cargoLock)
    (toString mapRuntimeSrc)
    (toString apiLibSrc)
    (toString clientLibSrc)
    (toString coreLibSrc)
  ];
in
builtins.substring 0 16 (builtins.hashString "sha256" keyMaterial)
