{ lib, pkgs }:
let
  portableLib = import "${pkgs.path}/nixos/modules/system/service/portable/lib.nix" { inherit lib; };
  configuredService = portableLib.configure {
    serviceManagerPkgs = pkgs;
  };

  rootModule = {
    options.services = lib.mkOption {
      type = lib.types.attrsOf configuredService.serviceSubmodule;
      default = { };
    };
  };

  checkAssertions =
    name: service:
    let
      failures = builtins.filter (assertion: !assertion.assertion) (
        portableLib.getAssertions [ "services" name ] service
      );
      warnings = portableLib.getWarnings [ "services" name ] service;
      failureMessage = lib.concatStringsSep "\n" (map (assertion: assertion.message) failures);
      warnMessage = lib.concatStringsSep "\n" warnings;
    in
    if failures != [ ] then
      throw failureMessage
    else
      lib.warnIf (warnings != [ ]) warnMessage service;
in
{
  name,
  serviceModule,
  configuration ? { },
  extraModules ? [ ],
}:
let
  evaluation = lib.evalModules {
    modules = [
      rootModule
      {
        services.${name}.imports = [ serviceModule ] ++ extraModules;
      }
      {
        services.${name} = configuration;
      }
      {
        services.${name}.bundle.id = lib.mkDefault name;
      }
    ];
  };
in
checkAssertions name evaluation.config.services.${name}
