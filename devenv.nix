{ pkgs, ... }: {
  name = "default";
  packages = with pkgs; [
    just
    dolt
    gawk
    xlsx2csv
  ];
}
