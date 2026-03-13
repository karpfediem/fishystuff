{ pkgs, ... }: {
  name = "default";
  packages = with pkgs; [
    just
    curl
    dolt
    gawk
    lftp
    xlsx2csv
  ];

  languages.python.enable = true;
}
