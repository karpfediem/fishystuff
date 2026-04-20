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
  explicitMaterializationRoots = service.bundle.materialization.roots;
  materializationRoots =
    if explicitMaterializationRoots != [ ] then
      explicitMaterializationRoots
    else
      lib.imap1 (
        idx: path:
        helpers.mkMaterializationRoot {
          handle = "root/${toString idx}";
          inherit path;
        }
      ) explicitStoreRoots;
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

  mkArtifact =
    name: artifact:
    artifact
    // {
      storePath = toString artifact.storePath;
      store_path = toString artifact.storePath;
      bundlePath = "artifacts/${name}";
      bundle_path = "artifacts/${name}";
    };

  mkActivation =
    activation:
    activation
    // {
      writable_paths = activation.writablePaths;
    };

  mkIdentity =
    identity:
    identity
    // {
      dynamic_user = identity.dynamicUser;
      supplementary_groups = identity.supplementaryGroups;
    };

  mkRestart =
    restart:
    restart
    // {
      delay_seconds = restart.delaySeconds;
    };

  mkReload =
    reload:
    reload
    // {
      argv = map helpers.stringify reload.argv;
    };

  mkStop =
    stop:
    stop
    // {
      argv = map helpers.stringify stop.argv;
      timeout_seconds = stop.timeoutSeconds;
    };

  mkSupervision =
    supervision:
    supervision
    // {
      argv = map helpers.stringify supervision.argv;
      environment_files = supervision.environmentFiles;
      working_directory = supervision.workingDirectory;
      identity = mkIdentity supervision.identity;
      restart = mkRestart supervision.restart;
      reload = mkReload supervision.reload;
      stop = mkStop supervision.stop;
    };

  mkRuntimeOverlay =
    overlay:
    overlay
    // {
      target_path = overlay.targetPath;
      merge_mode = overlay.mergeMode;
      on_change = overlay.onChange;
    };

  mkMaterializationRoot =
    root:
    root
    // {
      path = toString root.path;
      allow_build = root.allowBuild;
    };

  contract = {
    contractVersion = 1;
    contract_version = 1;
    id =
      if service.bundle.id != "" then
        service.bundle.id
      else
        name;
    roots = {
      store = explicitStoreRootStrings;
    };
    artifacts = lib.mapAttrs mkArtifact service.bundle.artifacts;
    activation = mkActivation service.bundle.activation;
    supervision = mkSupervision (
      service.bundle.supervision
      // {
        argv = service.process.argv;
      }
    );
    runtimeOverlays = service.bundle.runtimeOverlays;
    runtime_overlays = map mkRuntimeOverlay service.bundle.runtimeOverlays;
    requiredCapabilities = service.bundle.requiredCapabilities;
    required_capabilities = service.bundle.requiredCapabilities;
    backends = service.bundle.backends;
    materialization = {
      schemaVersion = 1;
      schema_version = 1;
      roots = map mkMaterializationRoot materializationRoots;
    };
    bundleFiles = {
      bundleJson = "bundle.json";
      materializationJson = "materialization.json";
      registration = "registration";
      modeSubstitute = "mode-substitute.txt";
      modeRealise = "mode-realise.txt";
      storePaths = "store-paths";
      modeVerify = "mode-verify.txt";
    };
    bundle_files = {
      bundle_json = "bundle.json";
      materialization_json = "materialization.json";
      registration = "registration";
      mode_substitute = "mode-substitute.txt";
      mode_realise = "mode-realise.txt";
      store_paths = "store-paths";
      mode_verify = "mode-verify.txt";
    };
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

    jq -r '.artifacts | to_entries[] | [.value.bundle_path, .value.store_path] | @tsv' \
      "$contractPath" | while IFS=$'\t' read -r rel store; do
        mkdir -p "$out/$(dirname "$rel")"
        ln -sfnT "$store" "$out/$rel"
      done

    cp ${closureInfo}/registration "$out/registration"
    cp ${closureInfo}/store-paths "$out/store-paths"
    jq '.materialization' "$contractPath" > "$out/materialization.json"
    jq -r '.materialization.roots[] | select(.acquisition == "substitute") | .path' "$contractPath" | sort -u > "$out/mode-substitute.txt"
    jq -r '.materialization.roots[] | select(.acquisition == "substitute-or-build") | .path' "$contractPath" | sort -u > "$out/mode-realise.txt"
    jq -r '.materialization.roots[] | select(.acquisition == "push") | .path' "$contractPath" | sort -u > "$out/mode-verify.txt"
    jq \
      --arg root "$out" \
      --rawfile storePaths ${closureInfo}/store-paths \
      '. + {
        storePaths: ($storePaths | split("\n") | map(select(length > 0))),
        closure: {
          root: $root,
          materialization_file: "materialization.json",
          mode_substitute_file: "mode-substitute.txt",
          mode_realise_file: "mode-realise.txt",
          registration_file: "registration",
          store_paths_file: "store-paths",
          store_paths: ($storePaths | split("\n") | map(select(length > 0))),
          mode_verify_file: "mode-verify.txt"
        }
      }' \
      "$contractPath" > "$out/bundle.json"
  ''
