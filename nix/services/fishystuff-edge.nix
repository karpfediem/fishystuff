{ pkgs }:
{
  config,
  lib,
  ...
}:
let
  helpers = import ./helpers.nix { inherit lib; };
  systemdBackend = import ./systemd-backend.nix { inherit lib pkgs; };
  inherit (lib) mkOption optional optionalString types;
  cfg = config.fishystuff.edge;
  caddyExe = lib.getExe' cfg.package "caddy";
  cdnImmutablePaths = lib.concatStringsSep " " [
    "/map/runtime-manifest.*.json"
    "/map/fishystuff_ui_bevy.*.js"
    "/map/fishystuff_ui_bevy_bg.*.wasm"
    "/images/items/*.webp"
    "/images/pets/*.webp"
    "/images/tiles/*"
    "/images/terrain/*"
    "/images/terrain_drape/*"
    "/images/terrain_height/*"
    "/images/terrain_fullres/*"
    "/fields/*"
    "/waypoints/*"
  ];
  tlsDirective = optionalString cfg.tlsEnable ''
    tls {$CREDENTIALS_DIRECTORY}/fullchain.pem {$CREDENTIALS_DIRECTORY}/privkey.pem
  '';
  adminAddress = "127.0.0.1:2019";
  caddyfile = pkgs.writeText "fishystuff-edge.Caddyfile" ''
    {
      auto_https off
      admin ${adminAddress}
    }

    ${cfg.siteAddress} {
      ${tlsDirective}
      log {
        output stdout
        format json
      }

      root * ${cfg.siteRoot}
      encode zstd gzip

      @runtime_config path /runtime-config.js
      @site_static path /css/* /js/* /img/*

      handle @runtime_config {
        header Cache-Control "no-store"
        file_server
      }

      handle @site_static {
        header Cache-Control "public, max-age=3600"
        file_server
      }

      handle {
        header Cache-Control "no-store"
        try_files {path} {path}.html {path}/index.html =404
        file_server
      }
    }

    ${cfg.apiAddress} {
      ${tlsDirective}
      log {
        output stdout
        format json
      }

      reverse_proxy ${cfg.apiUpstream}
    }

    ${cfg.cdnAddress} {
      ${tlsDirective}
      log {
        output stdout
        format json
      }

      root * ${cfg.cdnRoot}

      @runtime_manifest path /map/runtime-manifest.json
      @immutable path ${cdnImmutablePaths}

      header Access-Control-Allow-Origin "*"

      handle @runtime_manifest {
        header Cache-Control "no-store"
        file_server
      }

      handle @immutable {
        header Cache-Control "public, max-age=31536000, immutable"
        file_server
      }

      handle {
        header Cache-Control "public, max-age=3600"
        file_server
      }
    }

    ${cfg.telemetryAddress} {
      ${tlsDirective}
      log {
        output stdout
        format json
      }

      @telemetry_preflight method OPTIONS
      @telemetry_logs path /v1/logs
      @telemetry_otlp path /v1/metrics /v1/traces

      header Vary Origin

      handle @telemetry_preflight {
        header Access-Control-Allow-Origin "*"
        header Access-Control-Allow-Methods "POST, OPTIONS"
        header Access-Control-Allow-Headers "Content-Type"
        header Access-Control-Max-Age "86400"
        respond "" 204
      }

      handle @telemetry_logs {
        header Access-Control-Allow-Origin "*"
        header Access-Control-Allow-Methods "POST, OPTIONS"
        header Access-Control-Allow-Headers "Content-Type"
        reverse_proxy ${cfg.telemetryLogsUpstream}
      }

      handle @telemetry_otlp {
        header Access-Control-Allow-Origin "*"
        header Access-Control-Allow-Methods "POST, OPTIONS"
        header Access-Control-Allow-Headers "Content-Type"
        reverse_proxy ${cfg.telemetryOtlpUpstream}
      }
    }
  '';
  serviceArgv = [
    caddyExe
    "run"
    "--config"
    caddyfile
    "--adapter"
    "caddyfile"
  ];
  reloadArgv = [
    caddyExe
    "reload"
    "--config"
    "${caddyfile}"
    "--adapter"
    "caddyfile"
    "--address"
    adminAddress
    "--force"
  ];
  systemdUnit = systemdBackend.mkSystemdUnit {
    unitName = "fishystuff-edge.service";
    description = "Fishystuff public edge";
    argv = serviceArgv;
    environment = { };
    environmentFiles = [ ];
    dynamicUser = cfg.dynamicUser;
    supplementaryGroups = cfg.supplementaryGroups;
    after = [ "network-online.target" ];
    wants = [
      "network-online.target"
      "fishystuff-api.service"
      "fishystuff-vector.service"
    ];
    restartPolicy = "on-failure";
    restartDelaySeconds = 5;
    execReloadArgv = reloadArgv;
    serviceLines =
      lib.optionals cfg.tlsEnable [
        "LoadCredential=fullchain.pem:${cfg.tlsCertificatePath}"
        "LoadCredential=privkey.pem:${cfg.tlsPrivateKeyPath}"
      ]
      ++ [
        "AmbientCapabilities=CAP_NET_BIND_SERVICE"
        "CapabilityBoundingSet=CAP_NET_BIND_SERVICE"
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
        "UMask=0022"
      ];
  };
in
{
  _class = "service";
  imports = [ ./bundle-module.nix ];

  options.fishystuff.edge = {
    package = mkOption {
      type = types.package;
      default = pkgs.caddy;
      defaultText = lib.literalExpression "pkgs.caddy";
      description = "Package containing the `caddy` executable.";
    };

    siteRoot = mkOption {
      type = types.str;
      default = "/srv/fishystuff/site";
      description = "Runtime site root served from the edge.";
    };

    cdnRoot = mkOption {
      type = types.str;
      default = "/srv/fishystuff/cdn";
      description = "Runtime CDN payload root served from the edge.";
    };

    siteAddress = mkOption {
      type = types.str;
      default = "http://beta.fishystuff.fish";
      description = "Public site listener address for Caddy.";
    };

    apiAddress = mkOption {
      type = types.str;
      default = "http://api.beta.fishystuff.fish";
      description = "Public API listener address for Caddy.";
    };

    cdnAddress = mkOption {
      type = types.str;
      default = "http://cdn.beta.fishystuff.fish";
      description = "Public CDN listener address for Caddy.";
    };

    telemetryAddress = mkOption {
      type = types.str;
      default = "http://telemetry.beta.fishystuff.fish";
      description = "Public telemetry listener address for Caddy.";
    };

    tlsEnable = mkOption {
      type = types.bool;
      default = false;
      description = "Whether the edge should terminate TLS with host-provided certificate overlays.";
    };

    tlsCertificatePath = mkOption {
      type = types.str;
      default = "/run/fishystuff/edge/tls/fullchain.pem";
      description = "Runtime path containing the TLS full chain for the public edge.";
    };

    tlsPrivateKeyPath = mkOption {
      type = types.str;
      default = "/run/fishystuff/edge/tls/privkey.pem";
      description = "Runtime path containing the TLS private key for the public edge.";
    };

    apiUpstream = mkOption {
      type = types.str;
      default = "127.0.0.1:8080";
      description = "Loopback upstream for the API runtime.";
    };

    telemetryLogsUpstream = mkOption {
      type = types.str;
      default = "127.0.0.1:4820";
      description = "Loopback upstream for browser OTLP log ingestion.";
    };

    telemetryOtlpUpstream = mkOption {
      type = types.str;
      default = "127.0.0.1:4821";
      description = "Loopback upstream for browser OTLP metrics and traces.";
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
      id = "fishystuff-edge";

      roots.store = [
        cfg.package
        caddyfile
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
          path = caddyfile;
        })
        (helpers.mkMaterializationRoot {
          handle = "systemd/unit";
          path = systemdUnit.file;
        })
      ];

      artifacts = {
        "exe/main" = helpers.mkArtifact {
          kind = "binary";
          storePath = caddyExe;
          executable = true;
        };

        "config/base" = helpers.mkArtifact {
          kind = "config";
          storePath = caddyfile;
          destination = "Caddyfile";
        };

        "systemd/unit" = systemdUnit.artifact;
      };

      activation = {
        directories = [
          (helpers.mkActivationDirectory {
            purpose = "site-root";
            path = "/srv/fishystuff";
            owner = "root";
            group = "root";
            mode = "0755";
          })
          (helpers.mkActivationDirectory {
            purpose = "site-root";
            path = cfg.siteRoot;
            owner = "root";
            group = "root";
            mode = "0755";
          })
          (helpers.mkActivationDirectory {
            purpose = "cdn-root";
            path = cfg.cdnRoot;
            owner = "root";
            group = "root";
            mode = "0755";
          })
        ]
        ++ optional cfg.tlsEnable (
          helpers.mkActivationDirectory {
            purpose = "tls-root";
            path = "/run/fishystuff/edge/tls";
            owner = "root";
            group = "root";
            mode = "0755";
          }
        );
        users = [ ];
        groups = [ ];
        writablePaths = [ ];
        requiredPaths = [
          cfg.siteRoot
          cfg.cdnRoot
        ];
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
          mode = "command";
          signal = null;
          argv = reloadArgv;
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
        lib.optionals cfg.tlsEnable [
          (helpers.mkRuntimeOverlay {
            name = "tls-fullchain";
            targetPath = cfg.tlsCertificatePath;
            format = "pem";
            required = true;
            secret = false;
            onChange = "restart";
          })
          (helpers.mkRuntimeOverlay {
            name = "tls-private-key";
            targetPath = cfg.tlsPrivateKeyPath;
            format = "pem";
            required = true;
            secret = true;
            onChange = "restart";
          })
        ];
      requiredCapabilities = optional cfg.dynamicUser "dynamic-user";
      backends.systemd = systemdUnit.backend;
    };
  };
}
