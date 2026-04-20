{ pkgs }:
{
  config,
  lib,
  ...
}:
let
  helpers = import ./helpers.nix { inherit lib; };
  systemdBackend = import ./systemd-backend.nix { inherit lib pkgs; };
  inherit (lib) mkOption optional types;
  cfg = config.fishystuff.grafana;
  iniSource = pkgs.writeText "fishystuff-grafana.ini" (builtins.readFile cfg.iniSource);
  provisioningSource = pkgs.runCommandLocal "fishystuff-grafana-provisioning" { } ''
    mkdir -p "$out"
    cp -R ${cfg.provisioningSource}/. "$out"/
  '';
  dashboardsSource = pkgs.runCommandLocal "fishystuff-grafana-dashboards" { } ''
    mkdir -p "$out"
    cp -R ${cfg.dashboardsSource}/. "$out"/
  '';
  grafanaExe = lib.getExe' cfg.package "grafana";
  grafanaHome = "${cfg.package}/share/grafana";
  serviceArgv = [
    grafanaExe
    "server"
    "--homepath"
    grafanaHome
    "--config"
    iniSource
  ];
  staticEnvironment = {
    GF_SERVER_HTTP_ADDR = cfg.listenAddress;
    GF_SERVER_HTTP_PORT = toString cfg.port;
    GF_PATHS_DATA = cfg.dataDir;
    GF_PATHS_PROVISIONING = toString provisioningSource;
    GF_DASHBOARDS_DEFAULT_HOME_DASHBOARD_PATH =
      "${toString dashboardsSource}/fishystuff-operator-overview.json";
    GF_ANALYTICS_REPORTING_ENABLED = "false";
    GF_ANALYTICS_CHECK_FOR_UPDATES = "false";
    GF_ANALYTICS_CHECK_FOR_PLUGIN_UPDATES = "false";
    GF_AUTH_ANONYMOUS_ENABLED = "true";
    GF_AUTH_ANONYMOUS_ORG_ROLE = "Viewer";
    GF_AUTH_DISABLE_LOGIN_FORM = "true";
    FISHYSTUFF_GRAFANA_DASHBOARDS_PATH = toString dashboardsSource;
    FISHYSTUFF_DEV_LOKI_HTTP_PORT = toString cfg.lokiPort;
    FISHYSTUFF_DEV_PROMETHEUS_PORT = toString cfg.prometheusPort;
    FISHYSTUFF_DEV_JAEGER_UI_PORT = toString cfg.jaegerPort;
  };
  systemdUnit = systemdBackend.mkSystemdUnit {
    unitName = "fishystuff-grafana.service";
    description = "Fishystuff Grafana service";
    argv = serviceArgv;
    environment = staticEnvironment;
    environmentFiles = [ ];
    dynamicUser = cfg.dynamicUser;
    supplementaryGroups = cfg.supplementaryGroups;
    workingDirectory = cfg.dataDir;
    after = [
      "network-online.target"
      "fishystuff-jaeger.service"
      "fishystuff-loki.service"
      "fishystuff-prometheus.service"
      "fishystuff-vector.service"
    ];
    wants = [
      "network-online.target"
      "fishystuff-jaeger.service"
      "fishystuff-loki.service"
      "fishystuff-prometheus.service"
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

  options.fishystuff.grafana = {
    package = mkOption {
      type = types.package;
      default = pkgs.grafana;
      defaultText = lib.literalExpression "pkgs.grafana";
      description = "Package containing the `grafana-server` executable.";
    };

    iniSource = mkOption {
      type = types.path;
      default = ../../tools/telemetry/grafana.local.ini;
      description = "Grafana ini configuration file.";
    };

    provisioningSource = mkOption {
      type = types.path;
      default = ../../tools/telemetry/grafana/provisioning;
      description = "Grafana provisioning tree.";
    };

    dashboardsSource = mkOption {
      type = types.path;
      default = ../../tools/telemetry/grafana/dashboards;
      description = "Grafana dashboards tree.";
    };

    stateDirectoryName = mkOption {
      type = types.str;
      default = "fishystuff/grafana";
      description = "systemd StateDirectory name used for Grafana state.";
    };

    dataDir = mkOption {
      type = types.str;
      default = "/var/lib/${cfg.stateDirectoryName}";
      description = "Persistent Grafana data directory.";
    };

    listenAddress = mkOption {
      type = types.str;
      default = "127.0.0.1";
      description = "Address for the Grafana UI.";
    };

    port = mkOption {
      type = types.port;
      default = 3000;
      description = "TCP port for the Grafana UI.";
    };

    lokiPort = mkOption {
      type = types.port;
      default = 3100;
      description = "Loki HTTP port used in Grafana provisioning.";
    };

    prometheusPort = mkOption {
      type = types.port;
      default = 9090;
      description = "Prometheus HTTP port used in Grafana provisioning.";
    };

    jaegerPort = mkOption {
      type = types.port;
      default = 16686;
      description = "Jaeger UI port used in Grafana provisioning.";
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
    process.argv = serviceArgv;

    bundle = {
      id = "fishystuff-grafana";

      roots.store = [
        cfg.package
        iniSource
        provisioningSource
        dashboardsSource
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
          path = iniSource;
        })
        (helpers.mkMaterializationRoot {
          handle = "config/provisioning";
          path = provisioningSource;
        })
        (helpers.mkMaterializationRoot {
          handle = "config/dashboards";
          path = dashboardsSource;
        })
        (helpers.mkMaterializationRoot {
          handle = "systemd/unit";
          path = systemdUnit.file;
        })
      ];

      artifacts = {
        "exe/main" = helpers.mkArtifact {
          kind = "binary";
          storePath = grafanaExe;
          executable = true;
        };

        "config/base" = helpers.mkArtifact {
          kind = "config";
          storePath = iniSource;
          destination = "grafana.ini";
        };

        "config/provisioning" = helpers.mkArtifact {
          kind = "config";
          storePath = provisioningSource;
          destination = "provisioning";
        };

        "config/dashboards" = helpers.mkArtifact {
          kind = "config";
          storePath = dashboardsSource;
          destination = "dashboards";
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
        environment = staticEnvironment;
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
