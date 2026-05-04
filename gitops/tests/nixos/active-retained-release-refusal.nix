{
  pkgs,
  mgmtPackage,
  gitopsSrc,
}:
let
  apiArtifact = pkgs.writeText "fishystuff-gitops-active-retained-refusal-api" "api\n";
  doltServiceArtifact = pkgs.writeText "fishystuff-gitops-active-retained-refusal-dolt-service" "dolt service\n";
  siteArtifact = pkgs.runCommand "fishystuff-gitops-active-retained-refusal-site" { } ''
    mkdir -p "$out"
    printf 'active retained release refusal site\n' > "$out/index.html"
  '';
  currentCdnRoot = pkgs.runCommand "fishystuff-gitops-active-retained-refusal-cdn-current" { } ''
    mkdir -p "$out/map"
    printf '{"module":"fishystuff_ui_bevy.active-retained.js","wasm":"fishystuff_ui_bevy_bg.active-retained.wasm"}\n' > "$out/map/runtime-manifest.json"
    printf 'active retained release refusal module\n' > "$out/map/fishystuff_ui_bevy.active-retained.js"
    printf 'active retained release refusal wasm\n' > "$out/map/fishystuff_ui_bevy_bg.active-retained.wasm"
  '';
  cdnRuntimeArtifact = pkgs.callPackage ../../../nix/packages/cdn-serving-root.nix {
    currentRoot = currentCdnRoot;
    previousRoots = [ currentCdnRoot ];
  };
  desiredState = pkgs.writeText "active-retained-release-refusal.desired.json" (builtins.toJSON {
    cluster = "local-test";
    generation = 51;
    mode = "vm-test";
    hosts.vm-single-host = {
      enabled = true;
      role = "single-site";
      hostname = "vm-single-host";
    };
    releases.candidate-release = {
      generation = 51;
      git_rev = "active-retained-release-refusal";
      dolt_commit = "active-retained-release-refusal";
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
          store_path = "${cdnRuntimeArtifact}";
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
        commit = "active-retained-release-refusal";
        branch_context = "local-test";
        mode = "read_only";
        materialization = "metadata_only";
        remote_url = "";
        cache_dir = "";
        release_ref = "";
      };
    };
    environments.local-test = {
      enabled = true;
      strategy = "single_active";
      host = "vm-single-host";
      active_release = "candidate-release";
      retained_releases = [ "candidate-release" ];
      serve = true;
    };
  });
in
pkgs.testers.runNixOSTest {
  name = "fishystuff-gitops-active-retained-release-refusal";

  nodes.machine =
    { ... }:
    {
      system.stateVersion = "25.11";
      networking.hostName = "vm-single-host";
      virtualisation.memorySize = 4096;
      virtualisation.additionalPaths = [
        apiArtifact
        doltServiceArtifact
        siteArtifact
        currentCdnRoot
        cdnRuntimeArtifact
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
    machine.succeed("jq -e '.mode == \"vm-test\" and .environments.\"local-test\".serve == true and .environments.\"local-test\".active_release == \"candidate-release\" and .environments.\"local-test\".retained_releases == [\"candidate-release\"]' ${desiredState}")
    machine.fail("timeout 15s env FISHYSTUFF_GITOPS_STATE_FILE=${desiredState} ${mgmtPackage}/bin/mgmt run --hostname vm-single-host --tmp-prefix --no-pgp --client-urls=http://127.0.0.1:2379 --server-urls=http://127.0.0.1:2380 --advertise-client-urls=http://127.0.0.1:2379 --advertise-server-urls=http://127.0.0.1:2380 --converged-timeout=-1 lang ${gitopsSrc}/main.mcl >/tmp/fishystuff-gitops-active-retained-release-refusal.log 2>&1")
    machine.succeed("test ! -e /var/lib/fishystuff/gitops-test/status/local-test.json")
    machine.succeed("test ! -e /var/lib/fishystuff/gitops-test/instances/local-test-candidate-release.json")
    machine.succeed("test ! -e /var/lib/fishystuff/gitops-test/active/local-test.json")
    machine.succeed("test ! -e /var/lib/fishystuff/gitops-test/rollback/local-test.json")
    machine.succeed("test ! -e /run/fishystuff/gitops-test/routes/local-test.json")
    machine.succeed("test ! -e /srv/fishystuff")
    machine.succeed("test ! -e /var/lib/fishystuff/mgmt")
  '';
}
