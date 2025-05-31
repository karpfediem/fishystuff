# The importApply argument. Use this to reference things defined locally,
# as opposed to the flake where this is imported.
localFlake:

# Regular module arguments; self, inputs, etc all reference the final user flake,
# where this module was imported.
{ lib, config, self, inputs, ... }:
{
  flake =
    let
      nixpkgs = ({ config, pkgs, lib, ... }:
        {
          nixpkgs.config = import ./nixpkgs-config.nix { inherit lib; };
          nixpkgs.overlays = import ./overlays { inherit inputs lib; };
        });
    in
    {
      nixosModules.nixpkgs = nixpkgs;
    };
  perSystem = { system, ... }: {
    _module.args.pkgs = import inputs.nixpkgs {
      inherit system;
      config = import ./nixpkgs-config.nix { inherit lib; };
      overlays = import ./overlays { inherit inputs lib; };
    };
  };
}
