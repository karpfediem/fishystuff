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
  cfg = config.fishystuff.loki;
  configSource = yamlFormat.generate "fishystuff-loki.yaml" {
    auth_enabled = false;
    server = {
      http_listen_address = cfg.listenAddress;
      http_listen_port = cfg.httpPort;
      grpc_listen_address = cfg.listenAddress;
      grpc_listen_port = cfg.grpcPort;
      log_level = cfg.logLevel;
    };
    common = {
      instance_addr = cfg.listenAddress;
      path_prefix = cfg.dataDir;
      replication_factor = 1;
      ring.kvstore.store = "inmemory";
    };
    query_range.results_cache.cache.embedded_cache = {
      enabled = true;
      max_size_mb = 32;
    };
    limits_config = {
      allow_structured_metadata = true;
      volume_enabled = true;
    };
    schema_config.configs = [
      {
        from = "2024-01-01";
        store = "tsdb";
        object_store = "filesystem";
        schema = "v13";
        index = {
          prefix = "index_";
          period = "24h";
        };
      }
    ];
    storage_config = {
      tsdb_shipper = {
        active_index_directory = "${cfg.dataDir}/tsdb-index";
        cache_location = "${cfg.dataDir}/tsdb-cache";
      };
      filesystem.directory = "${cfg.dataDir}/chunks";
    };
    analytics.reporting_enabled = false;
  };
  serviceArgv = [
    (lib.getExe' cfg.package "loki")
    "-config.file=${configSource}"
  ];
  systemdUnit = systemdBackend.mkSystemdUnit {
    unitName = "fishystuff-loki.service";
    description = "Fishystuff Loki service";
    argv = serviceArgv;
    environment = { };
    environmentFiles = [ ];
    dynamicUser = cfg.dynamicUser;
    supplementaryGroups = cfg.supplementaryGroups;
    workingDirectory = cfg.dataDir;
    after = [ "network-online.target" ];
    wants = [ "network-online.target" ];
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

  options.fishystuff.loki = {
    package = mkOption {
      type = types.package;
      default = pkgs.grafana-loki;
      defaultText = lib.literalExpression "pkgs.grafana-loki";
      description = "Package containing the `loki` executable.";
    };

    configFileName = mkOption {
      type = types.str;
      default = "loki.yaml";
      description = "Bundle-relative name for the Loki config artifact.";
    };

    stateDirectoryName = mkOption {
      type = types.str;
      default = "fishystuff/loki";
      description = "systemd StateDirectory name used for Loki state.";
    };

    dataDir = mkOption {
      type = types.str;
      default = "/var/lib/${cfg.stateDirectoryName}";
      description = "Persistent Loki data directory.";
    };

    listenAddress = mkOption {
      type = types.str;
      default = "127.0.0.1";
      description = "Address for Loki listeners.";
    };

    httpPort = mkOption {
      type = types.port;
      default = 3100;
      description = "TCP port for Loki HTTP.";
    };

    grpcPort = mkOption {
      type = types.port;
      default = 9096;
      description = "TCP port for Loki gRPC.";
    };

    logLevel = mkOption {
      type = types.str;
      default = "info";
      description = "Loki log level.";
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
      id = "fishystuff-loki";

      roots.store = [
        cfg.package
        configSource
        systemdUnit.file
      ];

      artifacts = {
        "exe/main" = helpers.mkArtifact {
          kind = "binary";
          storePath = lib.getExe' cfg.package "loki";
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
