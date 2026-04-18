{ inputs, pkgs, lib, config, ... }:
let
  dbHost = "127.0.0.1";
  dbPort = 3306;
  dbUser = "root";
  dbName = "fishystuff";
  apiHost = "127.0.0.1";
  apiPort = 8080;
  otelCollectorHttpPort = 4818;
  otelCollectorHealthPort = 13133;
  otelCollectorSpanmetricsPort = 8889;
  jaegerUiPort = 16686;
  jaegerQueryGrpcPort = 16685;
  jaegerHealthPort = 14269;
  jaegerMetricsPort = 8888;
  jaegerOtlpGrpcPort = 4317;
  jaegerOtlpHttpPort = 4318;
  prometheusPort = 9090;
  cdnHost = "127.0.0.1";
  cdnPort = 4040;
  siteHost = "127.0.0.1";
  sitePort = 1990;
  toString = builtins.toString;
  logTimestampRunner =
    "${pkgs.bash}/bin/bash ${config.devenv.root}/tools/scripts/with_log_timestamps.sh";
  rustHookToolchain = pkgs.symlinkJoin {
    name = "fishystuff-rust-hook-toolchain";
    paths = [
      config.languages.rust.toolchainPackage
      pkgs.stdenv.cc
    ];
  };
  jaegerLocal = pkgs.callPackage ./nix/packages/jaeger-local.nix { };
  prometheusLocal = pkgs.callPackage ./nix/packages/prometheus-local.nix { };
