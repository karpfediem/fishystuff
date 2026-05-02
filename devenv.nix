{ inputs, pkgs, lib, config, ... }:
let
  dbHost = "127.0.0.1";
  dbPort = 3306;
  dbUser = "root";
  dbName = "fishystuff";
  apiHost = "127.0.0.1";
  apiPort = 8080;
  vectorApiPort = 8686;
  vectorOtlpGrpcPort = 4817;
  vectorOtlpHttpPort = 4820;
  vectorOtlpPassthroughHttpPort = 4821;
  lokiHttpPort = 3100;
  lokiGrpcPort = 9096;
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
  grafanaPort = 3000;
  cdnHost = "127.0.0.1";
  cdnPort = 4040;
  siteHost = "127.0.0.1";
  sitePort = 1990;
  telemetryHost = "telemetry.localhost";
  siteImmutableFilePattern = "\\.[0-9a-f]{16}\\.(css|js)(\\.map)?$";
  siteStaticFilePattern = "\\.(css|js|mjs|map|svg|png|webp|jpe?g|gif|ico|woff2?|ttf|json|xml|txt|ziggy)$";
  toString = builtins.toString;
  # Paths in this matcher must be content-addressed or otherwise versioned by
  # their path. Browsers may keep them for a year without revalidation.
  cdnImmutablePaths = lib.concatStringsSep " " [
    "/map/runtime-manifest.*.json"
    "/map/fishystuff_ui_bevy.*.js"
    "/map/fishystuff_ui_bevy_bg.*.wasm"
    "/images/items/*.webp"
    "/images/pets/*.webp"
    "/images/tiles/*"
    "/images/terrain/*"
    "/images/terrain_drape/*"
    "/images/terrain_height/*"
    "/images/terrain_fullres/*"
    "/fields/*"
    "/waypoints/*"
  ];
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
      esbuild
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
      grafana
      grafana-loki
      opentelemetry-collector-contrib
      vector
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
      perf
      (pkgs.callPackage ./nix/packages/zine-prebuilt.nix { })
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
        files = "^(Cargo\\.lock|Cargo\\.toml|devenv\\.nix|lib/fishystuff_(api|client|core)/|map/fishystuff_ui_bevy/|site/assets/map/|site/scripts/(finalize-assets|write-runtime-config)\\.mjs|site/scripts/build-public-release\\.sh|tools/scripts/(build_map|check_cdn_map_runtime_assets|check_cdn_map_runtime_assets_pre_push|resolve_map_runtime_cache_key|push_bunnycdn|stage_cdn_assets)\\.sh)";
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
    FISHYSTUFF_DEV_VECTOR_API_PORT = toString vectorApiPort;
    FISHYSTUFF_DEV_VECTOR_OTLP_GRPC_PORT = toString vectorOtlpGrpcPort;
    FISHYSTUFF_DEV_VECTOR_OTLP_HTTP_PORT = toString vectorOtlpHttpPort;
    FISHYSTUFF_DEV_VECTOR_OTLP_PASSTHROUGH_HTTP_PORT = toString vectorOtlpPassthroughHttpPort;
    FISHYSTUFF_DEV_LOKI_HTTP_PORT = toString lokiHttpPort;
    FISHYSTUFF_DEV_LOKI_GRPC_PORT = toString lokiGrpcPort;
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
    FISHYSTUFF_DEV_GRAFANA_PORT = toString grafanaPort;
    FISHYSTUFF_RUNTIME_API_BASE_URL = "http://${apiHost}:${toString apiPort}";
    FISHYSTUFF_RUNTIME_CDN_BASE_URL = "http://${cdnHost}:${toString cdnPort}";
    FISHYSTUFF_RUNTIME_SITE_BASE_URL = "http://${siteHost}:${toString sitePort}";
    DOLT_REMOTE_BRANCH = "beta";
    FISHYSTUFF_LOKI_DATA_DIR = "${config.devenv.root}/data/loki";
    FISHYSTUFF_VECTOR_DATA_DIR = "${config.devenv.root}/data/vector";
    FISHYSTUFF_RUNTIME_OTEL_ENABLED = "true";
    FISHYSTUFF_RUNTIME_TELEMETRY_DEFAULT_MODE = "opt-in";
    FISHYSTUFF_RUNTIME_OTEL_SERVICE_NAME = "fishystuff-site-local";
    FISHYSTUFF_RUNTIME_OTEL_DEPLOYMENT_ENVIRONMENT = "local";
    FISHYSTUFF_RUNTIME_OTEL_SERVICE_VERSION = "dev";
    FISHYSTUFF_RUNTIME_OTEL_EXPORTER_ENDPOINT =
      "http://${telemetryHost}:${toString sitePort}/v1/traces";
    FISHYSTUFF_RUNTIME_OTEL_METRICS_ENABLED = "true";
    FISHYSTUFF_RUNTIME_OTEL_METRICS_ENDPOINT =
      "http://${telemetryHost}:${toString sitePort}/v1/metrics";
    FISHYSTUFF_RUNTIME_OTEL_METRIC_EXPORT_INTERVAL_MS = "5000";
    FISHYSTUFF_RUNTIME_OTEL_LOGS_ENABLED = "true";
    FISHYSTUFF_RUNTIME_OTEL_LOGS_ENDPOINT =
      "http://${telemetryHost}:${toString sitePort}/v1/logs";
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

      @site_runtime path /runtime-config.js /asset-manifest.json /build-info.json
      @site_immutable path_regexp ${siteImmutableFilePattern}
      @site_static path_regexp ${siteStaticFilePattern}

      handle @site_runtime {
        header Cache-Control "no-store"
        file_server
      }

      handle @site_immutable {
        header Cache-Control "public, max-age=31536000, immutable"
        file_server
      }

      handle @site_static {
        header Cache-Control "public, max-age=3600"
        file_server
      }

      handle {
        header Cache-Control "no-store"
        try_files {path} {path}.html {path}/index.html =404
        file_server
      }
    '';
    virtualHosts."http://localhost:${toString sitePort}".extraConfig = ''
      root * ${config.devenv.root}/site/.out

      @site_runtime path /runtime-config.js /asset-manifest.json /build-info.json
      @site_immutable path_regexp ${siteImmutableFilePattern}
      @site_static path_regexp ${siteStaticFilePattern}

      handle @site_runtime {
        header Cache-Control "no-store"
        file_server
      }

      handle @site_immutable {
        header Cache-Control "public, max-age=31536000, immutable"
        file_server
      }

      handle @site_static {
        header Cache-Control "public, max-age=3600"
        file_server
      }

      handle {
        header Cache-Control "no-store"
        try_files {path} {path}.html {path}/index.html =404
        file_server
      }
    '';
    # Local browser OTLP intentionally goes through a Caddy telemetry edge
    # instead of raw Vector. Vector's http_server ingest can accept OTLP, but
    # it does not own the deploy-time CORS contract for telemetry.*. The edge
    # does, both in the intended public topology and in local validation.
    virtualHosts."http://${telemetryHost}:${toString sitePort}".extraConfig = ''
      @telemetry_allowed_origin header_regexp telemetry_allowed_origin Origin ^http://(localhost|127\.0\.0\.1):${toString sitePort}$
      @telemetry_preflight method OPTIONS
      @telemetry_logs path /v1/logs
      @telemetry_otlp path /v1/metrics /v1/traces

      header Vary Origin

      handle @telemetry_preflight {
        header @telemetry_allowed_origin Access-Control-Allow-Origin "{http.request.header.Origin}"
        header @telemetry_allowed_origin Access-Control-Allow-Methods "POST, OPTIONS"
        header @telemetry_allowed_origin Access-Control-Allow-Headers "Content-Type"
        header @telemetry_allowed_origin Access-Control-Max-Age "86400"
        respond "" 204
      }

      handle @telemetry_logs {
        header @telemetry_allowed_origin Access-Control-Allow-Origin "{http.request.header.Origin}"
        header @telemetry_allowed_origin Access-Control-Allow-Methods "POST, OPTIONS"
        header @telemetry_allowed_origin Access-Control-Allow-Headers "Content-Type"
        reverse_proxy ${apiHost}:${toString vectorOtlpHttpPort}
      }

      handle @telemetry_otlp {
        header @telemetry_allowed_origin Access-Control-Allow-Origin "{http.request.header.Origin}"
        header @telemetry_allowed_origin Access-Control-Allow-Methods "POST, OPTIONS"
        header @telemetry_allowed_origin Access-Control-Allow-Headers "Content-Type"
        reverse_proxy ${apiHost}:${toString vectorOtlpPassthroughHttpPort}
      }
    '';
    virtualHosts."http://${cdnHost}:${toString cdnPort}".extraConfig = ''
      root * ${config.devenv.root}/data/cdn/public

      @runtime_manifest path /map/runtime-manifest.json
      @immutable path ${cdnImmutablePaths}

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

      @runtime_manifest path /map/runtime-manifest.json
      @immutable path ${cdnImmutablePaths}

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

  # Keep the local stack on plain process supervision. The inner devenv task
  # runner's readiness probes flap under process-compose and restart healthy
  # services mid-query.
  processes.db = {
    cwd = config.devenv.root;
    exec = ''
      exec env LOG_TS_LABEL=db LOG_TS_FILE=${config.devenv.root}/data/vector/process/db.log ${logTimestampRunner} \
        dolt sql-server --host ${dbHost} --port ${toString dbPort}
    '';
  };

  processes.api = {
    cwd = config.devenv.root;
    exec = ''
      exec env API_BIND_HOST=${apiHost} API_PORT=${toString apiPort} \
        LOG_TS_FILE=${config.devenv.root}/data/vector/process/api.log \
        ${config.devenv.root}/tools/scripts/run_api.sh
    '';
    after = [
      "devenv:processes:db@started"
      "devenv:processes:vector@started"
    ];
  };

  processes.jaeger = {
    cwd = config.devenv.root;
    exec = ''
      exec env LOG_TS_LABEL=jaeger LOG_TS_FILE=${config.devenv.root}/data/vector/process/jaeger.log ${logTimestampRunner} \
        ${jaegerLocal}/bin/jaeger \
        --config ${config.devenv.root}/tools/telemetry/jaeger.local.yaml
    '';
  };

  processes.loki = {
    cwd = config.devenv.root;
    exec = ''
      mkdir -p ${config.devenv.root}/data/loki
      exec env LOG_TS_LABEL=loki LOG_TS_FILE=${config.devenv.root}/data/vector/process/loki.log ${logTimestampRunner} \
        ${pkgs.grafana-loki}/bin/loki \
        -config.file=${config.devenv.root}/tools/telemetry/loki.local.yaml
    '';
  };

  processes.otel-collector = {
    cwd = config.devenv.root;
    exec = ''
      exec env LOG_TS_LABEL=otelcol LOG_TS_FILE=${config.devenv.root}/data/vector/process/otel-collector.log ${logTimestampRunner} \
        ${pkgs.opentelemetry-collector-contrib}/bin/otelcol-contrib \
        --config ${config.devenv.root}/tools/telemetry/otel-collector.local.yaml
    '';
    after = [ "devenv:processes:jaeger@started" ];
  };

  processes.vector = {
    cwd = config.devenv.root;
    exec = ''
      mkdir -p \
        ${config.devenv.root}/data/vector/process \
        ${config.devenv.root}/data/vector/archive/logs \
        ${config.devenv.root}/data/vector/archive/traces \
        ${config.devenv.root}/data/vector/state
      exec env LOG_TS_LABEL=vector LOG_TS_FILE=${config.devenv.root}/data/vector/process/vector.log ${logTimestampRunner} \
        ${pkgs.vector}/bin/vector \
        --config-yaml ${config.devenv.root}/tools/telemetry/vector.local.yaml
    '';
    after = [
      "devenv:processes:loki@started"
      "devenv:processes:otel-collector@started"
    ];
  };

  processes.prometheus = {
    cwd = config.devenv.root;
    exec = ''
      mkdir -p ${config.devenv.root}/data/prometheus
      exec env LOG_TS_LABEL=prometheus LOG_TS_FILE=${config.devenv.root}/data/vector/process/prometheus.log ${logTimestampRunner} \
        ${prometheusLocal}/bin/prometheus \
        --config.file ${config.devenv.root}/tools/telemetry/prometheus.local.yaml \
        --storage.tsdb.path ${config.devenv.root}/data/prometheus \
        --storage.tsdb.retention.time 24h \
        --web.listen-address 127.0.0.1:${toString prometheusPort}
    '';
    after = [ "devenv:processes:vector@started" ];
  };

  processes.grafana = {
    cwd = config.devenv.root;
    exec = ''
      mkdir -p ${config.devenv.root}/data/grafana

      if [ -x ${pkgs.grafana}/bin/grafana-server ]; then
        exec env LOG_TS_LABEL=grafana LOG_TS_FILE=${config.devenv.root}/data/vector/process/grafana.log \
          GF_SERVER_HTTP_ADDR=127.0.0.1 \
          GF_SERVER_HTTP_PORT=${toString grafanaPort} \
          GF_PATHS_DATA=${config.devenv.root}/data/grafana \
          GF_PATHS_PROVISIONING=${config.devenv.root}/tools/telemetry/grafana/provisioning \
          GF_DASHBOARDS_DEFAULT_HOME_DASHBOARD_PATH=${config.devenv.root}/tools/telemetry/grafana/dashboards/fishystuff-operator-overview.json \
          GF_AUTH_ANONYMOUS_ENABLED=true \
          GF_AUTH_ANONYMOUS_ORG_ROLE=Viewer \
          GF_AUTH_DISABLE_LOGIN_FORM=true \
          FISHYSTUFF_GRAFANA_DASHBOARDS_PATH=${config.devenv.root}/tools/telemetry/grafana/dashboards \
          ${logTimestampRunner} \
          ${pkgs.grafana}/bin/grafana-server \
          --homepath ${pkgs.grafana}/share/grafana \
          --config ${config.devenv.root}/tools/telemetry/grafana.local.ini
      fi

      exec env LOG_TS_LABEL=grafana LOG_TS_FILE=${config.devenv.root}/data/vector/process/grafana.log \
        GF_SERVER_HTTP_ADDR=127.0.0.1 \
        GF_SERVER_HTTP_PORT=${toString grafanaPort} \
        GF_PATHS_DATA=${config.devenv.root}/data/grafana \
        GF_PATHS_PROVISIONING=${config.devenv.root}/tools/telemetry/grafana/provisioning \
        GF_DASHBOARDS_DEFAULT_HOME_DASHBOARD_PATH=${config.devenv.root}/tools/telemetry/grafana/dashboards/fishystuff-operator-overview.json \
        GF_AUTH_ANONYMOUS_ENABLED=true \
        GF_AUTH_ANONYMOUS_ORG_ROLE=Viewer \
        GF_AUTH_DISABLE_LOGIN_FORM=true \
        FISHYSTUFF_GRAFANA_DASHBOARDS_PATH=${config.devenv.root}/tools/telemetry/grafana/dashboards \
        ${logTimestampRunner} \
        ${pkgs.grafana}/bin/grafana \
        server \
        --homepath ${pkgs.grafana}/share/grafana \
        --config ${config.devenv.root}/tools/telemetry/grafana.local.ini
    '';
    after = [
      "devenv:processes:jaeger@started"
      "devenv:processes:loki@started"
      "devenv:processes:prometheus@started"
      "devenv:processes:vector@started"
    ];
  };

  profiles.watch.module = {
    processes = {
      api.exec = lib.mkForce ''
        exec env LOG_TS_FILE=${config.devenv.root}/data/vector/process/api.log watchexec -r \
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
          exec env LOG_TS_LABEL=map-build LOG_TS_FILE=${config.devenv.root}/data/vector/process/map-build.log ${logTimestampRunner} watchexec -r --postpone \
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
          exec env LOG_TS_LABEL=cdn-stage LOG_TS_FILE=${config.devenv.root}/data/vector/process/cdn-stage.log ${logTimestampRunner} watchexec -r --postpone \
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
          exec env LOG_TS_LABEL=site-build LOG_TS_FILE=${config.devenv.root}/data/vector/process/site-build.log ${logTimestampRunner} watchexec --postpone --on-busy-update=do-nothing \
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
            --ignore 'site/assets/js/d3.js' \
            --ignore 'site/assets/js/otel.js' \
            --ignore 'site/assets/js/generated/**' \
            --ignore 'site/assets/embed.png' \
            --ignore 'site/assets/img/icons.svg' \
            --ignore 'site/assets/img/embed*.png' \
            --ignore 'site/assets/*/embed.png' \
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
