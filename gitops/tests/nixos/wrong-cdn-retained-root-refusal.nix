{
  pkgs,
  mgmtPackage,
  gitopsSrc,
}:
let
  apiArtifact = pkgs.writeText "fishystuff-gitops-wrong-cdn-retained-root-api" "wrong cdn retained root api\n";
  doltServiceArtifact = pkgs.writeText "fishystuff-gitops-wrong-cdn-retained-root-dolt-service" "wrong cdn retained root dolt service\n";
  previousApiArtifact = pkgs.writeText "fishystuff-gitops-wrong-cdn-retained-root-previous-api" "previous wrong cdn retained root api\n";
  previousDoltServiceArtifact = pkgs.writeText "fishystuff-gitops-wrong-cdn-retained-root-previous-dolt-service" "previous wrong cdn retained root dolt service\n";
  siteArtifact = pkgs.runCommand "fishystuff-gitops-wrong-cdn-retained-root-site" { } ''
    mkdir -p "$out"
    printf 'wrong cdn retained root refusal site\n' > "$out/index.html"
  '';
  previousSiteArtifact = pkgs.runCommand "fishystuff-gitops-wrong-cdn-retained-root-previous-site" { } ''
    mkdir -p "$out"
    printf 'previous wrong cdn retained root refusal site\n' > "$out/index.html"
  '';
  currentCdnRoot = pkgs.runCommand "fishystuff-gitops-wrong-cdn-retained-root-current" { } ''
    mkdir -p "$out/map"
    printf '{"module":"fishystuff_ui_bevy.current.js","wasm":"fishystuff_ui_bevy_bg.current.wasm"}\n' > "$out/map/runtime-manifest.json"
    printf 'current module\n' > "$out/map/fishystuff_ui_bevy.current.js"
    printf 'current wasm\n' > "$out/map/fishystuff_ui_bevy_bg.current.wasm"
  '';
  previousCdnRoot = pkgs.runCommand "fishystuff-gitops-wrong-cdn-retained-root-previous" { } ''
    mkdir -p "$out/map"
    printf '{"module":"fishystuff_ui_bevy.previous.js","wasm":"fishystuff_ui_bevy_bg.previous.wasm"}\n' > "$out/map/runtime-manifest.json"
    printf 'previous module\n' > "$out/map/fishystuff_ui_bevy.previous.js"
    printf 'previous wasm\n' > "$out/map/fishystuff_ui_bevy_bg.previous.wasm"
  '';
  unrelatedCdnRoot = pkgs.runCommand "fishystuff-gitops-wrong-cdn-retained-root-unrelated" { } ''
    mkdir -p "$out/map"
    printf '{"module":"fishystuff_ui_bevy.unrelated.js","wasm":"fishystuff_ui_bevy_bg.unrelated.wasm"}\n' > "$out/map/runtime-manifest.json"
    printf 'unrelated module\n' > "$out/map/fishystuff_ui_bevy.unrelated.js"
    printf 'unrelated wasm\n' > "$out/map/fishystuff_ui_bevy_bg.unrelated.wasm"
  '';
  activeCdnServingRoot = pkgs.callPackage ../../../nix/packages/cdn-serving-root.nix {
    currentRoot = currentCdnRoot;
    previousRoots = [ unrelatedCdnRoot ];
  };
  previousCdnServingRoot = pkgs.callPackage ../../../nix/packages/cdn-serving-root.nix {
    currentRoot = previousCdnRoot;
  };
  desiredState = pkgs.writeText "vm-wrong-cdn-retained-root-refusal.desired.json" (builtins.toJSON {
    cluster = "local-test";
    generation = 16;
    mode = "vm-test";
    hosts.vm-single-host = {
      enabled = true;
      role = "single-site";
      hostname = "vm-single-host";
    };
    releases.wrong-cdn-retained-root-release = {
      generation = 16;
      git_rev = "wrong-cdn-retained-root-refusal";
      dolt_commit = "wrong-cdn-retained-root-refusal";
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
        commit = "wrong-cdn-retained-root-refusal";
        branch_context = "local-test";
        mode = "read_only";
      };
    };
    releases.previous-release = {
      generation = 15;
      git_rev = "previous-wrong-cdn-retained-root-refusal";
      dolt_commit = "previous-wrong-cdn-retained-root-refusal";
      closures = {
        api = {
          enabled = false;
          store_path = "${previousApiArtifact}";
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
        commit = "previous-wrong-cdn-retained-root-refusal";
        branch_context = "local-test";
        mode = "read_only";
      };
    };
    environments.local-test = {
      enabled = true;
      strategy = "single_active";
      host = "vm-single-host";
      active_release = "wrong-cdn-retained-root-release";
      retained_releases = [ "previous-release" ];
      serve = true;
    };
  });
in
pkgs.testers.runNixOSTest {
  name = "fishystuff-gitops-wrong-cdn-retained-root-refusal";

  nodes.machine =
    { ... }:
    {
      system.stateVersion = "25.11";
      networking.hostName = "vm-single-host";
      virtualisation.additionalPaths = [
        apiArtifact
        doltServiceArtifact
        previousApiArtifact
        previousDoltServiceArtifact
        siteArtifact
        previousSiteArtifact
        currentCdnRoot
        unrelatedCdnRoot
        activeCdnServingRoot
        previousCdnRoot
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
    machine.succeed("jq -e '.mode == \"vm-test\" and .environments.\"local-test\".serve == true and .environments.\"local-test\".retained_releases == [\"previous-release\"]' ${desiredState}")
    machine.succeed("jq -e '.retained_root_count == 1 and .retained_roots == [\"${unrelatedCdnRoot}\"]' ${activeCdnServingRoot}/cdn-serving-manifest.json")
    machine.succeed("jq -e '.current_root == \"${previousCdnRoot}\"' ${previousCdnServingRoot}/cdn-serving-manifest.json")
    machine.fail("timeout 15s env FISHYSTUFF_GITOPS_STATE_FILE=${desiredState} ${mgmtPackage}/bin/mgmt run --hostname vm-single-host --tmp-prefix --no-pgp --client-urls=http://127.0.0.1:2379 --server-urls=http://127.0.0.1:2380 --advertise-client-urls=http://127.0.0.1:2379 --advertise-server-urls=http://127.0.0.1:2380 --converged-timeout=-1 lang ${gitopsSrc}/main.mcl >/tmp/fishystuff-gitops-wrong-cdn-retained-root-refusal.log 2>&1")
    machine.succeed("test ! -e /var/lib/fishystuff/gitops-test/status/local-test.json")
    machine.succeed("test ! -e /var/lib/fishystuff/gitops-test/instances/local-test-wrong-cdn-retained-root-release.json")
    machine.succeed("test ! -e /var/lib/fishystuff/gitops-test/active/local-test.json")
    machine.succeed("test ! -e /srv/fishystuff")
    machine.succeed("test ! -e /var/lib/fishystuff/mgmt")
  '';
}
