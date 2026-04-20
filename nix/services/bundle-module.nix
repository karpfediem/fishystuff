{ lib, ... }:
let
  helpers = import ./helpers.nix { inherit lib; };
  inherit (lib) mkOption types;

  activationDirectoryType = types.submodule {
    options = {
      purpose = mkOption {
        type = types.str;
        description = "Semantic purpose of the directory.";
      };

      path = mkOption {
        type = types.str;
        description = "Runtime path of the directory.";
      };

      create = mkOption {
        type = types.bool;
        default = true;
        description = "Whether deployment tooling should create the directory.";
      };

      mode = mkOption {
        type = types.str;
        default = "0755";
        description = "POSIX mode string for the directory.";
      };

      owner = mkOption {
        type = types.nullOr types.str;
        default = null;
        description = "Owning user, when applicable.";
      };

      group = mkOption {
        type = types.nullOr types.str;
        default = null;
        description = "Owning group, when applicable.";
      };
    };
  };

  activationUserType = types.submodule {
    options = {
      name = mkOption {
        type = types.str;
        description = "User account name.";
      };

      group = mkOption {
        type = types.nullOr types.str;
        default = null;
        description = "Primary group for the user.";
      };

      system = mkOption {
        type = types.bool;
        default = true;
        description = "Whether the user should be treated as a system account.";
      };
    };
  };

  activationGroupType = types.submodule {
    options = {
      name = mkOption {
        type = types.str;
        description = "Group name.";
      };
    };
  };

  artifactType = types.submodule {
    options = {
      kind = mkOption {
        type = types.str;
        description = "Artifact kind identifier.";
      };

      storePath = mkOption {
        type = helpers.artifactPathType;
        description = "Immutable store path for the artifact.";
      };

      destination = mkOption {
        type = types.nullOr types.str;
        default = null;
        description = "Optional runtime-relative destination path.";
      };

      executable = mkOption {
        type = types.bool;
        default = false;
        description = "Whether the artifact should be treated as executable.";
      };
    };
  };

  materializationRootType = types.submodule {
    options = {
      handle = mkOption {
        type = types.str;
        description = "Stable identifier for this materialization root.";
      };

      path = mkOption {
        type = helpers.storePathType;
        description = "Store path that deployment tooling may need to materialize.";
      };

      drv = mkOption {
        type = types.nullOr helpers.artifactPathType;
        default = null;
        description = "Optional derivation path that can realize this root when target-side builds are allowed.";
      };

      class = mkOption {
        type = types.str;
        default = "workspace-local";
        description = "Planner-facing path class for deployment policy.";
      };

      acquisition = mkOption {
        type = types.enum [
          "push"
          "substitute"
          "substitute-or-build"
        ];
        default = "push";
        description = "Generic acquisition mode for this root.";
      };

      allowBuild = mkOption {
        type = types.bool;
        default = false;
        description = "Whether a target may build this root locally.";
      };

      required = mkOption {
        type = types.bool;
        default = true;
        description = "Whether this root is required for successful activation.";
      };
    };
  };

  identityType = types.submodule {
    options = {
      user = mkOption {
        type = types.nullOr types.str;
        default = null;
        description = "Runtime user.";
      };

      group = mkOption {
        type = types.nullOr types.str;
        default = null;
        description = "Runtime group.";
      };

      dynamicUser = mkOption {
        type = types.bool;
        default = false;
        description = "Whether the backend should allocate an ephemeral user.";
      };

      supplementaryGroups = mkOption {
        type = types.listOf types.str;
        default = [ ];
        description = "Supplementary groups for the process.";
      };
    };
  };

  restartType = types.submodule {
    options = {
      policy = mkOption {
        type = types.enum [
          "always"
          "on-failure"
          "never"
        ];
        description = "Restart policy.";
      };

      delaySeconds = mkOption {
        type = types.int;
        default = 5;
        description = "Restart delay in seconds.";
      };
    };
  };

  reloadType = types.submodule {
    options = {
      mode = mkOption {
        type = types.enum [
          "none"
          "signal"
          "command"
          "restart"
        ];
        description = "Reload strategy.";
      };

      signal = mkOption {
        type = types.nullOr types.str;
        default = null;
        description = "Signal to send when `mode = signal`.";
      };

      argv = mkOption {
        type = types.listOf types.str;
        default = [ ];
        description = "Command to run when `mode = command`.";
      };
    };
  };

  stopType = types.submodule {
    options = {
      mode = mkOption {
        type = types.enum [
          "signal"
          "command"
        ];
        description = "Stop strategy.";
      };

      signal = mkOption {
        type = types.nullOr types.str;
        default = "TERM";
        description = "Signal to send when `mode = signal`.";
      };

      argv = mkOption {
        type = types.listOf types.str;
        default = [ ];
        description = "Command to run when `mode = command`.";
      };

      timeoutSeconds = mkOption {
        type = types.int;
        default = 30;
        description = "Graceful shutdown timeout in seconds.";
      };
    };
  };

  readinessType = types.submodule {
    options = {
      mode = mkOption {
        type = types.enum [
          "simple"
          "notify"
          "forking"
          "oneshot"
        ];
        description = "Readiness model.";
      };
    };
  };

  runtimeOverlayType = types.submodule {
    options = {
      name = mkOption {
        type = types.str;
        description = "Overlay identifier.";
      };

      targetPath = mkOption {
        type = types.str;
        description = "Runtime destination for the overlay.";
      };

      format = mkOption {
        type = types.str;
        description = "Overlay format identifier.";
      };

      mergeMode = mkOption {
        type = types.str;
        description = "Merge semantics for the overlay.";
      };

      required = mkOption {
        type = types.bool;
        default = false;
        description = "Whether deployment must materialize the overlay.";
      };

      secret = mkOption {
        type = types.bool;
        default = false;
        description = "Whether the overlay may contain secret material.";
      };

      keys = mkOption {
        type = types.listOf types.str;
        default = [ ];
        description = "Known keys carried by the overlay.";
      };

      onChange = mkOption {
        type = types.enum [
          "reload"
          "restart"
          "none"
        ];
        description = "Required action when the overlay changes.";
      };
    };
  };
