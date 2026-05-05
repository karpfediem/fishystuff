{
  pkgs,
  mgmtPackage,
  gitopsSrc,
}:
let
  previousApi = pkgs.writeText "fishystuff-gitops-rollback-retention-previous-api" "previous api\n";
  candidateApi = pkgs.writeText "fishystuff-gitops-rollback-retention-candidate-api" "candidate api\n";
  olderApi = pkgs.writeText "fishystuff-gitops-rollback-retention-older-api" "older api\n";
  previousDoltService = pkgs.writeText "fishystuff-gitops-rollback-retention-previous-dolt-service" "previous dolt service\n";
  candidateDoltService = pkgs.writeText "fishystuff-gitops-rollback-retention-candidate-dolt-service" "candidate dolt service\n";
  olderDoltService = pkgs.writeText "fishystuff-gitops-rollback-retention-older-dolt-service" "older dolt service\n";
  previousSite = pkgs.runCommand "fishystuff-gitops-rollback-retention-previous-site" { } ''
    mkdir -p "$out"
    printf 'previous rollback retention site\n' > "$out/index.html"
  '';
  candidateSite = pkgs.runCommand "fishystuff-gitops-rollback-retention-candidate-site" { } ''
    mkdir -p "$out"
    printf 'candidate rollback retention site\n' > "$out/index.html"
  '';
  olderSite = pkgs.runCommand "fishystuff-gitops-rollback-retention-older-site" { } ''
    mkdir -p "$out"
    printf 'older rollback retention site\n' > "$out/index.html"
  '';
  previousCdnRoot = pkgs.runCommand "fishystuff-gitops-rollback-retention-previous-cdn-current" { } ''
    mkdir -p "$out/map"
    printf '{"module":"fishystuff_ui_bevy.previous.js","wasm":"fishystuff_ui_bevy_bg.previous.wasm"}\n' > "$out/map/runtime-manifest.json"
    printf 'previous rollback retention module\n' > "$out/map/fishystuff_ui_bevy.previous.js"
    printf 'previous rollback retention wasm\n' > "$out/map/fishystuff_ui_bevy_bg.previous.wasm"
  '';
  candidateCdnRoot = pkgs.runCommand "fishystuff-gitops-rollback-retention-candidate-cdn-current" { } ''
    mkdir -p "$out/map"
    printf '{"module":"fishystuff_ui_bevy.candidate.js","wasm":"fishystuff_ui_bevy_bg.candidate.wasm"}\n' > "$out/map/runtime-manifest.json"
    printf 'candidate rollback retention module\n' > "$out/map/fishystuff_ui_bevy.candidate.js"
    printf 'candidate rollback retention wasm\n' > "$out/map/fishystuff_ui_bevy_bg.candidate.wasm"
  '';
  olderCdnRoot = pkgs.runCommand "fishystuff-gitops-rollback-retention-older-cdn-current" { } ''
    mkdir -p "$out/map"
    printf '{"module":"fishystuff_ui_bevy.older.js","wasm":"fishystuff_ui_bevy_bg.older.wasm"}\n' > "$out/map/runtime-manifest.json"
    printf 'older rollback retention module\n' > "$out/map/fishystuff_ui_bevy.older.js"
    printf 'older rollback retention wasm\n' > "$out/map/fishystuff_ui_bevy_bg.older.wasm"
  '';
  rollbackCdnServingRoot = pkgs.callPackage ../../../nix/packages/cdn-serving-root.nix {
    currentRoot = previousCdnRoot;
    previousRoots = [ olderCdnRoot ];
  };
  release =
    {
      generation,
      gitRev,
      doltCommit,
      api,
      site,
      cdn,
      doltService,
    }:
    {
      generation = generation;
      git_rev = gitRev;
      dolt_commit = doltCommit;
      closures = {
        api = {
          enabled = false;
          store_path = "${api}";
          gcroot_path = "";
        };
        site = {
          enabled = false;
          store_path = "${site}";
          gcroot_path = "";
        };
        cdn_runtime = {
          enabled = false;
          store_path = "${cdn}";
          gcroot_path = "";
        };
        dolt_service = {
          enabled = false;
          store_path = "${doltService}";
          gcroot_path = "";
        };
      };
      dolt = {
        repository = "fishystuff/fishystuff";
        commit = doltCommit;
        branch_context = "local-test";
        mode = "read_only";
        materialization = "metadata_only";
        remote_url = "";
        cache_dir = "";
        release_ref = "";
      };
    };
  desiredState = pkgs.writeText "rollback-transition-retention-refusal.desired.json" (builtins.toJSON {
    cluster = "local-test";
    generation = 61;
    mode = "vm-test";
    hosts.vm-single-host = {
      enabled = true;
      role = "single-site";
      hostname = "vm-single-host";
    };
    releases = {
      previous-release = release {
        generation = 61;
        gitRev = "previous-rollback-retention";
        doltCommit = "previous-rollback-retention";
        api = previousApi;
        site = previousSite;
        cdn = rollbackCdnServingRoot;
        doltService = previousDoltService;
      };
      candidate-release = release {
        generation = 60;
        gitRev = "candidate-rollback-retention";
        doltCommit = "candidate-rollback-retention";
        api = candidateApi;
        site = candidateSite;
        cdn = candidateCdnRoot;
        doltService = candidateDoltService;
      };
      older-release = release {
        generation = 59;
        gitRev = "older-rollback-retention";
        doltCommit = "older-rollback-retention";
        api = olderApi;
        site = olderSite;
        cdn = olderCdnRoot;
        doltService = olderDoltService;
      };
    };
    environments.local-test = {
      enabled = true;
      strategy = "single_active";
      host = "vm-single-host";
      active_release = "previous-release";
      retained_releases = [ "older-release" ];
      serve = true;
      transition = {
        kind = "rollback";
        from_release = "candidate-release";
        reason = "unsafe rollback retention refusal";
      };
    };
  });
