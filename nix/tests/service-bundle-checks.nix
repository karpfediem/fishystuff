{
  pkgs,
  lib ? pkgs.lib,
  apiServiceBundle,
  apiServiceBundleProduction,
  doltServiceBundle,
  doltServiceBundleProduction,
  edgeServiceBundle,
  edgeServiceBundleBetaGitopsHandoff,
  edgeServiceBundleProduction,
  edgeServiceBundleProductionGitopsHandoff,
  vectorAgentServiceBundle,
  vectorAgentServiceBundleProduction,
}:
let
  mkBundleCheck =
    {
      name,
      bundle,
      serviceId,
      configDestination,
      runtimeEnvTarget ? null,
      unitName,
      requireSecretSpecPath ? false,
      workingDirectory ? null,
      minArgvLength ? 1,
      requiredEnvironment ? { },
      requiredConfigLines ? [ ],
      forbiddenConfigFragments ? [ ],
      requiredUnitLines ? [ ],
      forbiddenUnitLines ? [ ],
      requiredBundleJsonChecks ? [ ],
      expectedReloadMode ? "restart",
      expectedRuntimeOverlayCount ? 0,
      requiredMaterializationAcquisition ? null,
      requiredMaterializationHandle ? null,
    }:
    pkgs.runCommand name
      {
        nativeBuildInputs = [ pkgs.jq ];
      }
      ''
        bundle_json=${bundle}/bundle.json
        store_paths=${bundle}/store-paths

        jq -e '.id == "${serviceId}"' "$bundle_json" >/dev/null
        jq -e '.roots.store | length >= 2' "$bundle_json" >/dev/null
        jq -e '.artifacts["exe/main"].kind == "binary"' "$bundle_json" >/dev/null
        jq -e '.artifacts["config/base"].kind == "config"' "$bundle_json" >/dev/null
        jq -e '.artifacts["config/base"].destination == "${configDestination}"' "$bundle_json" >/dev/null
        jq -e 'if .id == "fishystuff-dolt" then .artifacts["script/refresh"].kind == "script" else true end' "$bundle_json" >/dev/null
        jq -e '.artifacts["systemd/unit"].kind == "systemd-unit"' "$bundle_json" >/dev/null
        jq -e '.artifacts["systemd/unit"].destination == "${unitName}"' "$bundle_json" >/dev/null
        jq -e '.artifacts["systemd/unit"].bundle_path == "artifacts/systemd/unit"' "$bundle_json" >/dev/null
        if jq -e '.id == "fishystuff-dolt"' "$bundle_json" >/dev/null; then
          jq -e '.artifacts["script/refresh"].bundle_path == "artifacts/script/refresh"' "$bundle_json" >/dev/null
        fi
        jq -e '.bundle_files.bundle_json == "bundle.json"' "$bundle_json" >/dev/null
        jq -e '.bundle_files.materialization_json == "materialization.json"' "$bundle_json" >/dev/null
        jq -e '.bundle_files.mode_substitute == "mode-substitute.txt"' "$bundle_json" >/dev/null
        jq -e '.bundle_files.mode_realise == "mode-realise.txt"' "$bundle_json" >/dev/null
        jq -e '.bundle_files.registration == "registration"' "$bundle_json" >/dev/null
        jq -e '.bundle_files.store_paths == "store-paths"' "$bundle_json" >/dev/null
        jq -e '.bundle_files.mode_verify == "mode-verify.txt"' "$bundle_json" >/dev/null
        jq -e '.materialization.schema_version == 1' "$bundle_json" >/dev/null
        jq -e '.materialization.roots | length > 0' "$bundle_json" >/dev/null
        jq -e '[.materialization.roots[] | select(.allow_build == true) | .drv_path] | all(. != null)' "$bundle_json" >/dev/null
        jq -e '.closure.materialization_file == "materialization.json"' "$bundle_json" >/dev/null
        jq -e '.closure.mode_substitute_file == "mode-substitute.txt"' "$bundle_json" >/dev/null
        jq -e '.closure.mode_realise_file == "mode-realise.txt"' "$bundle_json" >/dev/null
        jq -e '.closure.registration_file == "registration"' "$bundle_json" >/dev/null
        jq -e '.closure.store_paths_file == "store-paths"' "$bundle_json" >/dev/null
        jq -e '.closure.mode_verify_file == "mode-verify.txt"' "$bundle_json" >/dev/null
        jq -e '.supervision.argv | length >= ${toString minArgvLength}' "$bundle_json" >/dev/null
        jq -e '.supervision.restart.policy == "on-failure"' "$bundle_json" >/dev/null
        jq -e '.supervision.reload.mode == "${expectedReloadMode}"' "$bundle_json" >/dev/null
        jq -e '.backends.systemd.service_manager == "systemd"' "$bundle_json" >/dev/null
        jq -e '.backends.systemd.daemon_reload == true' "$bundle_json" >/dev/null
        jq -e '.backends.systemd.units | length == 1' "$bundle_json" >/dev/null
        jq -e '.backends.systemd.units[0].name == "${unitName}"' "$bundle_json" >/dev/null
        jq -e '.backends.systemd.units[0].install_path == "/etc/systemd/system/${unitName}"' "$bundle_json" >/dev/null
        jq -e '.backends.systemd.units[0].artifact == "systemd/unit"' "$bundle_json" >/dev/null
        jq -e '.backends.systemd.units[0].startup == "enabled"' "$bundle_json" >/dev/null
        jq -e '.backends.systemd.units[0].state == "running"' "$bundle_json" >/dev/null

        exe_path=$(jq -r '.artifacts["exe/main"].storePath' "$bundle_json")
        config_path=$(jq -r '.artifacts["config/base"].storePath' "$bundle_json")
        unit_path=$(jq -r '.artifacts["systemd/unit"].storePath' "$bundle_json")
        exe_root=$(printf '%s\n' "$exe_path" | cut -d/ -f1-4)
        grep -Fx "$exe_root" "$store_paths" >/dev/null
        grep -Fx "$config_path" "$store_paths" >/dev/null
        grep -Fx "$unit_path" "$store_paths" >/dev/null
        test -L "${bundle}/artifacts/systemd/unit"
        test "$(readlink -f "${bundle}/artifacts/systemd/unit")" = "$unit_path"
        if jq -e '.id == "fishystuff-dolt"' "$bundle_json" >/dev/null; then
          refresh_path=$(jq -r '.artifacts["script/refresh"].storePath' "$bundle_json")
          refresh_root=$(printf '%s\n' "$refresh_path" | cut -d/ -f1-4)
          grep -Fx "$refresh_root" "$store_paths" >/dev/null
          test -L "${bundle}/artifacts/script/refresh"
          test "$(readlink -f "${bundle}/artifacts/script/refresh")" = "$refresh_path"
          grep -F "ExecReload=" "$unit_path" >/dev/null
          grep -Fx "data_dir: /var/lib/fishystuff/dolt/fishystuff" "$config_path" >/dev/null
          grep -Fx "cfg_dir: /var/lib/fishystuff/dolt/.doltcfg" "$config_path" >/dev/null
          grep -F "cp -R --no-preserve=ownership,mode" "$exe_path" >/dev/null
          if grep -F "cp -a" "$exe_path" >/dev/null; then
            echo "dolt start script must not preserve snapshot ownership or mode" >&2
            exit 1
          fi
        fi
        test -f "${bundle}/materialization.json"
        test -f "${bundle}/mode-substitute.txt"
        test -f "${bundle}/mode-realise.txt"
        test -f "${bundle}/mode-verify.txt"
        grep -F "ExecStart=" "$unit_path" >/dev/null
        grep -F "Restart=on-failure" "$unit_path" >/dev/null
        grep -F "WantedBy=multi-user.target" "$unit_path" >/dev/null
        ${lib.concatStringsSep "\n" (map (line: "grep -F ${lib.escapeShellArg line} \"$config_path\" >/dev/null") requiredConfigLines)}
        ${lib.concatStringsSep "\n" (
          map (
            fragment:
            ''
              if grep -F ${lib.escapeShellArg fragment} "$config_path" >/dev/null; then
                echo "unexpected config fragment present: ${fragment}" >&2
                exit 1
              fi
            ''
          ) forbiddenConfigFragments
        )}
        ${lib.concatStringsSep "\n" (map (line: "grep -Fx ${lib.escapeShellArg line} \"$unit_path\" >/dev/null") requiredUnitLines)}
        ${lib.concatStringsSep "\n" (map (expr: "jq -e ${lib.escapeShellArg expr} \"$bundle_json\" >/dev/null") requiredBundleJsonChecks)}
        ${lib.concatStringsSep "\n" (
          map (
            line:
            ''
              if grep -Fx ${lib.escapeShellArg line} "$unit_path" >/dev/null; then
                echo "unexpected unit line present: ${line}" >&2
                exit 1
              fi
            ''
          ) forbiddenUnitLines
        )}
        ${lib.concatStringsSep "\n" (
          lib.mapAttrsToList (
            name: value:
            ''
              jq -e '.supervision.environment.${name} == "${value}"' "$bundle_json" >/dev/null
              grep -Fx 'Environment="${name}=${value}"' "$unit_path" >/dev/null
            ''
          ) requiredEnvironment
        )}
        ${if requiredMaterializationHandle == null then
          ""
        else
          ''
            jq -e '.materialization.roots[] | select(.handle == "${requiredMaterializationHandle}")' "$bundle_json" >/dev/null
          ''}
        ${if requiredMaterializationAcquisition == null then
          ""
        else
          ''
            jq -e '.materialization.roots[] | select(.handle == "${requiredMaterializationHandle}" and .acquisition == "${requiredMaterializationAcquisition}")' "$bundle_json" >/dev/null
          ''}

        if jq -e '.runtimeOverlays[]? | select(.secret == true) | .targetPath | startswith("/nix/store/")' "$bundle_json" >/dev/null; then
          echo "secret overlay target unexpectedly points into the Nix store" >&2
          exit 1
        fi

        ${if runtimeEnvTarget == null then
          ''
            jq -e '.runtimeOverlays | length == ${toString expectedRuntimeOverlayCount}' "$bundle_json" >/dev/null

            if grep -F "EnvironmentFile=" "$unit_path" >/dev/null; then
              echo "unexpected environment file in unit" >&2
              exit 1
            fi
          ''
        else
          ''
            jq -e '.runtimeOverlays[] | select(.secret == true and .targetPath == "${runtimeEnvTarget}" and .onChange == "restart")' "$bundle_json" >/dev/null

            if grep -Fx "${runtimeEnvTarget}" "$store_paths" >/dev/null; then
              echo "secret overlay target leaked into the closure" >&2
              exit 1
            fi
          ''}

        ${if workingDirectory == null then
          ''
            jq -e '.supervision.workingDirectory == null' "$bundle_json" >/dev/null
          ''
        else
          ''
            jq -e '.supervision.workingDirectory == "${workingDirectory}"' "$bundle_json" >/dev/null
            grep -Fx "WorkingDirectory=${workingDirectory}" "$unit_path" >/dev/null
          ''}

        ${if requireSecretSpecPath then
          ''
            jq -e '.supervision.environment.FISHYSTUFF_SECRETSPEC_PATH | endswith("/etc/fishystuff/secretspec.toml")' "$bundle_json" >/dev/null
          ''
        else
          ""
        }

        touch "$out"
      '';