in {
  name = "default";

  process.manager.implementation = "process-compose";

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
      opentelemetry-collector-contrib
      rsync
      skopeo
      xlsx2csv
      clang
      chromium
      jaegerLocal
      mariadb
      python3Packages.fonttools
      valgrind
      wasm-bindgen-cli_0_2_108
      woff2
      imagemagick
      watchexec
      xauth
      xvfb
      xvfb-run
      xxd
      linuxPackages.perf
      (inputs.zine.packages.${pkgs.system}.default.override { zigPreferMusl = true; })
      prometheusLocal
    ];

  languages.python = {
    enable = true;
    venv.enable = true;
    uv = {
      enable = true;
      sync.enable = true;
    };
  };
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
        entry = "./tools/scripts/check_cdn_map_runtime_assets_pre_push.sh";
        files = "^(Cargo\\.lock|Cargo\\.toml|devenv\\.nix|lib/fishystuff_(api|client|core)/|map/fishystuff_ui_bevy/|site/assets/map/|site/scripts/(build-public-release|write-runtime-config)\\.mjs|tools/scripts/(build_map|check_cdn_map_runtime_assets|check_cdn_map_runtime_assets_pre_push|resolve_map_runtime_cache_key|push_bunnycdn|stage_cdn_assets)\\.sh)";
        language = "system";
        pass_filenames = false;
        stages = [ "pre-push" ];
      };
    };
  };

  env = {
    FISHYSTUFF_DEV_DB_PORT = toString dbPort;
    FISHYSTUFF_DEV_API_PORT = toString apiPort;
    FISHYSTUFF_DEV_CDN_PORT = toString cdnPort;
    FISHYSTUFF_DEV_SITE_PORT = toString sitePort;
    FISHYSTUFF_DEV_OTEL_COLLECTOR_HTTP_PORT = toString otelCollectorHttpPort;
    FISHYSTUFF_DEV_OTEL_COLLECTOR_HEALTH_PORT = toString otelCollectorHealthPort;
    FISHYSTUFF_DEV_OTEL_SPANMETRICS_PORT = toString otelCollectorSpanmetricsPort;
    FISHYSTUFF_DEV_JAEGER_UI_PORT = toString jaegerUiPort;
    FISHYSTUFF_DEV_JAEGER_QUERY_GRPC_PORT = toString jaegerQueryGrpcPort;
    FISHYSTUFF_DEV_JAEGER_HEALTH_PORT = toString jaegerHealthPort;
    FISHYSTUFF_DEV_JAEGER_METRICS_PORT = toString jaegerMetricsPort;
    FISHYSTUFF_DEV_JAEGER_OTLP_GRPC_PORT = toString jaegerOtlpGrpcPort;
    FISHYSTUFF_DEV_JAEGER_OTLP_HTTP_PORT = toString jaegerOtlpHttpPort;
    FISHYSTUFF_DEV_PROMETHEUS_PORT = toString prometheusPort;
    FISHYSTUFF_RUNTIME_API_BASE_URL = "http://${apiHost}:${toString apiPort}";
    FISHYSTUFF_RUNTIME_CDN_BASE_URL = "http://${cdnHost}:${toString cdnPort}";
    FISHYSTUFF_RUNTIME_SITE_BASE_URL = "http://${siteHost}:${toString sitePort}";
    FISHYSTUFF_RUNTIME_OTEL_ENABLED = "true";
    FISHYSTUFF_RUNTIME_OTEL_SERVICE_NAME = "fishystuff-site-local";
    FISHYSTUFF_RUNTIME_OTEL_DEPLOYMENT_ENVIRONMENT = "local";
    FISHYSTUFF_RUNTIME_OTEL_SERVICE_VERSION = "dev";
    FISHYSTUFF_RUNTIME_OTEL_EXPORTER_ENDPOINT =
      "http://127.0.0.1:${toString otelCollectorHttpPort}/v1/traces";
    FISHYSTUFF_RUNTIME_OTEL_JAEGER_UI_URL = "http://${siteHost}:${toString jaegerUiPort}";
    FISHYSTUFF_RUNTIME_OTEL_SAMPLE_RATIO = "0.25";
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
      "https://fishystuff.fish,http://${siteHost}:${toString sitePort},http://localhost:${toString sitePort}";
  };

  services.caddy = {
    enable = true;
    virtualHosts."http://${siteHost}:${toString sitePort}".extraConfig = ''
      root * ${config.devenv.root}/site/.out
      header Cache-Control "no-store"
      try_files {path} {path}.html {path}/index.html =404
      file_server
    '';
    virtualHosts."http://localhost:${toString sitePort}".extraConfig = ''
      root * ${config.devenv.root}/site/.out
      header Cache-Control "no-store"
      try_files {path} {path}.html {path}/index.html =404
      file_server
    '';
    virtualHosts."http://${cdnHost}:${toString cdnPort}".extraConfig = ''
      root * ${config.devenv.root}/data/cdn/public

      @runtime_manifest path /map/runtime-manifest.json /map/runtime-manifest.*.json
      @immutable path /map/fishystuff_ui_bevy.*.js /map/fishystuff_ui_bevy_bg.*.wasm

      header Access-Control-Allow-Origin "*"

      handle @runtime_manifest {
        header Cache-Control "no-store"
        file_server
      }

      handle @immutable {
        header Cache-Control "public, max-age=31536000, immutable"
        file_server
      }

      handle {
        header Cache-Control "public, max-age=3600"
        file_server
      }
    '';
    virtualHosts."http://localhost:${toString cdnPort}".extraConfig = ''
      root * ${config.devenv.root}/data/cdn/public

      @runtime_manifest path /map/runtime-manifest.json /map/runtime-manifest.*.json
      @immutable path /map/fishystuff_ui_bevy.*.js /map/fishystuff_ui_bevy_bg.*.wasm

      header Access-Control-Allow-Origin "*"

      handle @runtime_manifest {
        header Cache-Control "no-store"
        file_server
      }

      handle @immutable {
        header Cache-Control "public, max-age=31536000, immutable"
        file_server
      }

      handle {
        header Cache-Control "public, max-age=3600"
        file_server
      }
    '';
  };

  processes.db = {
    cwd = config.devenv.root;
    exec = ''
      exec env LOG_TS_LABEL=db ${logTimestampRunner} \
        dolt sql-server --host ${dbHost} --port ${toString dbPort}
    '';
    ready = {
      exec = ''
        mysql --protocol tcp --host ${dbHost} --port ${toString dbPort} --user ${dbUser} ${dbName} --execute "select 1" >/dev/null
      '';
      success_threshold = 3;
      period = 1;
      probe_timeout = 1;
      timeout = 30;
    };
  };

  processes.api = {
    cwd = config.devenv.root;
    exec = ''
      exec env API_BIND_HOST=${apiHost} API_PORT=${toString apiPort} \
        ${config.devenv.root}/tools/scripts/run_api.sh
    '';
    after = [ "devenv:processes:db" "devenv:processes:otel-collector" ];
    ready = {
      exec = ''
        curl -fsS http://${apiHost}:${toString apiPort}/api/v1/meta >/dev/null
      '';
      initial_delay = 1;
      period = 1;
      probe_timeout = 1;
      timeout = 30;
    };
  };

  processes.jaeger = {
    cwd = config.devenv.root;
    exec = ''
      exec env LOG_TS_LABEL=jaeger ${logTimestampRunner} \
        ${jaegerLocal}/bin/jaeger \
        --config ${config.devenv.root}/tools/telemetry/jaeger.local.yaml
    '';
    ready = {
      exec = ''
        curl -fsS http://127.0.0.1:${toString jaegerHealthPort}/status >/dev/null
      '';
      success_threshold = 3;
      period = 1;
      probe_timeout = 1;
      timeout = 30;
    };
  };

  processes.otel-collector = {
    cwd = config.devenv.root;
    exec = ''
      exec env LOG_TS_LABEL=otelcol ${logTimestampRunner} \
        ${pkgs.opentelemetry-collector-contrib}/bin/otelcol-contrib \
        --config ${config.devenv.root}/tools/telemetry/otel-collector.local.yaml
    '';
    after = [ "devenv:processes:jaeger" ];
    ready = {
      exec = ''
        curl -fsS http://127.0.0.1:${toString otelCollectorHealthPort}/ >/dev/null
      '';
      success_threshold = 3;
      period = 1;
      probe_timeout = 1;
      timeout = 30;
    };
  };

  processes.prometheus = {
    cwd = config.devenv.root;
    exec = ''
      mkdir -p ${config.devenv.root}/data/prometheus
      exec env LOG_TS_LABEL=prometheus ${logTimestampRunner} \
        ${prometheusLocal}/bin/prometheus \
        --config.file ${config.devenv.root}/tools/telemetry/prometheus.local.yaml \
        --storage.tsdb.path ${config.devenv.root}/data/prometheus \
        --storage.tsdb.retention.time 24h \
        --web.listen-address 127.0.0.1:${toString prometheusPort}
    '';
    after = [ "devenv:processes:otel-collector" ];
    ready = {
      exec = ''
        curl -fsS http://127.0.0.1:${toString prometheusPort}/-/ready >/dev/null
      '';
      success_threshold = 3;
      period = 1;
      probe_timeout = 1;
      timeout = 30;
    };
  };

  profiles.watch.module = {
    processes = {
      api.exec = lib.mkForce ''
        exec watchexec -r \
          -w api \
          -w lib \
          -w Cargo.toml \
          -w Cargo.lock \
          -w secretspec.toml \
          -w tools/scripts/run_api.sh \
          --exts rs,toml \
          -- ${config.devenv.root}/tools/scripts/run_api.sh
      '';

      api.process-compose.availability.restart = "no";

      map-build = {
        cwd = config.devenv.root;
        exec = ''
          exec watchexec -r --postpone \
            -w map/fishystuff_ui_bevy \
            -w lib/fishystuff_api \
            -w lib/fishystuff_client \
            -w lib/fishystuff_core \
            -w Cargo.toml \
            -w Cargo.lock \
            -w tools/scripts/build_map.sh \
            --exts rs,toml,css \
            -- just build-map
        '';
        process-compose.availability.restart = "no";
      };

      cdn-stage = {
        cwd = config.devenv.root;
        exec = ''
          exec watchexec -r --postpone \
            -w site/assets/map \
            -w tools/scripts/stage_cdn_assets.sh \
            -w tools/scripts/build_item_icons_from_source.mjs \
            --exts js,mjs,css \
            -- just cdn-stage
        '';
        process-compose.availability.restart = "no";
      };

      site-build = {
        cwd = config.devenv.root;
        exec = ''
          exec watchexec -r --postpone \
            -w site/content \
            -w site/layouts \
            -w site/assets \
            -w site/package.json \
            -w site/bun.lock \
            -w site/scripts \
            -w site/Justfile \
            -w site/tailwind.input.css \
            -w site/zine.ziggy \
            --ignore 'site/assets/js/datastar.js' \
            --ignore 'site/assets/img/icons.svg' \
            --ignore 'site/assets/img/guides/*-320.webp' \
            --ignore 'site/assets/img/guides/*-640.webp' \
            --ignore 'site/assets/img/favicon-16x16.png' \
            --ignore 'site/assets/img/favicon-32x32.png' \
            --ignore 'site/assets/img/logo-32.png' \
            --ignore 'site/assets/img/logo-64.png' \
            --ignore 'site/assets/css/fonts/**/*.site.woff2' \
            --ignore 'site/assets/css/site.css' \
            -- just build-site
        '';
        process-compose.availability.restart = "no";
      };
    };
  };
}
