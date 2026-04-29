{
  pkgs,
  lib ? pkgs.lib,
  apiServiceBundle,
  doltServiceBundle,
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
      requiredUnitLines ? [ ],
      forbiddenUnitLines ? [ ],
      expectedReloadMode ? "restart",
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
        ${lib.concatStringsSep "\n" (map (line: "grep -Fx ${lib.escapeShellArg line} \"$unit_path\" >/dev/null") requiredUnitLines)}
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
            jq -e '.runtimeOverlays | length == 0' "$bundle_json" >/dev/null

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
}
