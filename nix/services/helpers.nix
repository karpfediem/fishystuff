{ lib }:
let
  pathLikeType = lib.types.oneOf [ lib.types.path lib.types.str ];
  storePathType = lib.types.oneOf [ lib.types.package lib.types.path ];
  artifactPathType = lib.types.oneOf [ lib.types.package lib.types.path lib.types.str ];
  envValueType = lib.types.oneOf [ lib.types.bool lib.types.int lib.types.path lib.types.str ];

  stringify = value:
    if builtins.isBool value then
      lib.boolToString value
    else
      toString value;
in
{
  inherit
    artifactPathType
    envValueType
    pathLikeType
    storePathType
    stringify
    ;

  stringifyEnvironment = env:
    lib.mapAttrs (_: value: stringify value) env;

  mkActivationDirectory =
    attrs:
    {
      create = true;
      mode = "0755";
      owner = null;
      group = null;
    }
    // attrs;

  mkArtifact =
    attrs:
    {
      destination = null;
      executable = false;
    }
    // attrs;

  mkMaterializationRoot =
    attrs:
    {
      class = "workspace-local";
      drv = null;
      acquisition = "push";
      allowBuild = false;
      required = true;
    }
    // attrs;

  mkRuntimeOverlay =
    attrs:
    {
      format = "env";
      mergeMode = "replace";
      required = false;
      secret = false;
      keys = [ ];
      onChange = "restart";
    }
    // attrs;
}
