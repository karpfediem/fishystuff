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
  cfg = config.fishystuff.api;
  apiExe = lib.getExe' cfg.package "fishystuff_server";
  configName = cfg.configFileName;
  configSource = config.configData.${configName}.source;
  runtimeEnvFiles =
    optional (cfg.runtimeEnvFile != null) (toString cfg.runtimeEnvFile)
    ++ map toString cfg.environmentFiles;
in
{
  _class = "service";
  imports = [ ./bundle-module.nix ];

  options.fishystuff.api = {
    package = mkOption {
      type = types.package;
      description = "Package containing the `fishystuff_server` executable.";
    };

    baseConfigSource = mkOption {
      type = types.path;
      default = pkgs.callPackage ../packages/api-service-base-config.nix { };
      description = "Immutable base config for the API process.";
    };

    configFileName = mkOption {
      type = types.str;
      default = "config.toml";
      description = "Bundle-relative name for the immutable base config artifact.";
    };

    listenAddress = mkOption {
      type = types.str;
      default = "127.0.0.1";
      description = "Address for the HTTP listener.";
    };

    port = mkOption {
      type = types.port;
      default = 8080;
      description = "TCP port for the HTTP listener.";
    };

    requestTimeoutSecs = mkOption {
      type = types.nullOr types.int;
      default = null;
      description = "Optional request timeout override.";
    };

    extraArgs = mkOption {
      type = types.listOf types.str;
      default = [ ];
      description = "Additional CLI arguments for `fishystuff_server`.";
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
      default = "/run/fishystuff/api/env";
      description = "Primary externally managed runtime environment file.";
    };

    workingDirectory = mkOption {
      type = types.nullOr helpers.pathLikeType;
      default = null;
      description = "Optional working directory.";
    };

    user = mkOption {
      type = types.str;
      default = "fishystuff-api";
      description = "Runtime user for the API process.";
    };

    group = mkOption {
      type = types.str;
      default = "fishystuff-api";
      description = "Runtime group for the API process.";
    };

    dynamicUser = mkOption {
      type = types.bool;
      default = false;
      description = "Whether a backend may allocate an ephemeral user.";
    };

    supplementaryGroups = mkOption {
      type = types.listOf types.str;
      default = [ ];
      description = "Supplementary runtime groups.";
    };

    requiredPaths = mkOption {
      type = types.listOf types.str;
      default = [ ];
      description = "Extra runtime paths that must exist before launch.";
    };
  };

  config = {
    assertions = [
      {
        assertion = cfg.package != null;
        message = "fishystuff.api.package must be set.";
      }
    ];

    configData.${configName}.source = cfg.baseConfigSource;

    process.argv =
      [
        apiExe
        "--config"
        configSource
        "--bind"
        "${cfg.listenAddress}:${toString cfg.port}"
      ]
      ++ optional (cfg.requestTimeoutSecs != null) "--request-timeout-secs"
      ++ optional (cfg.requestTimeoutSecs != null) (toString cfg.requestTimeoutSecs)
      ++ cfg.extraArgs;

    bundle = {
      id = "fishystuff-api";

      roots.store = [
        cfg.package
        configSource
      ];

      artifacts = {
        "exe/main" = helpers.mkArtifact {
          kind = "binary";
          storePath = apiExe;
          executable = true;
        };

        "config/base" = helpers.mkArtifact {
          kind = "config";
          storePath = configSource;
          destination = configName;
        };
      };

      activation = {
        directories = [ ];
        users = optional (!cfg.dynamicUser) {
          name = cfg.user;
          group = cfg.group;
          system = true;
        };
        groups = optional (!cfg.dynamicUser) { name = cfg.group; };
        writablePaths = [ ];
        requiredPaths = cfg.requiredPaths;
      };

      supervision = {
        environment = helpers.stringifyEnvironment cfg.environment;
        environmentFiles = runtimeEnvFiles;
        workingDirectory =
          if cfg.workingDirectory == null then null else toString cfg.workingDirectory;
        identity = {
          user = cfg.user;
          group = cfg.group;
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

      runtimeOverlays =
        optional (cfg.runtimeEnvFile != null) (
          helpers.mkRuntimeOverlay {
            name = "runtime-environment";
            targetPath = toString cfg.runtimeEnvFile;
            secret = true;
            keys = [
              "FISHYSTUFF_DATABASE_URL"
              "FISHYSTUFF_CORS_ALLOWED_ORIGINS"
              "FISHYSTUFF_RUNTIME_CDN_BASE_URL"
            ];
            onChange = "restart";
          }
        );

      requiredCapabilities =
        optional cfg.dynamicUser "dynamic-user"
        ++ optional (!cfg.dynamicUser) "run-as-user";
    };
  }
  // optionalAttrs (options ? systemd) {
    systemd.services."" = {
      environment = helpers.stringifyEnvironment cfg.environment;
      restartTriggers = [ configSource ];
      serviceConfig =
        {
          Type = "simple";
          Restart = "on-failure";
          RestartSec = "5s";
          EnvironmentFile =
            optional (cfg.runtimeEnvFile != null) "-${toString cfg.runtimeEnvFile}"
            ++ map toString cfg.environmentFiles;
          DynamicUser = cfg.dynamicUser;
          SupplementaryGroups = cfg.supplementaryGroups;
          PrivateTmp = true;
          PrivateDevices = true;
          ProtectSystem = "strict";
          ProtectHome = true;
          ProtectKernelTunables = true;
          ProtectKernelModules = true;
          ProtectControlGroups = true;
          LockPersonality = true;
          MemoryDenyWriteExecute = true;
          NoNewPrivileges = true;
          RestrictRealtime = true;
          RestrictSUIDSGID = true;
          SystemCallArchitectures = "native";
          UMask = "0077";
        }
        // optionalAttrs (!cfg.dynamicUser) {
          User = cfg.user;
          Group = cfg.group;
        }
        // optionalAttrs (cfg.workingDirectory != null) {
          WorkingDirectory = toString cfg.workingDirectory;
        };
    };

    bundle.backends.systemd = {
      unit = "fishystuff-api.service";
    };
  };
}