in
{
  api-service-bundle = mkBundleCheck {
    name = "api-service-bundle-check";
    bundle = apiServiceBundle;
    serviceId = "fishystuff-api";
    configDestination = "config.toml";
    runtimeEnvTarget = "/run/fishystuff/api/env";
    unitName = "fishystuff-api.service";
    minArgvLength = 3;
    requireSecretSpecPath = true;
    requiredEnvironment = {
      FISHYSTUFF_DEPLOYMENT_ENVIRONMENT = "beta";
      FISHYSTUFF_OTEL_DEPLOYMENT_ENVIRONMENT = "beta";
    };
    requiredMaterializationHandle = "pkg/main";
    requiredMaterializationAcquisition = "push";
  };

  api-service-bundle-production = mkBundleCheck {
    name = "api-service-bundle-production-check";
    bundle = apiServiceBundleProduction;
    serviceId = "fishystuff-api";
    configDestination = "config.toml";
    runtimeEnvTarget = "/run/fishystuff/api/env";
    unitName = "fishystuff-api.service";
    minArgvLength = 3;
    requireSecretSpecPath = true;
    requiredEnvironment = {
      FISHYSTUFF_DEPLOYMENT_ENVIRONMENT = "production";
      FISHYSTUFF_OTEL_DEPLOYMENT_ENVIRONMENT = "production";
    };
    requiredMaterializationHandle = "pkg/main";
    requiredMaterializationAcquisition = "push";
  };

  dolt-service-bundle = mkBundleCheck {
    name = "dolt-service-bundle-check";
    bundle = doltServiceBundle;
    serviceId = "fishystuff-dolt";
    configDestination = "sql-server.yaml";
    runtimeEnvTarget = "/run/fishystuff/api/env";
    unitName = "fishystuff-dolt.service";
    workingDirectory = "/var/lib/fishystuff/dolt";
    requiredEnvironment = {
      FISHYSTUFF_DEPLOYMENT_ENVIRONMENT = "beta";
      HOME = "/var/lib/fishystuff/dolt/home";
    };
    requiredUnitLines = [
      "User=fishystuff-dolt"
      "Group=fishystuff-dolt"
      "StateDirectory=fishystuff/dolt"
      "StateDirectoryMode=0750"
    ];
    requiredMaterializationHandle = "pkg/main";
    requiredMaterializationAcquisition = "substitute";
    forbiddenUnitLines = [
      "DynamicUser=true"
      "ReadWritePaths=/var/lib/fishystuff/dolt /var/lib/fishystuff/dolt/.doltcfg"
    ];
    expectedReloadMode = "command";
  };

  dolt-service-bundle-production = mkBundleCheck {
    name = "dolt-service-bundle-production-check";
    bundle = doltServiceBundleProduction;
    serviceId = "fishystuff-dolt";
    configDestination = "sql-server.yaml";
    runtimeEnvTarget = "/run/fishystuff/api/env";
    unitName = "fishystuff-dolt.service";
    workingDirectory = "/var/lib/fishystuff/dolt";
    requiredEnvironment = {
      FISHYSTUFF_DEPLOYMENT_ENVIRONMENT = "production";
      HOME = "/var/lib/fishystuff/dolt/home";
    };
    requiredUnitLines = [
      "User=fishystuff-dolt"
      "Group=fishystuff-dolt"
      "StateDirectory=fishystuff/dolt"
      "StateDirectoryMode=0750"
    ];
    requiredMaterializationHandle = "pkg/main";
    requiredMaterializationAcquisition = "substitute";
    forbiddenUnitLines = [
      "DynamicUser=true"
      "ReadWritePaths=/var/lib/fishystuff/dolt /var/lib/fishystuff/dolt/.doltcfg"
    ];
    expectedReloadMode = "command";
  };

  edge-service-bundle = mkBundleCheck {
    name = "edge-service-bundle-check";
    bundle = edgeServiceBundle;
    serviceId = "fishystuff-edge";
    configDestination = "Caddyfile";
    unitName = "fishystuff-edge.service";
    minArgvLength = 5;
    expectedReloadMode = "command";
    expectedRuntimeOverlayCount = 2;
    requiredMaterializationHandle = "pkg/main";
    requiredMaterializationAcquisition = "push";
    requiredConfigLines = [
      "https://beta.fishystuff.fish {"
      "https://api.beta.fishystuff.fish {"
      "https://cdn.beta.fishystuff.fish {"
      "https://telemetry.beta.fishystuff.fish {"
      "@runtime_manifest path /map/runtime-manifest.json"
      "/map/fishystuff_ui_bevy.*.js"
      "/map/fishystuff_ui_bevy.*.js.map"
      "/map/fishystuff_ui_bevy_bg.*.wasm"
      "/map/fishystuff_ui_bevy_bg.*.wasm.map"
      "br 1"
      "precompressed br gzip"
      "header Cache-Control \"no-store\""
      "header Cache-Control \"public, max-age=31536000, immutable\""
    ];
    requiredUnitLines = [
      "LoadCredential=fullchain.pem:/run/fishystuff/edge/tls/fullchain.pem"
      "LoadCredential=privkey.pem:/run/fishystuff/edge/tls/privkey.pem"
      "AmbientCapabilities=CAP_NET_BIND_SERVICE"
      "CapabilityBoundingSet=CAP_NET_BIND_SERVICE"
      "PrivateTmp=true"
      "ProtectSystem=strict"
      "NoNewPrivileges=true"
    ];
  };

  edge-service-bundle-production = mkBundleCheck {
    name = "edge-service-bundle-production-check";
    bundle = edgeServiceBundleProduction;
    serviceId = "fishystuff-edge";
    configDestination = "Caddyfile";
    unitName = "fishystuff-edge.service";
    minArgvLength = 5;
    expectedReloadMode = "command";
    expectedRuntimeOverlayCount = 2;
    requiredMaterializationHandle = "pkg/main";
    requiredMaterializationAcquisition = "push";
    requiredConfigLines = [
      "https://fishystuff.fish {"
      "https://api.fishystuff.fish {"
      "https://cdn.fishystuff.fish {"
      "https://telemetry.fishystuff.fish {"
      "@runtime_manifest path /map/runtime-manifest.json"
      "/map/fishystuff_ui_bevy.*.js"
      "/map/fishystuff_ui_bevy.*.js.map"
      "/map/fishystuff_ui_bevy_bg.*.wasm"
      "/map/fishystuff_ui_bevy_bg.*.wasm.map"
      "br 1"
      "precompressed br gzip"
      "header Cache-Control \"no-store\""
      "header Cache-Control \"public, max-age=31536000, immutable\""
    ];
    forbiddenConfigFragments = [
      "beta.fishystuff.fish"
    ];
    requiredUnitLines = [
      "LoadCredential=fullchain.pem:/run/fishystuff/edge/tls/fullchain.pem"
      "LoadCredential=privkey.pem:/run/fishystuff/edge/tls/privkey.pem"
      "AmbientCapabilities=CAP_NET_BIND_SERVICE"
      "CapabilityBoundingSet=CAP_NET_BIND_SERVICE"
      "PrivateTmp=true"
      "ProtectSystem=strict"
      "NoNewPrivileges=true"
    ];
  };

  edge-service-bundle-beta-gitops-handoff = mkBundleCheck {
    name = "edge-service-bundle-beta-gitops-handoff-check";
    bundle = edgeServiceBundleBetaGitopsHandoff;
    serviceId = "fishystuff-beta-edge";
    configDestination = "Caddyfile";
    unitName = "fishystuff-beta-edge.service";
    minArgvLength = 5;
    expectedReloadMode = "command";
    expectedRuntimeOverlayCount = 2;
    requiredMaterializationHandle = "pkg/main";
    requiredMaterializationAcquisition = "push";
    requiredConfigLines = [
      "https://beta.fishystuff.fish {"
      "https://api.beta.fishystuff.fish {"
      "https://cdn.beta.fishystuff.fish {"
      "https://telemetry.beta.fishystuff.fish {"
      "root * /var/lib/fishystuff/gitops-beta/served/beta/site"
      "root * /var/lib/fishystuff/gitops-beta/served/beta/cdn"
      "reverse_proxy 127.0.0.1:18192"
      "admin 127.0.0.1:2119"
      "@runtime_manifest path /map/runtime-manifest.json"
      "/map/fishystuff_ui_bevy.*.js"
      "/map/fishystuff_ui_bevy_bg.*.wasm"
      "header Cache-Control \"no-store\""
      "header Cache-Control \"public, max-age=31536000, immutable\""
    ];
    forbiddenConfigFragments = [
      "https://fishystuff.fish"
      "https://api.fishystuff.fish"
      "https://cdn.fishystuff.fish"
      "https://telemetry.fishystuff.fish"
      "/var/lib/fishystuff/gitops/served/production"
      "/srv/fishystuff"
    ];
    requiredUnitLines = [
      "Wants=network-online.target fishystuff-beta-api.service fishystuff-beta-vector.service"
      "LoadCredential=fullchain.pem:/run/fishystuff/beta-edge/tls/fullchain.pem"
      "LoadCredential=privkey.pem:/run/fishystuff/beta-edge/tls/privkey.pem"
      "AmbientCapabilities=CAP_NET_BIND_SERVICE"
      "CapabilityBoundingSet=CAP_NET_BIND_SERVICE"
      "PrivateTmp=true"
      "ProtectSystem=strict"
      "NoNewPrivileges=true"
    ];
    forbiddenUnitLines = [
      "Wants=network-online.target fishystuff-api.service fishystuff-vector.service"
      "LoadCredential=fullchain.pem:/run/fishystuff/edge/tls/fullchain.pem"
      "LoadCredential=privkey.pem:/run/fishystuff/edge/tls/privkey.pem"
    ];
    requiredBundleJsonChecks = [
      ''(.activation.directories | map(.path) | index("/var/lib/fishystuff/gitops-beta/served/beta/site") | not)''
      ''(.activation.directories | map(.path) | index("/var/lib/fishystuff/gitops-beta/served/beta/cdn") | not)''
      ''(.activation.requiredPaths | index("/var/lib/fishystuff/gitops-beta/served/beta/site")) != null''
      ''(.activation.requiredPaths | index("/var/lib/fishystuff/gitops-beta/served/beta/cdn")) != null''
      ''(.activation.directories | map(.path) | index("/run/fishystuff/beta-edge/tls")) != null''
      ''(.activation.directories | map(.path) | index("/run/fishystuff/edge/tls") | not)''
    ];
  };

  edge-service-bundle-production-gitops-handoff = mkBundleCheck {
    name = "edge-service-bundle-production-gitops-handoff-check";
    bundle = edgeServiceBundleProductionGitopsHandoff;
    serviceId = "fishystuff-edge";
    configDestination = "Caddyfile";
    unitName = "fishystuff-edge.service";
    minArgvLength = 5;
    expectedReloadMode = "command";
    expectedRuntimeOverlayCount = 2;
    requiredMaterializationHandle = "pkg/main";
    requiredMaterializationAcquisition = "push";
    requiredConfigLines = [
      "https://fishystuff.fish {"
      "https://api.fishystuff.fish {"
      "https://cdn.fishystuff.fish {"
      "https://telemetry.fishystuff.fish {"
      "root * /var/lib/fishystuff/gitops/served/production/site"
      "root * /var/lib/fishystuff/gitops/served/production/cdn"
      "reverse_proxy 127.0.0.1:18092"
      "@runtime_manifest path /map/runtime-manifest.json"
      "/map/fishystuff_ui_bevy.*.js"
      "/map/fishystuff_ui_bevy_bg.*.wasm"
      "header Cache-Control \"no-store\""
      "header Cache-Control \"public, max-age=31536000, immutable\""
    ];
    forbiddenConfigFragments = [
      "beta.fishystuff.fish"
      "/srv/fishystuff"
    ];
    requiredUnitLines = [
      "LoadCredential=fullchain.pem:/run/fishystuff/edge/tls/fullchain.pem"
      "LoadCredential=privkey.pem:/run/fishystuff/edge/tls/privkey.pem"
      "AmbientCapabilities=CAP_NET_BIND_SERVICE"
      "CapabilityBoundingSet=CAP_NET_BIND_SERVICE"
      "PrivateTmp=true"
      "ProtectSystem=strict"
      "NoNewPrivileges=true"
    ];
    requiredBundleJsonChecks = [
      ''(.activation.directories | map(.path) | index("/var/lib/fishystuff/gitops/served/production/site") | not)''
      ''(.activation.directories | map(.path) | index("/var/lib/fishystuff/gitops/served/production/cdn") | not)''
      ''(.activation.requiredPaths | index("/var/lib/fishystuff/gitops/served/production/site")) != null''
      ''(.activation.requiredPaths | index("/var/lib/fishystuff/gitops/served/production/cdn")) != null''
    ];
  };

  vector-agent-service-bundle = mkBundleCheck {
    name = "vector-agent-service-bundle-check";
    bundle = vectorAgentServiceBundle;
    serviceId = "fishystuff-vector";
    configDestination = "vector.yaml";
    unitName = "fishystuff-vector.service";
    minArgvLength = 3;
    workingDirectory = "/var/lib/fishystuff/vector";
    expectedRuntimeOverlayCount = 0;
    requiredMaterializationHandle = "pkg/main";
    requiredMaterializationAcquisition = "substitute";
    requiredConfigLines = [
      "env = \"beta\""
      ".deployment_environment = \"beta\""
      "agent_forward_to_telemetry_vector:"
      "address: \"10.0.0.4:6000\""
    ];
    requiredUnitLines = [
      "StateDirectory=fishystuff/vector"
      "StateDirectoryMode=0750"
      "SupplementaryGroups=systemd-journal"
      "PrivateTmp=true"
      "ProtectSystem=strict"
      "NoNewPrivileges=true"
    ];
  };

  vector-agent-service-bundle-production = mkBundleCheck {
    name = "vector-agent-service-bundle-production-check";
    bundle = vectorAgentServiceBundleProduction;
    serviceId = "fishystuff-vector";
    configDestination = "vector.yaml";
    unitName = "fishystuff-vector.service";
    minArgvLength = 3;
    workingDirectory = "/var/lib/fishystuff/vector";
    expectedRuntimeOverlayCount = 0;
    requiredMaterializationHandle = "pkg/main";
    requiredMaterializationAcquisition = "substitute";
    requiredConfigLines = [
      "env = \"production\""
      ".deployment_environment = \"production\""
      "agent_forward_to_telemetry_vector:"
      "address: \"10.0.0.4:6000\""
    ];
    forbiddenConfigFragments = [
      "env = \"beta\""
      ".deployment_environment = \"beta\""
    ];
    requiredUnitLines = [
      "StateDirectory=fishystuff/vector"
      "StateDirectoryMode=0750"
      "SupplementaryGroups=systemd-journal"
      "PrivateTmp=true"
      "ProtectSystem=strict"
      "NoNewPrivileges=true"
    ];
  };
}