in
pkgs.testers.runNixOSTest {
  name = "fishystuff-gitops-rollback-transition-retention-refusal";

  nodes.machine =
    { ... }:
    {
      system.stateVersion = "25.11";
      networking.hostName = "vm-single-host";
      virtualisation.memorySize = 12288;
      virtualisation.additionalPaths = [
        previousApi
        candidateApi
        olderApi
        previousDoltService
        candidateDoltService
        olderDoltService
        previousSite
        candidateSite
        olderSite
        previousCdnRoot
        candidateCdnRoot
        olderCdnRoot
        rollbackCdnServingRoot
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
    machine.succeed("jq -e '.mode == \"vm-test\" and .environments.\"local-test\".serve == true and .environments.\"local-test\".transition.kind == \"rollback\" and .environments.\"local-test\".transition.from_release == \"candidate-release\" and .environments.\"local-test\".retained_releases == [\"older-release\"]' ${desiredState}")
    machine.fail("timeout 60s env FISHYSTUFF_GITOPS_STATE_FILE=${desiredState} ${mgmtPackage}/bin/mgmt run --hostname vm-single-host --tmp-prefix --no-pgp --client-urls=http://127.0.0.1:2379 --server-urls=http://127.0.0.1:2380 --advertise-client-urls=http://127.0.0.1:2379 --advertise-server-urls=http://127.0.0.1:2380 --converged-timeout=-1 lang ${gitopsSrc}/main.mcl >/tmp/fishystuff-gitops-rollback-transition-retention-refusal.log 2>&1")
    machine.succeed("test ! -e /var/lib/fishystuff/gitops-test/status/local-test.json")
    machine.succeed("test ! -e /var/lib/fishystuff/gitops-test/instances/local-test-previous-release.json")
    machine.succeed("test ! -e /var/lib/fishystuff/gitops-test/active/local-test.json")
    machine.succeed("test ! -e /var/lib/fishystuff/gitops-test/rollback/local-test.json")
    machine.succeed("test ! -e /var/lib/fishystuff/gitops-test/rollback-set/local-test.json")
    machine.succeed("test ! -e /run/fishystuff/gitops-test/routes/local-test.json")
    machine.succeed("test ! -e /srv/fishystuff")
    machine.succeed("test ! -e /var/lib/fishystuff/mgmt")
  '';
}
