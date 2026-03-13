{ pkgs, ... }: {
  name = "default";
  packages = with pkgs; [
    just
    curl
    dolt
    gawk
    lftp
    rsync
    xlsx2csv
  ];

  dotenv.enable = true;

  languages.python.enable = true;
}
