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
  cfg = config.fishystuff.otelCollector;
  configSource = yamlFormat.generate "fishystuff-otel-collector.yaml" {
    receivers.otlp.protocols.http.endpoint = "${cfg.listenAddress}:${toString cfg.httpPort}";
    processors.batch = {
      timeout = "250ms";
      send_batch_size = 32;
    };
    connectors.spanmetrics = {
      histogram.unit = "ms";
      dimensions = [
        {
          name = "deployment.environment";
          default = cfg.deploymentEnvironment;
        }
      ];
      metrics_flush_interval = "10s";
    };
    exporters = {
      "otlp/jaeger" = {
        endpoint = "${cfg.jaegerGrpcAddress}:${toString cfg.jaegerGrpcPort}";
        tls.insecure = true;
      };
      prometheus.endpoint = "${cfg.metricsListenAddress}:${toString cfg.spanmetricsPort}";
    };
    extensions.health_check.endpoint = "${cfg.healthListenAddress}:${toString cfg.healthPort}";
    service = {
      extensions = [ "health_check" ];
      pipelines = {
        traces = {
          receivers = [ "otlp" ];
          processors = [ "batch" ];
          exporters = [ "otlp/jaeger" "spanmetrics" ];
        };
        metrics = {
          receivers = [ "otlp" "spanmetrics" ];
          processors = [ "batch" ];
          exporters = [ "prometheus" ];
        };
      };
      telemetry.metrics.level = "none";
    };
  };
  serviceArgv = [
    (lib.getExe' cfg.package "otelcol-contrib")
    "--config"
    configSource
  ];
  systemdUnit = systemdBackend.mkSystemdUnit {
    unitName = "fishystuff-otel-collector.service";
    description = "Fishystuff OpenTelemetry collector";
    argv = serviceArgv;
    environment = { };
    environmentFiles = [ ];
    dynamicUser = cfg.dynamicUser;
    supplementaryGroups = cfg.supplementaryGroups;
    after = [
      "network-online.target"
      "fishystuff-jaeger.service"
    ];
    wants = [
      "network-online.target"
      "fishystuff-jaeger.service"
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

  options.fishystuff.otelCollector = {
    package = mkOption {
      type = types.package;
      default = pkgs.opentelemetry-collector-contrib;
      defaultText = lib.literalExpression "pkgs.opentelemetry-collector-contrib";
      description = "Package containing the `otelcol-contrib` executable.";
    };

    configFileName = mkOption {
      type = types.str;
      default = "otel-collector.yaml";
      description = "Bundle-relative name for the OTEL collector config artifact.";
    };

    listenAddress = mkOption {
      type = types.str;
      default = "127.0.0.1";
      description = "Address for OTLP HTTP ingestion.";
    };

    deploymentEnvironment = mkOption {
      type = types.str;
      default = "beta";
      description = "Fallback deployment environment used by spanmetrics when traces lack one.";
    };

    httpPort = mkOption {
      type = types.port;
      default = 4818;
      description = "TCP port for OTLP HTTP ingestion.";
    };

    healthListenAddress = mkOption {
      type = types.str;
      default = "127.0.0.1";
      description = "Address for the collector health check.";
    };

    healthPort = mkOption {
      type = types.port;
      default = 13133;
      description = "TCP port for the collector health check.";
    };

    metricsListenAddress = mkOption {
      type = types.str;
      default = "127.0.0.1";
      description = "Address for exported spanmetrics.";
    };

    spanmetricsPort = mkOption {
      type = types.port;
      default = 8889;
      description = "TCP port for exported spanmetrics.";
    };

    jaegerGrpcAddress = mkOption {
      type = types.str;
      default = "127.0.0.1";
      description = "Jaeger OTLP gRPC upstream address.";
    };

    jaegerGrpcPort = mkOption {
      type = types.port;
      default = 4317;
      description = "Jaeger OTLP gRPC upstream port.";
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
      id = "fishystuff-otel-collector";

      roots.store = [
        cfg.package
        configSource
        systemdUnit.file
      ];

      materialization.roots = [
        (helpers.mkMaterializationRoot {
          handle = "pkg/main";
          path = cfg.package;
          class = "nixpkgs-generic";
          acquisition = "substitute";
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
          storePath = lib.getExe' cfg.package "otelcol-contrib";
          executable = true;
        };

        "config/base" = helpers.mkArtifact {
          kind = "config";
          storePath = configSource;
          destination = cfg.configFileName;
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
