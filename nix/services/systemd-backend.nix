{ lib, pkgs }:
let
  helpers = import ./helpers.nix { inherit lib; };

  escapeForDoubleQuotes =
    value:
    lib.replaceStrings [ "\\" "\"" "\n" ] [ "\\\\" "\\\"" "\\n" ] (toString value);

  quote = value: "\"${escapeForDoubleQuotes value}\"";

  renderExecArg =
    value:
    let
      str = toString value;
      needsQuotes =
        lib.any (needle: lib.hasInfix needle str) [
          " "
          "\t"
          "\""
          "\\"
        ];
    in
    if needsQuotes then quote str else str;

  renderListLine =
    key: values:
    if values == [ ] then
      [ ]
    else
      [ "${key}=${lib.concatStringsSep " " values}" ];

  renderOptionalLine =
    key: value:
    if value == null || value == "" then
      [ ]
    else
      [ "${key}=${value}" ];

  renderEnvironmentLines =
    environment:
    lib.mapAttrsToList (name: value: "Environment=${quote "${name}=${toString value}"}") environment;
in
{
  mkSystemdUnit =
    {
      unitName,
      description,
      argv,
      environment ? { },
      environmentFiles ? [ ],
      user ? null,
      group ? null,
      dynamicUser ? false,
      supplementaryGroups ? [ ],
      workingDirectory ? null,
      after ? [ ],
      wants ? [ ],
      wantedBy ? [ "multi-user.target" ],
      restartPolicy ? "on-failure",
      restartDelaySeconds ? 5,
      execReloadArgv ? [ ],
      readWritePaths ? [ ],
      unitLines ? [ ],
      serviceLines ? [ ],
    }:
    let
      unitText = lib.concatLines (
        [
          "[Unit]"
          "Description=${description}"
        ]
        ++ unitLines
        ++ renderListLine "After" after
        ++ renderListLine "Wants" wants
        ++ [
          "[Service]"
          "Type=simple"
        ]
        ++ renderOptionalLine "User" user
        ++ renderOptionalLine "Group" group
        ++ lib.optional dynamicUser "DynamicUser=true"
        ++ renderListLine "SupplementaryGroups" supplementaryGroups
        ++ renderOptionalLine "WorkingDirectory" workingDirectory
        ++ renderEnvironmentLines environment
        ++ map (path: "EnvironmentFile=${path}") environmentFiles
        ++ [
          "ExecStart=${lib.concatStringsSep " " (map renderExecArg argv)}"
        ]
        ++ lib.optional (execReloadArgv != [ ])
          "ExecReload=${lib.concatStringsSep " " (map renderExecArg execReloadArgv)}"
        ++ [
          "Restart=${restartPolicy}"
          "RestartSec=${toString restartDelaySeconds}s"
        ]
        ++ renderListLine "ReadWritePaths" readWritePaths
        ++ serviceLines
        ++ [
          "[Install]"
          "WantedBy=${lib.concatStringsSep " " wantedBy}"
        ]
      );
      unitFile = pkgs.writeText unitName unitText;
    in
    {
      file = unitFile;
      artifact = helpers.mkArtifact {
        kind = "systemd-unit";
        storePath = unitFile;
        destination = unitName;
      };
      backend = {
        service_manager = "systemd";
        daemon_reload = true;
        units = [
          {
            name = unitName;
            artifact = "systemd/unit";
            install_path = "/etc/systemd/system/${unitName}";
            startup = "enabled";
            state = "running";
          }
        ];
      };
    };
}
