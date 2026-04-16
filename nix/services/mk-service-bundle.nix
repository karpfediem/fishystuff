{ lib, pkgs, evalService }:
let
  helpers = import ./helpers.nix { inherit lib; };
in
{
  name,
  serviceModule,
  configuration ? { },
  extraModules ? [ ],
}:
let
  service = evalService {
    inherit
      configuration
      extraModules
      name
      serviceModule
      ;
  };

  explicitStoreRoots = service.bundle.roots.store;
  explicitStoreRootStrings = map toString explicitStoreRoots;
  syntheticRoot =
    if explicitStoreRoots == [ ] then
      throw "Service ${name} did not declare any bundle.roots.store entries."
    else
      pkgs.linkFarm "${name}-bundle-roots" (
        lib.imap1 (
          idx: path: {
            name = "root-${toString idx}";
            inherit path;
          }
        ) explicitStoreRoots
      );
  closureInfo = pkgs.closureInfo {
    rootPaths = [ syntheticRoot ];
  };

  contract = {
    contractVersion = 1;
    id =
      if service.bundle.id != "" then
        service.bundle.id
      else
        name;
    roots = {
      store = explicitStoreRootStrings;
    };
    artifacts = lib.mapAttrs (
      _: artifact:
      artifact
      // {
        storePath = toString artifact.storePath;
      }
    ) service.bundle.artifacts;
    activation = service.bundle.activation;
    supervision =
      service.bundle.supervision
      // {
        argv = map helpers.stringify service.process.argv;
      };
    runtimeOverlays = service.bundle.runtimeOverlays;
    requiredCapabilities = service.bundle.requiredCapabilities;
    backends = service.bundle.backends;
  };
in
pkgs.runCommand "${name}-service-bundle"
  {
    nativeBuildInputs = [ pkgs.jq ];
    passAsFile = [ "contract" ];
    contract = builtins.toJSON contract;
  }
  ''
    mkdir -p "$out"
    cp ${closureInfo}/registration "$out/registration"
    cp ${closureInfo}/store-paths "$out/store-paths"
    jq --rawfile storePaths ${closureInfo}/store-paths \
      '. + { storePaths: ($storePaths | split("\n") | map(select(length > 0))) }' \
      "$contractPath" > "$out/bundle.json"
  ''
