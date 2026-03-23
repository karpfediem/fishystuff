{ inputs, pkgs, lib, config, ... }:
let
  dbHost = "127.0.0.1";
  dbPort = "3306";
  apiHost = "127.0.0.1";
  apiPort = "8080";
  cdnHost = "127.0.0.1";
  cdnPort = "4040";
  siteHost = "127.0.0.1";
  sitePort = "1990";
  rustHookToolchain = pkgs.symlinkJoin {
    name = "fishystuff-rust-hook-toolchain";
    paths = [
      config.languages.rust.toolchainPackage
      pkgs.stdenv.cc
    ];
  };
in {
  name = "default";

  packages = with pkgs;
    [
      just
      secretspec
      curl
      dolt
      flyctl
      gawk
      hyperfine
      jq
      libX11
      libXcursor
      libXext
      libXi
      libXinerama
      libXrandr
      libxcb
      libxkbcommon
      libxkbfile
      lsof
      mesa
      rsync
      skopeo
      xlsx2csv
      clang
      chromium
      mariadb
      python3Packages.fonttools
      valgrind
      wasm-bindgen-cli_0_2_108
      woff2
      imagemagick
      tailwindcss
      watchexec
      xauth
      xvfb
      xvfb-run
      xxd
      linuxPackages.perf
      (inputs.zine.packages.${pkgs.system}.default.override { zigPreferMusl = true; })
    ];

  languages.python.enable = true;
  languages.javascript.enable = true;
  languages.javascript.bun.enable = true;
  languages.rust = {
    enable = true;
    channel = "stable";
    components = [ "rustc" "cargo" "clippy" "rustfmt" "rust-analyzer" ];
    targets = [ "x86_64-unknown-linux-gnu" "wasm32-unknown-unknown" ];
  };

  git-hooks = {
    enable = true;
    hooks = {
      rustfmt = {
        enable = true;
        packageOverrides = {
          cargo = rustHookToolchain;
          rustfmt = rustHookToolchain;
        };
      };
      clippy = {
        enable = true;
        packageOverrides = {
          cargo = rustHookToolchain;
          clippy = rustHookToolchain;
        };
      };
      cdn-map-runtime = {
        enable = true;
        name = "CDN map runtime";
        entry = "./tools/scripts/check_cdn_map_runtime_assets.sh";
        files = "^(Cargo\\.lock|Cargo\\.toml|devenv\\.nix|lib/fishystuff_(api|client|core)/|map/fishystuff_ui_bevy/|site/assets/map/|site/scripts/(build-public-release|write-runtime-config)\\.mjs|tools/scripts/(build_map|check_cdn_map_runtime_assets|resolve_map_runtime_cache_key|push_bunnycdn|stage_cdn_assets)\\.sh)";
        language = "system";
        pass_filenames = false;
        stages = [ "pre-push" ];
      };
    };
  };

  env = {
    FISHYSTUFF_DEV_DB_PORT = dbPort;
    FISHYSTUFF_DEV_API_PORT = apiPort;
    FISHYSTUFF_DEV_CDN_PORT = cdnPort;
    FISHYSTUFF_DEV_SITE_PORT = sitePort;
    FISHYSTUFF_RUNTIME_API_BASE_URL = "http://${apiHost}:${apiPort}";
    FISHYSTUFF_RUNTIME_CDN_BASE_URL = "http://${cdnHost}:${cdnPort}";
    FISHYSTUFF_RUNTIME_SITE_BASE_URL = "http://${siteHost}:${sitePort}";
    LD_LIBRARY_PATH = lib.makeLibraryPath [
      pkgs.libX11
      pkgs.libXcursor
      pkgs.libXext
      pkgs.libXi
      pkgs.libXinerama
      pkgs.libXrandr
      pkgs.libxcb
      pkgs.libxkbcommon
      pkgs.libxkbfile
    ];
    FISHYSTUFF_CORS_ALLOWED_ORIGINS =
      "https://fishystuff.fish,https://www.fishystuff.fish,http://${siteHost}:${sitePort},http://localhost:${sitePort}";
  };

  processes.db = {
    exec = "./tools/scripts/run_db_server.sh";
    ports.sql.allocate = 3306;
    ready.notify = true;
    ready.timeout = 30;
    env = {
      DB_HOST = dbHost;
      DB_PORT = dbPort;
    };
  };

  processes.map-build = {
    exec = "./tools/scripts/watch_map_runtime.sh";
    ready.notify = true;
    ready.timeout = 300;
  };

  processes.cdn-stage = {
    exec = "./tools/scripts/watch_cdn_stage.sh";
    ready.notify = true;
    ready.timeout = 120;
    after = [ "devenv:processes:map-build" ];
  };

  processes.cdn = {
    exec = "./tools/scripts/run_cdn_server.sh";
    ports.http.allocate = 4040;
    ready.notify = true;
    ready.timeout = 30;
    after = [ "devenv:processes:cdn-stage" ];
    env = {
      CDN_HOST = cdnHost;
      CDN_PORT = cdnPort;
    };
  };

  processes.api = {
    exec = "./tools/scripts/watch_api.sh";
    ports.http.allocate = 8080;
    ready.notify = true;
    ready.timeout = 120;
    after = [ "devenv:processes:db" ];
    env = {
      DB_HOST = dbHost;
      DB_PORT = dbPort;
      API_BIND_HOST = apiHost;
      API_PORT = apiPort;
      SECRETSPEC_API_PROFILE = "api";
    };
  };

  processes.site-tailwind = {
    exec = "./tools/scripts/watch_site_tailwind.sh";
    ready.notify = true;
    ready.timeout = 120;
  };

  processes.site-build = {
    exec = "./tools/scripts/watch_site_release.sh";
    ready.notify = true;
    ready.timeout = 300;
    after = [ "devenv:processes:site-tailwind" ];
  };

  processes.site = {
    exec = "./tools/scripts/run_site_server.sh";
    ports.http.allocate = 1990;
    ready.notify = true;
    ready.timeout = 30;
    after = [
      "devenv:processes:site-build"
      "devenv:processes:cdn"
      "devenv:processes:api"
    ];
    env = {
      SITE_HOST = siteHost;
      SITE_PORT = sitePort;
    };
  };
}
