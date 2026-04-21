{ pkgs }:
{
  config,
  lib,
  options,
  ...
}:
let
  helpers = import ./helpers.nix { inherit lib; };
  systemdBackend = import ./systemd-backend.nix { inherit lib pkgs; };
  inherit (lib) mkOption optional optionalAttrs types;
  cfg = config.fishystuff.dolt;
  yamlFormat = pkgs.formats.yaml { };
  configName = cfg.configFileName;
  staticEnvironment = helpers.stringifyEnvironment (
    cfg.environment
    // {
      HOME = cfg.dataDir;
    }
  );
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
  startScript = pkgs.writeShellApplication {
    name = "fishystuff-dolt-start";
    runtimeInputs = [
      cfg.package
      pkgs.coreutils
    ];
    text = ''
      set -euo pipefail

      resolve_remote_branch() {
        local deployment_environment="''${FISHYSTUFF_DEPLOYMENT_ENVIRONMENT:-beta}"
        deployment_environment="$(printf '%s' "$deployment_environment" | tr '[:upper:]' '[:lower:]')"
        if [ "$deployment_environment" = "production" ]; then
          printf '%s' "main"
          return
        fi
        if [ -n "$deployment_environment" ]; then
          printf '%s' "$deployment_environment"
          return
        fi
        printf '%s' "beta"
      }

      data_dir=${lib.escapeShellArg cfg.dataDir}
      cfg_dir=${lib.escapeShellArg cfg.cfgDir}
      repo_name=${lib.escapeShellArg cfg.databaseName}
      repo_dir=${lib.escapeShellArg "${cfg.dataDir}/${cfg.databaseName}"}
      remote_url=${lib.escapeShellArg cfg.remoteUrl}
      remote_branch="$(resolve_remote_branch)"
      privilege_file=${lib.escapeShellArg cfg.privilegeFile}
      branch_control_file=${lib.escapeShellArg cfg.branchControlFile}
      repo_user_name=${lib.escapeShellArg cfg.repoUserName}
      repo_user_email=${lib.escapeShellArg cfg.repoUserEmail}

      export HOME="$data_dir"

      mkdir -p "$data_dir" "$cfg_dir"

      clone_remote_repo() {
        rm -rf "$repo_dir"
        clone_cmd=(dolt clone --branch "$remote_branch" --single-branch)
        ${lib.optionalString (cfg.cloneDepth != null) "clone_cmd+=(--depth ${lib.escapeShellArg (toString cfg.cloneDepth)})"}
        clone_cmd+=("$remote_url" "$repo_name")

        (
          cd "$data_dir"
          "''${clone_cmd[@]}"
        )
      }

      current_branch=""
      if [ -d "$repo_dir/.dolt" ]; then
        current_branch="$(
          cd "$repo_dir"
          dolt branch --show-current 2>/dev/null || true
        )"
      fi

      if [ ! -d "$repo_dir/.dolt" ] || [ "$current_branch" != "$remote_branch" ]; then
        clone_remote_repo
      fi

      (
        cd "$repo_dir"

        if ! dolt config --local --get user.name >/dev/null 2>&1; then
          dolt config --local --add user.name "$repo_user_name"
        fi

        if ! dolt config --local --get user.email >/dev/null 2>&1; then
          dolt config --local --add user.email "$repo_user_email"
        fi
      )

      # Keep SQL auth state deterministic across restarts.
      rm -f "$privilege_file" "$branch_control_file"

      exec dolt sql-server --config ${lib.escapeShellArg sqlServerConfig} ${lib.escapeShellArgs cfg.extraArgs}
    '';
  };
  runtimeEnvFiles =
    optional (cfg.runtimeEnvFile != null) (toString cfg.runtimeEnvFile)
    ++ map toString cfg.environmentFiles;
  systemdEnvironmentFiles =
    optional (cfg.runtimeEnvFile != null) "-${toString cfg.runtimeEnvFile}"
    ++ map toString cfg.environmentFiles;
  serviceArgv = [ (lib.getExe startScript) ];
  systemdUnit = systemdBackend.mkSystemdUnit {
    unitName = "fishystuff-dolt.service";
    description = "Fishystuff Dolt SQL service";
    argv = serviceArgv;
    environment = staticEnvironment;
    environmentFiles = systemdEnvironmentFiles;
    user = lib.optionalString (!cfg.dynamicUser) cfg.user;
    group = lib.optionalString (!cfg.dynamicUser) cfg.group;
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

    stateDirectoryName = mkOption {
      type = types.str;
      default = "fishystuff/dolt";
      description = "systemd StateDirectory name used for persistent Dolt state.";
    };

    dataDir = mkOption {
      type = types.str;
      default = "/var/lib/${cfg.stateDirectoryName}";
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

    databaseName = mkOption {
      type = types.str;
      default = "fishystuff";
      description = "Database directory name cloned below the Dolt data root.";
    };

    remoteUrl = mkOption {
      type = types.str;
      default = "fishystuff/fishystuff";
      description = "Upstream Dolt remote to clone when bootstrapping local state.";
    };

    cloneDepth = mkOption {
      type = types.nullOr types.int;
      default = 1;
      description = "Optional shallow-clone depth for the initial local repo bootstrap.";
    };

    repoUserName = mkOption {
      type = types.str;
      default = "fishystuff api";
      description = "Local Dolt repo user.name used when bootstrapping repository config.";
    };

    repoUserEmail = mkOption {
      type = types.str;
      default = "api@fishystuff.fish";
      description = "Local Dolt repo user.email used when bootstrapping repository config.";
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
      default = null;
      description = "Optional externally managed runtime environment file.";
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

    dynamicUser = mkOption {
      type = types.bool;
      default = true;
      description = "Whether a backend may allocate an ephemeral user.";
    };
  };

  config = {
    configData.${configName}.source = sqlServerConfig;

    process.argv = serviceArgv;

    bundle = {
      id = "fishystuff-dolt";

      roots.store = [
        cfg.package
        sqlServerConfig
        startScript
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
          path = sqlServerConfig;
        })
        (helpers.mkMaterializationRoot {
          handle = "script/start";
          path = startScript;
        })
        (helpers.mkMaterializationRoot {
          handle = "systemd/unit";
          path = systemdUnit.file;
        })
      ];

      artifacts = {
        "exe/main" = helpers.mkArtifact {
          kind = "binary";
          storePath = lib.getExe startScript;
          executable = true;
        };

        "exe/dolt" = helpers.mkArtifact {
          kind = "binary";
          storePath = lib.getExe cfg.package;
          executable = true;
        };

        "config/base" = helpers.mkArtifact {
          kind = "config";
          storePath = sqlServerConfig;
          destination = configName;
        };

        "script/start" = helpers.mkArtifact {
          kind = "script";
          storePath = lib.getExe startScript;
          destination = "start";
          executable = true;
        };

        "systemd/unit" = systemdUnit.artifact;
      };

      activation = {
        directories = [ ];
        users = optional (!cfg.dynamicUser) {
          name = cfg.user;
          group = cfg.group;
          system = true;
        };
        groups = optional (!cfg.dynamicUser) { name = cfg.group; };
        writablePaths = [
          cfg.dataDir
          cfg.cfgDir
          cfg.privilegeFile
          cfg.branchControlFile
        ];
        requiredPaths = [ ];
      };

      supervision = {
        environment = staticEnvironment;
        environmentFiles = runtimeEnvFiles;
        workingDirectory = cfg.dataDir;
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
            onChange = "restart";
          }
        );

      requiredCapabilities =
        optional cfg.dynamicUser "dynamic-user"
        ++ optional (!cfg.dynamicUser) "run-as-user";

      backends.systemd = systemdUnit.backend;
    };
  }
  // optionalAttrs (options ? systemd) {
    systemd.services."" = {
      environment = staticEnvironment;
      restartTriggers = [
        sqlServerConfig
        startScript
      ];
      serviceConfig =
        {
          Type = "simple";
          Restart = "on-failure";
          RestartSec = "5s";
          DynamicUser = cfg.dynamicUser;
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
          StateDirectory = cfg.stateDirectoryName;
          StateDirectoryMode = "0750";
        }
        // optionalAttrs (!cfg.dynamicUser) {
          User = cfg.user;
          Group = cfg.group;
        };
    };
  };
}
