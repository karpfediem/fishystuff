{ pkgs }:
{
  config,
  lib,
  ...
}:
let
  helpers = import ./helpers.nix { inherit lib; };
  systemdBackend = import ./systemd-backend.nix { inherit lib pkgs; };
  yamlFormat = pkgs.formats.yaml { };
  inherit (lib) mkOption optional types;
  cfg = config.fishystuff.jaeger;
  configSource = yamlFormat.generate "fishystuff-jaeger.yaml" {
    service = {
      extensions = [ "jaeger_storage" "jaeger_query" "healthcheckv2" ];
      pipelines.traces = {
        receivers = [ "otlp" ];
        processors = [ "batch" ];
        exporters = [ "jaeger_storage_exporter" ];
      };
      telemetry = {
        resource."service.name" = "jaeger";
        metrics = {
          level = "detailed";
          readers = [
            {
              pull.exporter.prometheus = {
                host = cfg.metricsListenAddress;
                port = cfg.metricsPort;
              };
            }
          ];
        };
        logs.level = "info";
      };
    };
    extensions = {
      jaeger_query = {
        storage = {
          traces = "local_trace_storage";
          metrics = "local_metrics_storage";
        };
        ui.config_file = cfg.uiConfigSource;
        grpc.endpoint = "${cfg.queryGrpcListenAddress}:${toString cfg.queryGrpcPort}";
        http.endpoint = "${cfg.uiListenAddress}:${toString cfg.uiPort}";
      };
      jaeger_storage = {
        backends.local_trace_storage.memory.max_traces = cfg.maxTraces;
        metric_backends.local_metrics_storage.prometheus = {
          endpoint = "http://${cfg.prometheusAddress}:${toString cfg.prometheusPort}";
          normalize_calls = true;
          normalize_duration = true;
        };
      };
      healthcheckv2 = {
        use_v2 = true;
        http.endpoint = "${cfg.healthListenAddress}:${toString cfg.healthPort}";
      };
    };
    receivers.otlp.protocols = {
      grpc.endpoint = "${cfg.otlpGrpcListenAddress}:${toString cfg.otlpGrpcPort}";
      http.endpoint = "${cfg.otlpHttpListenAddress}:${toString cfg.otlpHttpPort}";
    };
    processors.batch = {
      timeout = "250ms";
      send_batch_size = 32;
    };
    exporters.jaeger_storage_exporter.trace_storage = "local_trace_storage";
  };
  serviceArgv = [
    (lib.getExe' cfg.package "jaeger")
    "--config"
    configSource
  ];
  systemdUnit = systemdBackend.mkSystemdUnit {
    unitName = "fishystuff-jaeger.service";
    description = "Fishystuff Jaeger service";
    argv = serviceArgv;
    environment = { };
    environmentFiles = [ ];
    dynamicUser = cfg.dynamicUser;
    supplementaryGroups = cfg.supplementaryGroups;
    after = [
      "network-online.target"
      "fishystuff-prometheus.service"
    ];
    wants = [
      "network-online.target"
      "fishystuff-prometheus.service"
    ];
    restartPolicy = "on-failure";
    restartDelaySeconds = 5;
    serviceLines = [
      "PrivateTmp=true"
      "PrivateDevices=true"
      "ProtectSystem=strict"
      "ProtectHome=true"
      "ProtectKernelTunables=true"
      "ProtectKernelModules=true"
      "ProtectControlGroups=true"
      "LockPersonality=true"
      "NoNewPrivileges=true"
      "RestrictRealtime=true"
      "RestrictSUIDSGID=true"
      "SystemCallArchitectures=native"
      "UMask=0077"
    ];
  };
in
{
  _class = "service";
  imports = [ ./bundle-module.nix ];

  options.fishystuff.jaeger = {
    package = mkOption {
      type = types.package;
      default = pkgs.callPackage ../packages/jaeger-local.nix { };
      description = "Package containing the `jaeger` executable.";
    };

    configFileName = mkOption {
      type = types.str;
      default = "jaeger.yaml";
      description = "Bundle-relative name for the Jaeger config artifact.";
    };

    uiConfigSource = mkOption {
      type = types.path;
      default = ../../tools/telemetry/jaeger-ui.local.json;
      description = "Jaeger UI config JSON.";
    };

    uiListenAddress = mkOption {
      type = types.str;
      default = "127.0.0.1";
      description = "Address for the Jaeger UI.";
    };

    uiPort = mkOption {
      type = types.port;
      default = 16686;
      description = "TCP port for the Jaeger UI.";
    };

    queryGrpcListenAddress = mkOption {
      type = types.str;
      default = "127.0.0.1";
      description = "Address for the Jaeger query gRPC listener.";
    };

    queryGrpcPort = mkOption {
      type = types.port;
      default = 16685;
      description = "TCP port for the Jaeger query gRPC listener.";
    };

    healthListenAddress = mkOption {
      type = types.str;
      default = "127.0.0.1";
      description = "Address for the Jaeger health endpoint.";
    };

    healthPort = mkOption {
      type = types.port;
      default = 14269;
      description = "TCP port for the Jaeger health endpoint.";
    };

    metricsListenAddress = mkOption {
      type = types.str;
      default = "127.0.0.1";
      description = "Address for Jaeger Prometheus metrics.";
    };

    metricsPort = mkOption {
      type = types.port;
      default = 8888;
      description = "TCP port for Jaeger Prometheus metrics.";
    };

    otlpGrpcListenAddress = mkOption {
      type = types.str;
      default = "127.0.0.1";
      description = "Address for the Jaeger OTLP gRPC endpoint.";
    };

    otlpGrpcPort = mkOption {
      type = types.port;
      default = 4317;
      description = "TCP port for the Jaeger OTLP gRPC endpoint.";
    };

    otlpHttpListenAddress = mkOption {
      type = types.str;
      default = "127.0.0.1";
      description = "Address for the Jaeger OTLP HTTP endpoint.";
    };

    otlpHttpPort = mkOption {
      type = types.port;
      default = 4318;
      description = "TCP port for the Jaeger OTLP HTTP endpoint.";
    };

    prometheusAddress = mkOption {
      type = types.str;
      default = "127.0.0.1";
      description = "Prometheus upstream address used for Jaeger SPM.";
    };

    prometheusPort = mkOption {
      type = types.port;
      default = 9090;
      description = "Prometheus upstream port used for Jaeger SPM.";
    };

    maxTraces = mkOption {
      type = types.int;
      default = 100000;
      description = "Maximum in-memory traces retained by Jaeger.";
    };

    dynamicUser = mkOption {
      type = types.bool;
      default = true;
      description = "Whether a backend may allocate an ephemeral user.";
    };

    supplementaryGroups = mkOption {
      type = types.listOf types.str;
      default = [ ];
      description = "Supplementary runtime groups.";
    };
  };

  config = {
    configData.${cfg.configFileName}.source = configSource;
    process.argv = serviceArgv;

    bundle = {
      id = "fishystuff-jaeger";

      roots.store = [
        cfg.package
        cfg.uiConfigSource
        configSource
        systemdUnit.file
      ];

      materialization.roots = [
        (helpers.mkMaterializationRoot {
          handle = "pkg/main";
          path = cfg.package;
          drv = cfg.package.drvPath;
          class = "upstream-fixed-output";
          acquisition = "substitute-or-build";
          allowBuild = true;
        })
        (helpers.mkMaterializationRoot {
          handle = "config/ui";
          path = cfg.uiConfigSource;
        })
        (helpers.mkMaterializationRoot {
          handle = "config/base";
          path = configSource;
        })
        (helpers.mkMaterializationRoot {
          handle = "systemd/unit";
          path = systemdUnit.file;
        })
      ];

      artifacts = {
        "exe/main" = helpers.mkArtifact {
          kind = "binary";
          storePath = lib.getExe' cfg.package "jaeger";
          executable = true;
        };

        "config/base" = helpers.mkArtifact {
          kind = "config";
          storePath = configSource;
          destination = cfg.configFileName;
        };

        "config/ui" = helpers.mkArtifact {
          kind = "config";
          storePath = cfg.uiConfigSource;
          destination = "jaeger-ui.json";
        };

        "systemd/unit" = systemdUnit.artifact;
      };

      activation = {
        directories = [ ];
        users = [ ];
        groups = [ ];
        writablePaths = [ ];
        requiredPaths = [ ];
      };

      supervision = {
        environment = { };
        environmentFiles = [ ];
        workingDirectory = null;
        identity = {
          user = null;
          group = null;
          dynamicUser = cfg.dynamicUser;
          supplementaryGroups = cfg.supplementaryGroups;
        };
        restart = {
          policy = "on-failure";
          delaySeconds = 5;
        };
        reload = {
          mode = "restart";
          signal = null;
          argv = [ ];
        };
        stop = {
          mode = "signal";
          signal = "TERM";
          argv = [ ];
          timeoutSeconds = 30;
        };
        readiness = {
          mode = "simple";
        };
      };

      runtimeOverlays = [ ];
      requiredCapabilities = optional cfg.dynamicUser "dynamic-user";
      backends.systemd = systemdUnit.backend;
    };
  };
}
