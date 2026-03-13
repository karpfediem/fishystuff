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
          waypoints = pkgs.runCommandLocal "filtered-waypoints" { } ''
            mkdir -p $out/bdo-fish-waypoints
            cd ${filteredWaypointsSrc}
            cp -r . $out/bdo-fish-waypoints/
          '';

          botSrc = pkgs.runCommandLocal "bot-combined-src" { } ''
            mkdir -p $out
            cp -r ${./bot}/* ${waypoints}/* $out
          '';

          craneLib = (crane.mkLib pkgs).overrideToolchain (p: p.rust-bin.stable.latest.default);
          bot = craneLib.buildPackage { src = botSrc; };
          bot-container = pkgs.dockerTools.buildLayeredImage {
            name = "crio";
            tag = "latest";
            contents = [ waypoints "${bot}/bin" ];
            config.Entrypoint = [ "bot" ];
            config.Env = [ "PATH=${bot}/bin" ];
          };
        in
        {
          packages = { inherit bot bot-container; };
        };
      flake = { };
    });
}
