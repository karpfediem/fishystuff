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
    # zig.url = "github:mitchellh/zig-overlay";
    zig.url = "github:bandithedoge/zig-overlay"; # provides download mirrors - nightly builds were purged from official zig github

    zig2nix.url = "github:Cloudef/zig2nix";
  };

  nixConfig = {
    extra-trusted-public-keys = "devenv.cachix.org-1:w1cLUi8dv3hnoSPGAuibQv+f9TZLr6cv/Hm9XgU50cw=";
    extra-substituters = "https://devenv.cachix.org";
  };

  outputs = inputs@{ self, flake-parts, devenv-root, zig, zig2nix, ... }: let
    overlays = [
      # Other overlays
      (final: prev: {
        zigpkgs = inputs.zig.packages.${prev.system};
        zig = inputs.zig.packages.${prev.system}."2024-12-30";
      })
    ];

    # Our supported systems are the same supported systems as the Zig binaries
    systems = builtins.attrNames inputs.zig.packages;
    in flake-parts.lib.mkFlake { inherit inputs; } {
      imports = [
        inputs.devenv.flakeModule
      ];
      inherit systems;

      perSystem = { config, self', inputs', pkgs, system, ... }: {
        packages.default = let env = zig2nix.outputs.zig-env.${system} { zig = zig2nix.outputs.packages.${system}.zig-master; }; in env.package {
            src = env.pkgs.lib.cleanSource ./.;
            nativeBuildInputs = with env.pkgs; [];
            buildInputs = with env.pkgs; [];
            zigPreferMusl = true;
        };

        devenv.shells = {
          default = {
            devenv.root =
              let
                devenvRootFileContent = builtins.readFile devenv-root.outPath;
              in
              pkgs.lib.mkIf (devenvRootFileContent != "") devenvRootFileContent;

            name = "fishystuff";
            imports = [
              ./devenv.nix
            ];
          };
        };

      };
      flake = {
      };
    };
}
