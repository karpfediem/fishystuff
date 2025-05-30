{ lib, config, pkgs, ... }: {
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
