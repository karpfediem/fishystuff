{ writeShellApplication, api, coreutils, dolt }:

writeShellApplication {
  name = "fishystuff-api-entrypoint";
  runtimeInputs = [
    api
    coreutils
    dolt
  ];
  text = builtins.readFile ../../api/entrypoint.sh;
}
