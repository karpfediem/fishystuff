{ lib, config, inputs, ... }: let
  pkgs = import inputs.nixpkgs {
    system = "x86_64-linux";
    overlays = [
      (final: prev: {
        zigpkgs = inputs.zig.packages.${prev.system};
      })
    ];
  };
in {
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
      package = pkgs.zigpkgs.master;
    };
  };
}
