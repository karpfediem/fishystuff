{ pkgs, lib ? pkgs.lib }:
{
  api = lib.modules.importApply ./fishystuff-api.nix { inherit pkgs; };
  dolt = lib.modules.importApply ./fishystuff-dolt.nix { inherit pkgs; };
}
