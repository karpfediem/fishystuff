{ pkgs, ... }: {
  name = "site";
  packages = with pkgs; [
    just
  ];
  languages = {
    zig = {
      enable = true;
      package = pkgs.zigpkgs.master;
    };
  };
}
