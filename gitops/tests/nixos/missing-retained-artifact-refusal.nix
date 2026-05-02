{
  pkgs,
  mgmtPackage,
  gitopsSrc,
}:
let
  apiArtifact = pkgs.writeText "fishystuff-gitops-missing-retained-artifact-api" "candidate api\n";
  doltServiceArtifact = pkgs.writeText "fishystuff-gitops-missing-retained-artifact-dolt-service" "candidate dolt service\n";
  previousDoltServiceArtifact = pkgs.writeText "fishystuff-gitops-missing-retained-artifact-previous-dolt-service" "previous dolt service\n";
  siteArtifact = pkgs.runCommand "fishystuff-gitops-missing-retained-artifact-site" { } ''
    mkdir -p "$out"
    printf 'missing retained artifact site\n' > "$out/index.html"
  '';
  previousSiteArtifact = pkgs.runCommand "fishystuff-gitops-missing-retained-artifact-previous-site" { } ''
    mkdir -p "$out"
    printf 'previous missing retained artifact site\n' > "$out/index.html"
  '';
  currentCdnRoot = pkgs.runCommand "fishystuff-gitops-missing-retained-artifact-current-cdn" { } ''
    mkdir -p "$out/map"
    printf '{"module":"fishystuff_ui_bevy.current.js","wasm":"fishystuff_ui_bevy_bg.current.wasm"}\n' > "$out/map/runtime-manifest.json"
    printf 'current module\n' > "$out/map/fishystuff_ui_bevy.current.js"
    printf 'current wasm\n' > "$out/map/fishystuff_ui_bevy_bg.current.wasm"
  '';
  previousCdnRoot = pkgs.runCommand "fishystuff-gitops-missing-retained-artifact-previous-cdn" { } ''
    mkdir -p "$out/map"
    printf '{"module":"fishystuff_ui_bevy.previous.js","wasm":"fishystuff_ui_bevy_bg.previous.wasm"}\n' > "$out/map/runtime-manifest.json"
    printf 'previous module\n' > "$out/map/fishystuff_ui_bevy.previous.js"
    printf 'previous wasm\n' > "$out/map/fishystuff_ui_bevy_bg.previous.wasm"
  '';
  activeCdnServingRoot = pkgs.callPackage ../../../nix/packages/cdn-serving-root.nix {
    currentRoot = currentCdnRoot;
    previousRoots = [ previousCdnRoot ];
  };
  previousCdnServingRoot = pkgs.callPackage ../../../nix/packages/cdn-serving-root.nix {
    currentRoot = previousCdnRoot;
  };
  desiredState = pkgs.writeText "vm-missing-retained-artifact-refusal.desired.json" (builtins.toJSON {
    cluster = "local-test";
    generation = 18;
    mode = "vm-test";
    hosts.vm-single-host = {
      enabled = true;
      role = "single-site";
      hostname = "vm-single-host";
    };
    releases.candidate-release = {
      generation = 18;
      git_rev = "missing-retained-artifact";
      dolt_commit = "missing-retained-artifact";
      closures = {
        api = {
          enabled = false;
          store_path = "${apiArtifact}";
          gcroot_path = "";
        };
        site = {
          enabled = false;
          store_path = "${siteArtifact}";
          gcroot_path = "";
        };
        cdn_runtime = {
          enabled = false;
          store_path = "${activeCdnServingRoot}";
          gcroot_path = "";
        };
        dolt_service = {
          enabled = false;
          store_path = "${doltServiceArtifact}";
          gcroot_path = "";
        };
      };
      dolt = {
        repository = "fishystuff/fishystuff";
        commit = "missing-retained-artifact";
        branch_context = "local-test";
        mode = "read_only";
      };
    };
    releases.previous-release = {
      generation = 17;
      git_rev = "previous-missing-retained-artifact";
      dolt_commit = "previous-missing-retained-artifact";
      closures = {
        api = {
          enabled = false;
          store_path = "";
          gcroot_path = "";
        };
        site = {
          enabled = false;
          store_path = "${previousSiteArtifact}";
          gcroot_path = "";
        };
        cdn_runtime = {
          enabled = false;
          store_path = "${previousCdnServingRoot}";
          gcroot_path = "";
        };
        dolt_service = {
          enabled = false;
          store_path = "${previousDoltServiceArtifact}";
          gcroot_path = "";
        };
      };
      dolt = {
        repository = "fishystuff/fishystuff";
        commit = "previous-missing-retained-artifact";
        branch_context = "local-test";
        mode = "read_only";
      };
    };
    environments.local-test = {
      enabled = true;
      strategy = "single_active";
      host = "vm-single-host";
      active_release = "candidate-release";
      retained_releases = [ "previous-release" ];
      serve = true;
    };
  });
in
pkgs.testers.runNixOSTest {
  name = "fishystuff-gitops-missing-retained-artifact-refusal";

  nodes.machine =
    { ... }:
    {
      system.stateVersion = "25.11";
      networking.hostName = "vm-single-host";
      virtualisation.additionalPaths = [
        apiArtifact
        doltServiceArtifact
        previousDoltServiceArtifact
        siteArtifact
        previousSiteArtifact
        currentCdnRoot
        previousCdnRoot
        activeCdnServingRoot
        previousCdnServingRoot
        desiredState
      ];
      environment.systemPackages = [
        mgmtPackage
        pkgs.jq
      ];
    };

  testScript = ''
    start_all()

    machine.succeed("test -x ${mgmtPackage}/bin/mgmt")
    machine.succeed("jq -e '.mode == \"vm-test\" and .environments.\"local-test\".serve == true and .releases.\"previous-release\".closures.api.store_path == \"\"' ${desiredState}")
    machine.fail("timeout 15s env FISHYSTUFF_GITOPS_STATE_FILE=${desiredState} ${mgmtPackage}/bin/mgmt run --hostname vm-single-host --tmp-prefix --no-pgp --client-urls=http://127.0.0.1:2379 --server-urls=http://127.0.0.1:2380 --advertise-client-urls=http://127.0.0.1:2379 --advertise-server-urls=http://127.0.0.1:2380 --converged-timeout=-1 lang ${gitopsSrc}/main.mcl >/tmp/fishystuff-gitops-missing-retained-artifact-refusal.log 2>&1")
    machine.succeed("grep -F 'serving retained release previous-release requires api store_path' /tmp/fishystuff-gitops-missing-retained-artifact-refusal.log")
    machine.succeed("test ! -e /var/lib/fishystuff/gitops-test/status/local-test.json")
    machine.succeed("test ! -e /var/lib/fishystuff/gitops-test/instances/local-test-candidate-release.json")
    machine.succeed("test ! -e /var/lib/fishystuff/gitops-test/active/local-test.json")
    machine.succeed("test ! -e /run/fishystuff/gitops-test/routes/local-test.json")
    machine.succeed("test ! -e /srv/fishystuff")
    machine.succeed("test ! -e /var/lib/fishystuff/mgmt")
  '';
}
