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
      HOME = cfg.homeDir;
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
  remoteBranchFunction = ''
    resolve_remote_branch() {
      local explicit_remote_branch="''${DOLT_REMOTE_BRANCH:-}"
      if [ -n "$explicit_remote_branch" ]; then
        printf '%s' "$explicit_remote_branch"
        return
      fi
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
  '';
  startScript = pkgs.writeShellApplication {
    name = "fishystuff-dolt-start";
    runtimeInputs = [
      cfg.package
      pkgs.coreutils
      pkgs.findutils
      pkgs.gnugrep
    ];
    text = ''
      set -euo pipefail

      ${remoteBranchFunction}

      data_dir=${lib.escapeShellArg cfg.dataDir}
      cfg_dir=${lib.escapeShellArg cfg.cfgDir}
      home_dir=${lib.escapeShellArg cfg.homeDir}
      repo_name=${lib.escapeShellArg cfg.databaseName}
      repo_dir=${lib.escapeShellArg "${cfg.dataDir}/${cfg.databaseName}"}
      remote_url=${lib.escapeShellArg cfg.remoteUrl}
      remote_branch="$(resolve_remote_branch)"
      privilege_file=${lib.escapeShellArg cfg.privilegeFile}
      branch_control_file=${lib.escapeShellArg cfg.branchControlFile}
      repo_user_name=${lib.escapeShellArg cfg.repoUserName}
      repo_user_email=${lib.escapeShellArg cfg.repoUserEmail}
      repo_snapshot_path="''${FISHYSTUFF_DOLT_REPO_SNAPSHOT:-}"

      mkdir -p "$data_dir" "$cfg_dir" "$home_dir"
      export HOME="$home_dir"

      if [ -e "$data_dir/.dolt/noms" ] || [ -e "$data_dir/.dolt/repo_state.json" ] || [ -e "$data_dir/.dolt/config.json" ]; then
        echo "refusing to start: found top-level $data_dir/.dolt; expected repo at $repo_dir/.dolt" >&2
        exit 64
      fi
      rm -rf "$data_dir/.dolt"

      normalize_repo_snapshot() {
        local snapshot_path="$1"

        if [ -d "$snapshot_path/.dolt" ]; then
          printf '%s/.dolt' "$snapshot_path"
          return
        fi

        if [ -d "$snapshot_path/noms" ]; then
          printf '%s' "$snapshot_path"
          return
        fi

        echo "configured Dolt repo snapshot does not look like a .dolt directory: $snapshot_path" >&2
        exit 1
      }

      install_repo_snapshot() {
        local snapshot_path="$1"
        local snapshot_dolt_path="$2"
        local marker_path="$repo_dir/.fishystuff-dolt-snapshot-source"
        local tmp_repo_dir="$data_dir/.''${repo_name}.snapshot-tmp"
        local old_repo_dir="$data_dir/.''${repo_name}.snapshot-old"

        if [ -d "$repo_dir/.dolt" ] && [ "$(cat "$marker_path" 2>/dev/null || true)" = "$snapshot_path" ]; then
          return
        fi

        rm -rf "$tmp_repo_dir" "$old_repo_dir"
        mkdir -p "$tmp_repo_dir"
        cp -R --no-preserve=ownership,mode "$snapshot_dolt_path" "$tmp_repo_dir/.dolt"
        find "$tmp_repo_dir/.dolt" -type d -exec chmod u+rwx,go-rwx {} +
        find "$tmp_repo_dir/.dolt" -type f -exec chmod u+rw,go-rwx {} +
        rm -rf "$tmp_repo_dir/.dolt/temptf" "$tmp_repo_dir/.dolt/tmp"
        find "$tmp_repo_dir/.dolt" -name LOCK -type f -delete
        rm -f "$tmp_repo_dir/.dolt/sql-server.info"
        printf '%s\n' "$snapshot_path" > "$tmp_repo_dir/.fishystuff-dolt-snapshot-source"

        if [ -e "$repo_dir" ]; then
          mv "$repo_dir" "$old_repo_dir"
        fi
        mv "$tmp_repo_dir" "$repo_dir"
        rm -rf "$old_repo_dir"
      }

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

      if [ -n "$repo_snapshot_path" ]; then
        repo_snapshot_dolt_path="$(normalize_repo_snapshot "$repo_snapshot_path")"
        install_repo_snapshot "$repo_snapshot_path" "$repo_snapshot_dolt_path"
      else
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

      cd "$repo_dir"
      exec dolt sql-server --config ${lib.escapeShellArg sqlServerConfig} ${lib.escapeShellArgs cfg.extraArgs}
    '';
  };
  refreshScript = pkgs.writeShellApplication {
    name = "fishystuff-dolt-refresh";
    runtimeInputs = [
      cfg.package
      pkgs.coreutils
      pkgs.gnugrep
    ];
    text = ''
      set -euo pipefail

      ${remoteBranchFunction}

      repo_dir=${lib.escapeShellArg "${cfg.dataDir}/${cfg.databaseName}"}
      repo_name=${lib.escapeShellArg cfg.databaseName}
      home_dir=${lib.escapeShellArg cfg.homeDir}
      sql_host=${lib.escapeShellArg cfg.listenAddress}
      sql_port=${lib.escapeShellArg (toString cfg.port)}
      remote_branch="$(resolve_remote_branch)"
      repo_snapshot_path="''${FISHYSTUFF_DOLT_REPO_SNAPSHOT:-}"

      mkdir -p "$home_dir"
      export HOME="$home_dir"

      if [ -n "$repo_snapshot_path" ]; then
        echo "FISHYSTUFF_DOLT_REPO_SNAPSHOT is set; restart fishystuff-dolt.service to activate $repo_snapshot_path" >&2
        exit 75
      fi

      cd "$repo_dir"

      dolt_sql() {
        dolt --host "$sql_host" --port "$sql_port" --user root --password "" --no-tls --use-db "$repo_name" sql -q "$1"
      }

      ${lib.optionalString cfg.readOnly ''
        restore_read_only() {
          dolt_sql "SET GLOBAL read_only = 1" || true
        }
        trap restore_read_only EXIT
        dolt_sql "SET GLOBAL read_only = 0"
        for _ in 1 2 3 4 5; do
          if dolt_sql "SELECT @@global.read_only" | grep -q '| 0'; then
            break
          fi
          sleep 1
        done
        if ! dolt_sql "SELECT @@global.read_only" | grep -q '| 0'; then
          echo "timed out waiting for Dolt read_only=0 before refresh" >&2
          exit 1
        fi
      ''}
      dolt_sql "CALL DOLT_FETCH('origin')"
      dolt_sql "CALL DOLT_RESET('--hard', 'origin/$remote_branch')"
      ${lib.optionalString cfg.readOnly ''
        dolt_sql "SET GLOBAL read_only = 1"
        trap - EXIT
      ''}
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
    execReloadArgv = [ (lib.getExe refreshScript) ];
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

    homeDir = mkOption {
      type = types.str;
      default = "${cfg.dataDir}/home";
      description = "HOME directory for Dolt helper commands.";
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
      default = null;
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
        refreshScript
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
          handle = "script/refresh";
          path = refreshScript;
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

        "script/refresh" = helpers.mkArtifact {
          kind = "script";
          storePath = lib.getExe refreshScript;
          destination = "refresh";
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
          cfg.homeDir
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
          mode = "command";
          signal = null;
          argv = [ (lib.getExe refreshScript) ];
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
        refreshScript
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
          ExecReload = lib.getExe refreshScript;
        }
        // optionalAttrs (!cfg.dynamicUser) {
          User = cfg.user;
          Group = cfg.group;
        };
    };
  };
}
