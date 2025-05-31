{ lib, config, pkgs, ... }: {
  name = "site";
  packages = with pkgs; [
    dolt
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
