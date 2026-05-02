{
  pkgs,
  mgmtPackage,
  gitopsSrc,
}:
let
  oldApi = pkgs.writeText "fishystuff-gitops-transition-old-api" "old api\n";
  previousApi = pkgs.writeText "fishystuff-gitops-transition-previous-api" "previous api\n";
  candidateApi = pkgs.writeText "fishystuff-gitops-transition-candidate-api" "candidate api\n";
  oldDoltService = pkgs.writeText "fishystuff-gitops-transition-old-dolt-service" "old dolt service\n";
  previousDoltService = pkgs.writeText "fishystuff-gitops-transition-previous-dolt-service" "previous dolt service\n";
  candidateDoltService = pkgs.writeText "fishystuff-gitops-transition-candidate-dolt-service" "candidate dolt service\n";
  oldSite = pkgs.runCommand "fishystuff-gitops-transition-old-site" { } ''
    mkdir -p "$out"
    printf 'old site\n' > "$out/index.html"
  '';
  previousSite = pkgs.runCommand "fishystuff-gitops-transition-previous-site" { } ''
    mkdir -p "$out"
    printf 'previous site\n' > "$out/index.html"
  '';
  candidateSite = pkgs.runCommand "fishystuff-gitops-transition-candidate-site" { } ''
    mkdir -p "$out"
    printf 'candidate site\n' > "$out/index.html"
  '';
  oldCdnRoot = pkgs.runCommand "fishystuff-gitops-transition-old-cdn-current" { } ''
    mkdir -p "$out/map"
    printf '{"module":"fishystuff_ui_bevy.old.js","wasm":"fishystuff_ui_bevy_bg.old.wasm"}\n' > "$out/map/runtime-manifest.json"
    printf 'old module\n' > "$out/map/fishystuff_ui_bevy.old.js"
    printf 'old wasm\n' > "$out/map/fishystuff_ui_bevy_bg.old.wasm"
  '';
  previousCdnRoot = pkgs.runCommand "fishystuff-gitops-transition-previous-cdn-current" { } ''
    mkdir -p "$out/map"
    printf '{"module":"fishystuff_ui_bevy.previous.js","wasm":"fishystuff_ui_bevy_bg.previous.wasm"}\n' > "$out/map/runtime-manifest.json"
    printf 'previous module\n' > "$out/map/fishystuff_ui_bevy.previous.js"
    printf 'previous wasm\n' > "$out/map/fishystuff_ui_bevy_bg.previous.wasm"
  '';
  candidateCdnRoot = pkgs.runCommand "fishystuff-gitops-transition-candidate-cdn-current" { } ''
    mkdir -p "$out/map"
    printf '{"module":"fishystuff_ui_bevy.candidate.js","wasm":"fishystuff_ui_bevy_bg.candidate.wasm"}\n' > "$out/map/runtime-manifest.json"
    printf 'candidate module\n' > "$out/map/fishystuff_ui_bevy.candidate.js"
    printf 'candidate wasm\n' > "$out/map/fishystuff_ui_bevy_bg.candidate.wasm"
  '';
  oldCdnServingRoot = pkgs.callPackage ../../../nix/packages/cdn-serving-root.nix {
    currentRoot = oldCdnRoot;
  };
  previousCdnServingRoot = pkgs.callPackage ../../../nix/packages/cdn-serving-root.nix {
    currentRoot = previousCdnRoot;
    previousRoots = [ oldCdnRoot ];
  };
  candidateCdnServingRoot = pkgs.callPackage ../../../nix/packages/cdn-serving-root.nix {
    currentRoot = candidateCdnRoot;
    previousRoots = [ previousCdnRoot ];
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
      inherit generation;
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
      };
    };
  host = {
    enabled = true;
    role = "single-site";
    hostname = "vm-single-host";
  };
  previousServedState = pkgs.writeText "vm-served-transition-previous.desired.json" (builtins.toJSON {
    cluster = "local-test";
    generation = 20;
    mode = "vm-test";
    hosts.vm-single-host = host;
    releases = {
      old-release = release {
        generation = 19;
        gitRev = "old-transition";
        doltCommit = "old-transition";
        api = oldApi;
        site = oldSite;
        cdn = oldCdnServingRoot;
        doltService = oldDoltService;
      };
      previous-release = release {
        generation = 20;
        gitRev = "previous-transition";
        doltCommit = "previous-transition";
        api = previousApi;
        site = previousSite;
        cdn = previousCdnServingRoot;
        doltService = previousDoltService;
      };
    };
    environments.local-test = {
      enabled = true;
      strategy = "single_active";
      host = "vm-single-host";
      active_release = "previous-release";
      retained_releases = [ "old-release" ];
      serve = true;
    };
  });
  candidateServedState = pkgs.writeText "vm-served-transition-candidate.desired.json" (builtins.toJSON {
    cluster = "local-test";
    generation = 21;
    mode = "vm-test";
    hosts.vm-single-host = host;
    releases = {
      previous-release = release {
        generation = 20;
        gitRev = "previous-transition";
        doltCommit = "previous-transition";
        api = previousApi;
        site = previousSite;
        cdn = previousCdnServingRoot;
        doltService = previousDoltService;
      };
      candidate-release = release {
        generation = 21;
        gitRev = "candidate-transition";
        doltCommit = "candidate-transition";
        api = candidateApi;
        site = candidateSite;
        cdn = candidateCdnServingRoot;
        doltService = candidateDoltService;
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
  name = "fishystuff-gitops-served-symlink-transition";

  nodes.machine =
    { ... }:
    {
      system.stateVersion = "25.11";
      networking.hostName = "vm-single-host";
      virtualisation.additionalPaths = [
        oldApi
        previousApi
        candidateApi
        oldDoltService
        previousDoltService
        candidateDoltService
        oldSite
        previousSite
        candidateSite
        oldCdnRoot
        previousCdnRoot
        candidateCdnRoot
        oldCdnServingRoot
        previousCdnServingRoot
        candidateCdnServingRoot
        previousServedState
        candidateServedState
      ];
      environment.systemPackages = [
        mgmtPackage
        pkgs.jq
      ];
    };

  testScript = ''
    start_all()

    machine.succeed("test -x ${mgmtPackage}/bin/mgmt")
    machine.succeed("jq -e '.environments.\"local-test\".active_release == \"previous-release\" and .environments.\"local-test\".retained_releases == [\"old-release\"]' ${previousServedState}")
    machine.succeed("jq -e '.environments.\"local-test\".active_release == \"candidate-release\" and .environments.\"local-test\".retained_releases == [\"previous-release\"]' ${candidateServedState}")
    machine.succeed("jq -e '.retained_roots == [\"${oldCdnRoot}\"]' ${previousCdnServingRoot}/cdn-serving-manifest.json")
    machine.succeed("jq -e '.retained_roots == [\"${previousCdnRoot}\"]' ${candidateCdnServingRoot}/cdn-serving-manifest.json")

    run_mgmt = "env FISHYSTUFF_GITOPS_STATE_FILE={state} ${mgmtPackage}/bin/mgmt run --hostname vm-single-host --tmp-prefix --no-pgp --client-urls=http://127.0.0.1:2379 --server-urls=http://127.0.0.1:2380 --advertise-client-urls=http://127.0.0.1:2379 --advertise-server-urls=http://127.0.0.1:2380 --converged-timeout=-1 lang ${gitopsSrc}/main.mcl >{log} 2>&1 & echo $! >{pid}"

    machine.succeed(run_mgmt.format(state="${previousServedState}", log="/tmp/fishystuff-gitops-transition-previous.log", pid="/tmp/fishystuff-gitops-transition-previous.pid"))
    active = "/var/lib/fishystuff/gitops-test/active/local-test.json"
    route = "/run/fishystuff/gitops-test/routes/local-test.json"
    machine.wait_for_file(active)
    machine.wait_for_file(route)
    machine.wait_until_succeeds(f"jq -e '.desired_generation == 20 and .release_id == \"previous-release\" and .site_content == \"${previousSite}\" and .cdn_runtime_content == \"${previousCdnServingRoot}\" and .site_link == \"/var/lib/fishystuff/gitops-test/served/site\" and .cdn_link == \"/var/lib/fishystuff/gitops-test/served/cdn\" and .route_state == \"selected_local_symlinks\"' {active}")
    machine.wait_until_succeeds(f"jq -e '.desired_generation == 20 and .release_id == \"previous-release\" and .site_root == \"/var/lib/fishystuff/gitops-test/served/site\" and .cdn_root == \"/var/lib/fishystuff/gitops-test/served/cdn\" and .state == \"selected_local_route\"' {route}")
    machine.succeed("test \"$(readlink /var/lib/fishystuff/gitops-test/served/site)\" = \"${previousSite}\"")
    machine.succeed("test \"$(readlink /var/lib/fishystuff/gitops-test/served/cdn)\" = \"${previousCdnServingRoot}\"")
    machine.succeed("kill $(cat /tmp/fishystuff-gitops-transition-previous.pid) || true")

    machine.succeed(run_mgmt.format(state="${candidateServedState}", log="/tmp/fishystuff-gitops-transition-candidate.log", pid="/tmp/fishystuff-gitops-transition-candidate.pid"))
    machine.wait_until_succeeds(f"jq -e '.desired_generation == 21 and .release_id == \"candidate-release\" and .site_content == \"${candidateSite}\" and .cdn_runtime_content == \"${candidateCdnServingRoot}\" and .site_link == \"/var/lib/fishystuff/gitops-test/served/site\" and .cdn_link == \"/var/lib/fishystuff/gitops-test/served/cdn\" and .route_state == \"selected_local_symlinks\"' {active}")
    machine.wait_until_succeeds(f"jq -e '.desired_generation == 21 and .release_id == \"candidate-release\" and .site_root == \"/var/lib/fishystuff/gitops-test/served/site\" and .cdn_root == \"/var/lib/fishystuff/gitops-test/served/cdn\" and .state == \"selected_local_route\"' {route}")
    machine.succeed("test \"$(readlink /var/lib/fishystuff/gitops-test/served/site)\" = \"${candidateSite}\"")
    machine.succeed("test \"$(readlink /var/lib/fishystuff/gitops-test/served/cdn)\" = \"${candidateCdnServingRoot}\"")
    machine.succeed("kill $(cat /tmp/fishystuff-gitops-transition-candidate.pid) || true")

    machine.fail("systemctl is-active fishystuff-api.service")
    machine.fail("systemctl is-active fishystuff-dolt.service")
    machine.fail("systemctl is-active fishystuff-edge.service")
    machine.succeed("test ! -e /srv/fishystuff")
    machine.succeed("test ! -e /var/lib/fishystuff/mgmt")
  '';
}
