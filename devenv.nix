{ pkgs, lib, config, inputs, ... }:
let
  pkgs-unstable = import inputs.nixpkgs-unstable { system = pkgs.stdenv.system; };
in
{
  name = "fishystuff";
  packages = with pkgs; [
    dolt
    flyctl
    gawk
    just
    xlsx2csv
  ];
  languages = {
    zig = {
      enable = true;
      package = pkgs-unstable.zig;
    };
  };
}