in
{
  options.bundle = {
    id = mkOption {
      type = types.str;
      default = "";
      description = "Stable bundle identifier.";
    };

    roots.store = mkOption {
      type = types.listOf helpers.storePathType;
      default = [ ];
      description = "Explicit immutable store roots for closure computation.";
    };

    materialization.roots = mkOption {
      type = types.listOf materializationRootType;
      default = [ ];
      description = "Planner-facing materialization policy for selected store roots.";
    };

    artifacts = mkOption {
      type = types.attrsOf artifactType;
      default = { };
      description = "Named immutable runtime artifacts.";
    };

    activation = {
      directories = mkOption {
        type = types.listOf activationDirectoryType;
        default = [ ];
        description = "Directories that deployment tooling should materialize.";
      };

      users = mkOption {
        type = types.listOf activationUserType;
        default = [ ];
        description = "User accounts required by the service.";
      };

      groups = mkOption {
        type = types.listOf activationGroupType;
        default = [ ];
        description = "Groups required by the service.";
      };

      writablePaths = mkOption {
        type = types.listOf types.str;
        default = [ ];
        description = "Writable paths required at runtime.";
      };

      requiredPaths = mkOption {
        type = types.listOf types.str;
        default = [ ];
        description = "Paths that must exist before the service starts.";
      };
    };

    supervision = {
      environment = mkOption {
        type = types.attrsOf types.str;
        default = { };
        description = "Non-secret static environment variables.";
      };

      environmentFiles = mkOption {
        type = types.listOf types.str;
        default = [ ];
        description = "Runtime environment files expected by the process.";
      };

      workingDirectory = mkOption {
        type = types.nullOr types.str;
        default = null;
        description = "Optional working directory.";
      };

      identity = mkOption {
        type = types.nullOr identityType;
        default = null;
        description = "Runtime identity metadata.";
      };

      restart = mkOption {
        type = restartType;
        default = {
          policy = "on-failure";
          delaySeconds = 5;
        };
        description = "Restart policy metadata.";
      };

      reload = mkOption {
        type = reloadType;
        default = {
          mode = "none";
          signal = null;
          argv = [ ];
        };
        description = "Reload strategy metadata.";
      };

      stop = mkOption {
        type = stopType;
        default = {
          mode = "signal";
          signal = "TERM";
          argv = [ ];
          timeoutSeconds = 30;
        };
        description = "Stop strategy metadata.";
      };

      readiness = mkOption {
        type = readinessType;
        default = {
          mode = "simple";
        };
        description = "Readiness metadata.";
      };
    };

    runtimeOverlays = mkOption {
      type = types.listOf runtimeOverlayType;
      default = [ ];
      description = "Mutable runtime overlays managed outside the store.";
    };

    requiredCapabilities = mkOption {
      type = types.listOf types.str;
      default = [ ];
      description = "Backend capabilities required to realize the service.";
    };

    backends = mkOption {
      type = types.attrs;
      default = { };
      description = "Optional backend-specific metadata.";
    };
  };
}
