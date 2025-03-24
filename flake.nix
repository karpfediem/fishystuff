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
    zig.url = "github:mitchellh/zig-overlay";
    zig2nix.url = "github:Cloudef/zig2nix";
  };

  nixConfig = {
    extra-trusted-public-keys = "devenv.cachix.org-1:w1cLUi8dv3hnoSPGAuibQv+f9TZLr6cv/Hm9XgU50cw=";
    extra-substituters = "https://devenv.cachix.org";
  };

  outputs = inputs@{ self, flake-parts, devenv-root, zig, zig2nix, ... }:
    flake-parts.lib.mkFlake { inherit inputs; } {
      imports = [
        inputs.devenv.flakeModule
      ];
      systems = [ "x86_64-linux" "i686-linux" "x86_64-darwin" "aarch64-linux" "aarch64-darwin" ];

      perSystem = { config, self', inputs', pkgs, system, ... }: {
        packages.default = let env = zig2nix.outputs.zig-env.${system} { zig = zig2nix.outputs.packages.${system}.zig-master; }; in env.package {
            src = env.pkgs.lib.cleanSource ./.;
            nativeBuildInputs = with env.pkgs; [];
            buildInputs = with env.pkgs; [];
            zigPreferMusl = true;
        };

        devenv.shells.default = {
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
      flake = {
      };
    };
}
