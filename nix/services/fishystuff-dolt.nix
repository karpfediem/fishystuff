{ pkgs }:
{
  config,
  lib,
  options,
  ...
}:
let
  helpers = import ./helpers.nix { inherit lib; };
  inherit (lib) mkOption optional optionalAttrs types;
  cfg = config.fishystuff.dolt;
  yamlFormat = pkgs.formats.yaml { };
  configName = cfg.configFileName;
  sqlServerConfig = yamlFormat.generate "fishystuff-dolt-sql-server.yaml" {
    log_level = cfg.logLevel;
    behavior = {
      read_only = cfg.readOnly;
    };
    listener = {
      host = cfg.listenAddress;
      port = cfg.port;
    };
    data_dir = cfg.dataDir;
    cfg_dir = cfg.cfgDir;
    privilege_file = cfg.privilegeFile;
    branch_control_file = cfg.branchControlFile;
  };
  runtimeEnvFiles =
    optional (cfg.runtimeEnvFile != null) (toString cfg.runtimeEnvFile)
    ++ map toString cfg.environmentFiles;
in
{
  _class = "service";
  imports = [ ./bundle-module.nix ];

  options.fishystuff.dolt = {
    package = mkOption {
      type = types.package;
      default = pkgs.dolt;
      defaultText = lib.literalExpression "pkgs.dolt";
      description = "Package containing the `dolt` executable.";
    };

    configFileName = mkOption {
      type = types.str;
      default = "sql-server.yaml";
      description = "Bundle-relative name for the immutable Dolt SQL config.";
    };

    dataDir = mkOption {
      type = types.str;
      default = "/var/lib/fishystuff/dolt";
      description = "Persistent Dolt data directory.";
    };

    cfgDir = mkOption {
      type = types.str;
      default = "${cfg.dataDir}/.doltcfg";
      description = "Directory for Dolt SQL runtime metadata.";
    };

    privilegeFile = mkOption {
      type = types.str;
      default = "${cfg.cfgDir}/privileges.db";
      description = "Privilege database path.";
    };

    branchControlFile = mkOption {
      type = types.str;
      default = "${cfg.cfgDir}/branch_control.db";
      description = "Branch control database path.";
    };

    listenAddress = mkOption {
      type = types.str;
      default = "127.0.0.1";
      description = "Address for the Dolt SQL listener.";
    };

    port = mkOption {
      type = types.port;
      default = 3306;
      description = "TCP port for the Dolt SQL listener.";
    };

    readOnly = mkOption {
      type = types.bool;
      default = true;
      description = "Whether to run the Dolt SQL server read-only.";
    };

    logLevel = mkOption {
      type = types.str;
      default = "info";
      description = "Dolt SQL log level.";
    };

    extraArgs = mkOption {
      type = types.listOf types.str;
      default = [ ];
      description = "Additional CLI arguments for `dolt sql-server`.";
    };

    environment = mkOption {
      type = types.attrsOf helpers.envValueType;
      default = { };
      description = "Static non-secret environment variables.";
    };

    environmentFiles = mkOption {
      type = types.listOf helpers.pathLikeType;
      default = [ ];
      description = "Additional runtime environment files.";
    };

    runtimeEnvFile = mkOption {
      type = types.nullOr helpers.pathLikeType;
      default = "/run/fishystuff/dolt/env";
      description = "Primary externally managed runtime environment file.";
    };

    user = mkOption {
      type = types.str;
      default = "fishystuff-dolt";
      description = "Runtime user for Dolt.";
    };

    group = mkOption {
      type = types.str;
      default = "fishystuff-dolt";
      description = "Runtime group for Dolt.";
    };

    supplementaryGroups = mkOption {
      type = types.listOf types.str;
      default = [ ];
      description = "Supplementary runtime groups.";
    };
  };

  config = {
    configData.${configName}.source = sqlServerConfig;

    process.argv = [
      (lib.getExe cfg.package)
      "sql-server"
      "--config"
      sqlServerConfig
    ] ++ cfg.extraArgs;

    bundle = {
      id = "fishystuff-dolt";

      roots.store = [
        cfg.package
        sqlServerConfig
      ];

      artifacts = {
        "exe/main" = helpers.mkArtifact {
          kind = "binary";
          storePath = lib.getExe cfg.package;
          executable = true;
        };

        "config/base" = helpers.mkArtifact {
          kind = "config";
          storePath = sqlServerConfig;
          destination = configName;
        };
      };

      activation = {
        directories = [
          (helpers.mkActivationDirectory {
            purpose = "state";
            path = cfg.dataDir;
            owner = cfg.user;
            group = cfg.group;
            mode = "0750";
          })
          (helpers.mkActivationDirectory {
            purpose = "config";
            path = cfg.cfgDir;
            owner = cfg.user;
            group = cfg.group;
            mode = "0750";
          })
        ];
        users = [
          {
            name = cfg.user;
            group = cfg.group;
            system = true;
          }
        ];
        groups = [ { name = cfg.group; } ];
        writablePaths = [
          cfg.dataDir
          cfg.cfgDir
          cfg.privilegeFile
          cfg.branchControlFile
        ];
        requiredPaths = [ ];
      };

      supervision = {
        environment = helpers.stringifyEnvironment cfg.environment;
        environmentFiles = runtimeEnvFiles;
        workingDirectory = cfg.dataDir;
        identity = {
          user = cfg.user;
          group = cfg.group;
          dynamicUser = false;
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

      runtimeOverlays =
        optional (cfg.runtimeEnvFile != null) (
          helpers.mkRuntimeOverlay {
            name = "runtime-environment";
            targetPath = toString cfg.runtimeEnvFile;
            secret = true;
            onChange = "restart";
          }
        );

      requiredCapabilities = [ "run-as-user" ];
    };
  }
  // optionalAttrs (options ? systemd) {
    systemd.services."" = {
      environment = helpers.stringifyEnvironment cfg.environment;
      restartTriggers = [ sqlServerConfig ];
      serviceConfig = {
        Type = "simple";
        Restart = "on-failure";
        RestartSec = "5s";
        User = cfg.user;
        Group = cfg.group;
        SupplementaryGroups = cfg.supplementaryGroups;
        WorkingDirectory = cfg.dataDir;
        EnvironmentFile =
          optional (cfg.runtimeEnvFile != null) "-${toString cfg.runtimeEnvFile}"
          ++ map toString cfg.environmentFiles;
        PrivateTmp = true;
        PrivateDevices = true;
        ProtectSystem = "strict";
        ProtectHome = true;
        ProtectKernelTunables = true;
        ProtectKernelModules = true;
        ProtectControlGroups = true;
        LockPersonality = true;
        NoNewPrivileges = true;
        RestrictRealtime = true;
        RestrictSUIDSGID = true;
        SystemCallArchitectures = "native";
        UMask = "0077";
        ReadWritePaths = [
          cfg.dataDir
          cfg.cfgDir
        ];
      };
    };

    bundle.backends.systemd = {
      unit = "fishystuff-dolt.service";
    };
  };
}
