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
          filteredWaypointsSrc = pkgs.lib.cleanSourceWith {
            name = "waypoints-no-webp";
            src = inputs.waypoints;
            filter = path: type:
              let lower = pkgs.lib.toLower path; in
                !(pkgs.lib.hasSuffix ".webp" lower);
          };
          craneLib = crane.mkLib pkgs;
          apiWorkspaceCargoToml = pkgs.callPackage ./nix/packages/api-workspace-cargo-toml.nix { };
          apiWorkspaceSrc = pkgs.callPackage ./nix/packages/api-workspace-src.nix {
            inherit apiWorkspaceCargoToml;
            apiWorkspaceCargoLock = ./nix/locks/api/Cargo.lock;
          };
          apiCargoSrc = craneLib.cleanCargoSource apiWorkspaceSrc;
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
              "NIX_SSL_CERT_FILE=${pkgs.cacert}/etc/ssl/certs/ca-bundle.crt"
              "SSL_CERT_FILE=${pkgs.cacert}/etc/ssl/certs/ca-bundle.crt"
            ];
          };
        in
        {
          packages = { inherit api api-container bot bot-container; };
        };
      flake = {
        nixosModules = {
          default = import ./nix/modules;
          fishystuff-api = import ./nix/modules/fishystuff-api.nix;
          fishystuff-dolt = import ./nix/modules/fishystuff-dolt.nix;
          fishystuff-caddy-static = import ./nix/modules/fishystuff-caddy-static.nix;
          fishystuff-caddy-proxy = import ./nix/modules/fishystuff-caddy-proxy.nix;
        };
      };
    });
}
