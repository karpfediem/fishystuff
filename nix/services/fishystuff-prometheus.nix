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
  cfg = config.fishystuff.prometheus;
  configSource = yamlFormat.generate "fishystuff-prometheus.yaml" {
    global = {
      scrape_interval = "5s";
      evaluation_interval = "5s";
    };
    scrape_configs = [
      {
        job_name = "vector-metrics";
        static_configs = [
          {
            targets = [ "${cfg.vectorMetricsAddress}:${toString cfg.vectorMetricsPort}" ];
          }
        ];
      }
    ];
  };
  serviceArgv = [
    (lib.getExe' cfg.package "prometheus")
    "--config.file"
    configSource
    "--storage.tsdb.path"
    cfg.dataDir
    "--storage.tsdb.retention.time"
    cfg.retentionTime
    "--web.listen-address"
    "${cfg.listenAddress}:${toString cfg.port}"
  ];
  systemdUnit = systemdBackend.mkSystemdUnit {
    unitName = "fishystuff-prometheus.service";
    description = "Fishystuff Prometheus service";
    argv = serviceArgv;
    environment = { };
    environmentFiles = [ ];
    dynamicUser = cfg.dynamicUser;
    supplementaryGroups = cfg.supplementaryGroups;
    workingDirectory = cfg.dataDir;
    after = [
      "network-online.target"
      "fishystuff-vector.service"
    ];
    wants = [
      "network-online.target"
      "fishystuff-vector.service"
    ];
    restartPolicy = "on-failure";
    restartDelaySeconds = 5;
    serviceLines = [
      "StateDirectory=${cfg.stateDirectoryName}"
      "StateDirectoryMode=0750"
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

  options.fishystuff.prometheus = {
    package = mkOption {
      type = types.package;
      default = pkgs.callPackage ../packages/prometheus-local.nix { };
      description = "Package containing the `prometheus` executable.";
    };

    configFileName = mkOption {
      type = types.str;
      default = "prometheus.yaml";
      description = "Bundle-relative name for the Prometheus config artifact.";
    };

    stateDirectoryName = mkOption {
      type = types.str;
      default = "fishystuff/prometheus";
      description = "systemd StateDirectory name used for Prometheus data.";
    };

    dataDir = mkOption {
      type = types.str;
      default = "/var/lib/${cfg.stateDirectoryName}";
      description = "Persistent Prometheus data directory.";
    };

    listenAddress = mkOption {
      type = types.str;
      default = "127.0.0.1";
      description = "Address for the Prometheus UI.";
    };

    port = mkOption {
      type = types.port;
      default = 9090;
      description = "TCP port for the Prometheus UI.";
    };

    retentionTime = mkOption {
      type = types.str;
      default = "24h";
      description = "Prometheus retention duration.";
    };

    vectorMetricsAddress = mkOption {
      type = types.str;
      default = "127.0.0.1";
      description = "Vector Prometheus exporter address.";
    };

    vectorMetricsPort = mkOption {
      type = types.port;
      default = 9598;
      description = "Vector Prometheus exporter port.";
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
      id = "fishystuff-prometheus";

      roots.store = [
        cfg.package
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
          storePath = lib.getExe' cfg.package "prometheus";
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
        writablePaths = [ cfg.dataDir ];
        requiredPaths = [ ];
      };

      supervision = {
        environment = { };
        environmentFiles = [ ];
        workingDirectory = cfg.dataDir;
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
