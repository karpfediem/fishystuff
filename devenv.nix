{ pkgs, lib, config, inputs, ... }:
let
  pkgs-unstable = import inputs.nixpkgs-unstable { system = pkgs.stdenv.system; };
in
{
  languages = {
    zig = {
      enable = true;
      package = pkgs-unstable.zig;
    };
  };
}
