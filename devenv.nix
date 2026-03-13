{ inputs, pkgs, ... }: {
  name = "default";

  packages = with pkgs;
    [
      just
      curl
      dolt
      flyctl
      gawk
      lftp
      lsof
      rsync
      skopeo
      xlsx2csv
      clang
      mariadb
      wasm-bindgen-cli_0_2_108
      imagemagick
      tailwindcss
      watchexec
      (inputs.zine.packages.${pkgs.system}.default.override { zigPreferMusl = true; })
    ];

  dotenv.enable = true;

  env = {
    FISHYSTUFF_DATABASE_URL = "mysql://root@localhost:3306/fishystuff";
    BUNNY_FTP_HOST = "storage.bunnycdn.com";
    BUNNY_FTP_PORT = "21";
    BUNNY_FTP_USER = "fishystuff";
  };

  languages.python.enable = true;
  languages.javascript.enable = true;
  languages.javascript.bun.enable = true;
  languages.rust = {
    enable = true;
    channel = "stable";
    components = [ "rustc" "cargo" "clippy" "rustfmt" "rust-analyzer" ];
    targets = [ "x86_64-unknown-linux-gnu" "wasm32-unknown-unknown" ];
  };

  tasks."cdn:cleanup-before".exec = "./tools/scripts/cleanup_cdn_server.sh";
  tasks."cdn:cleanup-before".before = [ "devenv:processes:cdn" ];

  tasks."cdn:cleanup-after".exec = "./tools/scripts/cleanup_cdn_server.sh";
  tasks."cdn:cleanup-after".after = [ "devenv:processes:cdn@completed" ];

  processes.db.exec = "dolt sql-server --host 127.0.0.1 --port 3306";
  processes.map-build.exec = "./tools/scripts/watch_map_runtime.sh";
  processes.cdn-stage.exec = "./tools/scripts/watch_cdn_stage.sh";
  processes.cdn.exec = "./tools/scripts/run_cdn_server.sh";
  processes.api.exec = "./tools/scripts/watch_api.sh";
  processes.site-build.exec = "cd site && just watch-release";
  processes.site-tailwind.exec = "cd site && just watch-tailwind";
  processes.site.exec = "cd site && just serve-release";
}
