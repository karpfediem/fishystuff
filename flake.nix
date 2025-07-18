{
  description = "Fishy Stuff - built with zine-ssg";

  inputs = {
    devenv-root = {
      url = "file+file:///dev/null";
      flake = false;
    };
    flake-parts.url = "github:hercules-ci/flake-parts";
    nixpkgs.url = "github:cachix/devenv-nixpkgs/rolling";
    nixpkgs-unstable.url = "github:nixos/nixpkgs/nixpkgs-unstable";
    devenv.url = "github:cachix/devenv";
    nix2container.url = "github:nlewo/nix2container";
    nix2container.inputs.nixpkgs.follows = "nixpkgs";
    mk-shell-bin.url = "github:rrbutani/nix-mk-shell-bin";

    fenix.url = "github:nix-community/fenix";
    fenix.inputs.nixpkgs.follows = "nixpkgs";
    crane.url = "github:ipetkov/crane";

    # zig.url = "github:mitchellh/zig-overlay";
    zig.url = "github:bandithedoge/zig-overlay"; # provides download mirrors - nightly builds were purged from official zig github

    zig2nix.url = "github:Cloudef/zig2nix";
    zine.url = "github:kristoff-it/zine";
  };

  nixConfig = {
    extra-trusted-public-keys = "devenv.cachix.org-1:w1cLUi8dv3hnoSPGAuibQv+f9TZLr6cv/Hm9XgU50cw=";
    extra-substituters = "https://devenv.cachix.org";
  };

  outputs = inputs@{ self, flake-parts, devenv-root, crane, fenix, ... }:
    flake-parts.lib.mkFlake { inherit inputs; } ({ withSystem, flake-parts-lib, ... }:
      let
        systems = builtins.attrNames inputs.zig.packages;
        inherit (flake-parts-lib) importApply;
        flakeModules.default = importApply ./nixpkgs { inherit withSystem; };
      in
      {
        imports = [
          flakeModules.default
          inputs.devenv.flakeModule
        ];
        inherit systems;

        perSystem = { config, self', inputs', pkgs, system, ... }:
          let
            craneLib = (crane.mkLib pkgs).overrideToolchain fenix.packages.${system}.minimal.toolchain;
            bot = craneLib.buildPackage { src = ./bot; };
            waypoints = ./bot/bdo-fish-waypoints;
            containerfs = pkgs.runCommand "containerfs" { } ''
              mkdir -p $out
              cp ${bot}/bin/bot $out/
              cp -r ${waypoints} $out/bdo-fish-waypoints
            '';
            bot-container = pkgs.dockerTools.buildLayeredImage {
              name = "crio";
              tag = "latest";
              contents = [ containerfs ];
              config = {
                Entrypoint = [ "${containerfs}/bot" ];
                WorkingDir = containerfs;
              };
            };
          in
          {
            packages = { inherit bot bot-container containerfs; };

            devenv.shells =
              let
                devenvRootFileContent = builtins.readFile devenv-root.outPath;
                root = pkgs.lib.mkIf (devenvRootFileContent != "") devenvRootFileContent;
                packages = with pkgs; [ flyctl ];
              in
              {
                default = { devenv = { inherit root; }; imports = [ ({ inherit packages; }) ./devenv.nix ]; };
                site = { devenv = { inherit root; }; imports = [ ({ inherit packages; }) ./site/devenv.nix ]; };
                map = { devenv = { inherit root; }; imports = [ ({ inherit packages; }) ./map/devenv.nix ]; };
                bot = { devenv = { inherit root; }; imports = [ ({ inherit packages; }) ./bot/devenv.nix ]; };
              };
          };
        flake = { };
      });
}
