{
  description = "Fishy Stuff - Fishing Guides and Tools for Black Desert";

  inputs = {
    flake-parts.url = "github:hercules-ci/flake-parts";
    nixpkgs.url = "github:nixos/nixpkgs/nixos-25.11";
    nixpkgs-unstable.url = "github:nixos/nixpkgs/nixpkgs-unstable";
    nix2container.url = "github:nlewo/nix2container";
    nix2container.inputs.nixpkgs.follows = "nixpkgs";
    mk-shell-bin.url = "github:rrbutani/nix-mk-shell-bin";

    crane.url = "github:ipetkov/crane";
    rust-overlay.url = "github:oxalica/rust-overlay";
    rust-overlay.inputs = { nixpkgs.follows = "nixpkgs"; };

    waypoints.url = "github:flockenberger/bdo-fish-waypoints";
    waypoints.flake = false;
  };

  outputs = inputs@{ self, flake-parts, crane, ... }:
    flake-parts.lib.mkFlake { inherit inputs; } ({ withSystem, ... }: {
      systems = [ "x86_64-linux" ];

      perSystem = { config, self', inputs', pkgs, system, waypoints, ... }:
        let
          operatorRepoRoot =
            let
              root = builtins.getEnv "FISHYSTUFF_OPERATOR_ROOT";
            in
            if root != "" then
              root
            else
              throw "FISHYSTUFF_OPERATOR_ROOT must be set for operator-local CDN data packages";
          filteredWaypointsSrc = pkgs.lib.cleanSourceWith {
            name = "waypoints-no-webp";
            src = inputs.waypoints;
            filter = path: type:
              let lower = pkgs.lib.toLower path; in
                !(pkgs.lib.hasSuffix ".webp" lower);
          };
          craneLib = crane.mkLib pkgs;
          apiWorkspaceCargoToml = pkgs.callPackage ./nix/packages/api-workspace-cargo-toml.nix { };
          tilegenWorkspaceCargoToml = pkgs.callPackage ./nix/packages/tilegen-workspace-cargo-toml.nix { };
          apiWorkspaceSrc = pkgs.callPackage ./nix/packages/api-workspace-src.nix {
            inherit apiWorkspaceCargoToml;
            apiWorkspaceCargoLock = ./nix/locks/api/Cargo.lock;
          };
          apiCargoSrc = craneLib.cleanCargoSource apiWorkspaceSrc;
          tilegenWorkspaceSrc = pkgs.callPackage ./nix/packages/tilegen-workspace-src.nix {
            inherit tilegenWorkspaceCargoToml;
          };
          minimapDisplayTiles = pkgs.callPackage ./nix/packages/minimap-display-tiles.nix {
            inherit craneLib tilegenWorkspaceSrc;
          };
          minimapSourceTiles = pkgs.callPackage ./nix/packages/minimap-source-tiles.nix {
            repoRoot = operatorRepoRoot;
          };
          cdnBaseContent = pkgs.callPackage ./nix/packages/cdn-base-content.nix {
            repoRoot = operatorRepoRoot;
          };
          cdnMinimapVisual = pkgs.callPackage ./nix/packages/cdn-minimap-visual.nix {
            inherit minimapDisplayTiles minimapSourceTiles;
          };
          botWaypoints = pkgs.callPackage ./nix/packages/bot-waypoints.nix {
            inherit filteredWaypointsSrc;
          };
          botSrc = pkgs.callPackage ./nix/packages/bot-src.nix {
            inherit botWaypoints;
          };
          botCargoSrc = craneLib.cleanCargoSource botSrc;
          bot = craneLib.buildPackage { src = botCargoSrc; };
          bot-container = pkgs.dockerTools.buildLayeredImage {
            name = "crio";
            tag = "latest";
            contents = [ botWaypoints "${bot}/bin" ];
            config.Entrypoint = [ "bot" ];
            config.Env = [ "PATH=${bot}/bin" ];
          };

          api = craneLib.buildPackage {
            pname = "fishystuff_server";
            version = "0.1.0";
            src = apiCargoSrc;
            cargoExtraArgs = "-p fishystuff_server";
          };

          apiConfig = pkgs.callPackage ./nix/packages/api-config.nix { };
          apiEntrypoint = pkgs.callPackage ./nix/packages/api-entrypoint.nix {
            inherit api;
          };
          cdnContent = pkgs.callPackage ./nix/packages/cdn-content.nix {
            inherit cdnBaseContent cdnMinimapVisual;
          };
          apiServiceBaseConfig = pkgs.callPackage ./nix/packages/api-service-base-config.nix { };
          serviceModules = import ./nix/services {
            inherit pkgs;
            lib = pkgs.lib;
          };
          evalService = pkgs.callPackage ./nix/services/eval-service.nix { };
          mkServiceBundle = pkgs.callPackage ./nix/services/mk-service-bundle.nix {
            inherit evalService;
          };
          apiServiceBundle = mkServiceBundle {
            name = "fishystuff-api";
            serviceModule = serviceModules.api;
            configuration.fishystuff.api = {
              package = api;
              baseConfigSource = apiServiceBaseConfig;
            };
          };
          doltServiceBundle = mkServiceBundle {
            name = "fishystuff-dolt";
            serviceModule = serviceModules.dolt;
          };
          edgeServiceBundle = mkServiceBundle {
            name = "fishystuff-edge";
            serviceModule = serviceModules.edge;
            configuration.fishystuff.edge = {
              tlsEnable = true;
              siteAddress = "https://beta.fishystuff.fish";
              apiAddress = "https://api.beta.fishystuff.fish";
              cdnAddress = "https://cdn.beta.fishystuff.fish";
              telemetryAddress = "https://telemetry.beta.fishystuff.fish";
            };
          };
          lokiServiceBundle = mkServiceBundle {
            name = "fishystuff-loki";
            serviceModule = serviceModules.loki;
          };
          otelCollectorServiceBundle = mkServiceBundle {
            name = "fishystuff-otel-collector";
            serviceModule = serviceModules."otel-collector";
          };
          vectorServiceBundle = mkServiceBundle {
            name = "fishystuff-vector";
            serviceModule = serviceModules.vector;
          };
          prometheusServiceBundle = mkServiceBundle {
            name = "fishystuff-prometheus";
            serviceModule = serviceModules.prometheus;
          };
          jaegerServiceBundle = mkServiceBundle {
            name = "fishystuff-jaeger";
            serviceModule = serviceModules.jaeger;
          };
          grafanaServiceBundle = mkServiceBundle {
            name = "fishystuff-grafana";
            serviceModule = serviceModules.grafana;
          };
          serviceBundleChecks = import ./nix/tests/service-bundle-checks.nix {
            inherit
              apiServiceBundle
              doltServiceBundle
              pkgs
              ;
          };
          modularServiceRuntime = pkgs.callPackage ./nix/tests/modular-service-runtime.nix {
            inherit serviceModules;
          };

          api-container = pkgs.dockerTools.buildLayeredImage {
            name = "api-fishystuff-fish";
            tag = "latest";
            contents = [
              apiEntrypoint
              apiConfig
              pkgs.cacert
              pkgs.dockerTools.fakeNss
            ];
            config.Entrypoint = [ "${apiEntrypoint}/bin/fishystuff-api-entrypoint" ];
            config.Env = [
              "API_CONFIG_PATH=${apiConfig}/etc/fishystuff/config.toml"
              "FISHYSTUFF_SECRETSPEC_PATH=${apiConfig}/etc/fishystuff/secretspec.toml"
              "NIX_SSL_CERT_FILE=${pkgs.cacert}/etc/ssl/certs/ca-bundle.crt"
              "SSL_CERT_FILE=${pkgs.cacert}/etc/ssl/certs/ca-bundle.crt"
            ];
          };
        in
        {
          packages = {
            inherit api api-container bot bot-container;
            default = api;
            api-service-base-config = apiServiceBaseConfig;
            api-service-bundle = apiServiceBundle;
            cdn-base-content = cdnBaseContent;
            cdn-content = cdnContent;
            dolt-service-bundle = doltServiceBundle;
            edge-service-bundle = edgeServiceBundle;
            grafana-service-bundle = grafanaServiceBundle;
            jaeger-service-bundle = jaegerServiceBundle;
            loki-service-bundle = lokiServiceBundle;
            otel-collector-service-bundle = otelCollectorServiceBundle;
            minimap-display-tiles = minimapDisplayTiles;
            minimap-source-tiles = minimapSourceTiles;
            prometheus-service-bundle = prometheusServiceBundle;
            vector-service-bundle = vectorServiceBundle;
          };
          checks =
            serviceBundleChecks
            // {
              modular-service-runtime = modularServiceRuntime;
            };
        };
      flake = {
        lib = {
          services = { pkgs }: import ./nix/services {
            inherit pkgs;
            lib = pkgs.lib;
          };
          evalService = { pkgs }: pkgs.callPackage ./nix/services/eval-service.nix { };
          mkServiceBundle =
            { pkgs }:
            pkgs.callPackage ./nix/services/mk-service-bundle.nix {
              evalService = pkgs.callPackage ./nix/services/eval-service.nix { };
            };
        };
      };
    });
}
