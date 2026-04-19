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
      runtimeEnvTarget,
      unitName,
      requireSecretSpecPath ? false,
      workingDirectory ? null,
      requiredEnvironment ? { },
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
        jq -e '.artifacts["systemd/unit"].kind == "systemd-unit"' "$bundle_json" >/dev/null
        jq -e '.artifacts["systemd/unit"].destination == "${unitName}"' "$bundle_json" >/dev/null
        jq -e '.artifacts["systemd/unit"].bundle_path == "artifacts/systemd/unit"' "$bundle_json" >/dev/null
        jq -e '.bundle_files.bundle_json == "bundle.json"' "$bundle_json" >/dev/null
        jq -e '.bundle_files.registration == "registration"' "$bundle_json" >/dev/null
        jq -e '.bundle_files.store_paths == "store-paths"' "$bundle_json" >/dev/null
        jq -e '.closure.registration_file == "registration"' "$bundle_json" >/dev/null
        jq -e '.closure.store_paths_file == "store-paths"' "$bundle_json" >/dev/null
        jq -e '.supervision.argv | length >= 3' "$bundle_json" >/dev/null
        jq -e '.supervision.restart.policy == "on-failure"' "$bundle_json" >/dev/null
        jq -e '.supervision.reload.mode == "restart"' "$bundle_json" >/dev/null
        jq -e '.runtimeOverlays[] | select(.secret == true and .targetPath == "${runtimeEnvTarget}" and .onChange == "restart")' "$bundle_json" >/dev/null
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
        grep -F "ExecStart=" "$unit_path" >/dev/null
        grep -F "Restart=on-failure" "$unit_path" >/dev/null
        grep -F "WantedBy=multi-user.target" "$unit_path" >/dev/null
        ${lib.concatStringsSep "\n" (
          lib.mapAttrsToList (
            name: value:
            ''
              jq -e '.supervision.environment.${name} == "${value}"' "$bundle_json" >/dev/null
              grep -Fx 'Environment="${name}=${value}"' "$unit_path" >/dev/null
            ''
          ) requiredEnvironment
        )}

        if jq -e '.runtimeOverlays[] | select(.secret == true) | .targetPath | startswith("/nix/store/")' "$bundle_json" >/dev/null; then
          echo "secret overlay target unexpectedly points into the Nix store" >&2
          exit 1
        fi

        if grep -Fx "${runtimeEnvTarget}" "$store_paths" >/dev/null; then
          echo "secret overlay target leaked into the closure" >&2
          exit 1
        fi

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
    requireSecretSpecPath = true;
  };

  dolt-service-bundle = mkBundleCheck {
    name = "dolt-service-bundle-check";
    bundle = doltServiceBundle;
    serviceId = "fishystuff-dolt";
    configDestination = "sql-server.yaml";
    runtimeEnvTarget = "/run/fishystuff/dolt/env";
    unitName = "fishystuff-dolt.service";
    workingDirectory = "/var/lib/fishystuff/dolt";
    requiredEnvironment = {
      HOME = "/var/lib/fishystuff/dolt";
    };
  };
}
