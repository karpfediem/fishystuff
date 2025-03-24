{ pkgs, lib, config, inputs, ... }:
let
  pkgs-unstable = import inputs.nixpkgs-unstable { system = pkgs.stdenv.system; };
in
{
  name = "fishystuff";
  packages = with pkgs; [
    flyctl
  ];
  languages = {
    zig = {
      enable = true;
      package = pkgs-unstable.zig;
    };
  };
}
