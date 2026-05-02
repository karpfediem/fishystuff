{
  pkgs,
  mgmtPackage,
  gitopsSrc,
}:
let
  previousSite = pkgs.runCommand "fishystuff-gitops-rollback-previous-site" { } ''
    mkdir -p "$out"
    printf 'previous site\n' > "$out/index.html"
  '';
  candidateSite = pkgs.runCommand "fishystuff-gitops-rollback-candidate-site" { } ''
    mkdir -p "$out"
    printf 'candidate site\n' > "$out/index.html"
  '';
  previousCdnRoot = pkgs.runCommand "fishystuff-gitops-rollback-previous-cdn-current" { } ''
    mkdir -p "$out/map"
    printf '{"module":"fishystuff_ui_bevy.previous.js","wasm":"fishystuff_ui_bevy_bg.previous.wasm"}\n' > "$out/map/runtime-manifest.json"
    printf 'previous module\n' > "$out/map/fishystuff_ui_bevy.previous.js"
    printf 'previous wasm\n' > "$out/map/fishystuff_ui_bevy_bg.previous.wasm"
  '';
  candidateCdnRoot = pkgs.runCommand "fishystuff-gitops-rollback-candidate-cdn-current" { } ''
    mkdir -p "$out/map"
    printf '{"module":"fishystuff_ui_bevy.candidate.js","wasm":"fishystuff_ui_bevy_bg.candidate.wasm"}\n' > "$out/map/runtime-manifest.json"
    printf 'candidate module\n' > "$out/map/fishystuff_ui_bevy.candidate.js"
    printf 'candidate wasm\n' > "$out/map/fishystuff_ui_bevy_bg.candidate.wasm"
  '';
  previousCdnServingRoot = pkgs.callPackage ../../../nix/packages/cdn-serving-root.nix {
    currentRoot = previousCdnRoot;
  };
  candidateCdnServingRoot = pkgs.callPackage ../../../nix/packages/cdn-serving-root.nix {
    currentRoot = candidateCdnRoot;
    previousRoots = [ previousCdnRoot ];
  };
  rollbackCdnServingRoot = pkgs.callPackage ../../../nix/packages/cdn-serving-root.nix {
    currentRoot = previousCdnRoot;
    previousRoots = [ candidateCdnRoot ];
  };
  release =
    {
      generation,
      gitRev,
      doltCommit,
      site,
      cdn,
    }:
    {
      inherit generation;
      git_rev = gitRev;
      dolt_commit = doltCommit;
      closures = {
        api = {
          enabled = false;
          store_path = "";
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
          store_path = "";
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
  candidateServedState = pkgs.writeText "vm-served-rollback-candidate.desired.json" (builtins.toJSON {
    cluster = "local-test";
    generation = 30;
    mode = "vm-test";
    hosts.vm-single-host = host;
    releases = {
      previous-release = release {
        generation = 29;
        gitRev = "previous-rollback";
        doltCommit = "previous-rollback";
        site = previousSite;
        cdn = previousCdnServingRoot;
      };
      candidate-release = release {
        generation = 30;
        gitRev = "candidate-rollback";
        doltCommit = "candidate-rollback";
        site = candidateSite;
        cdn = candidateCdnServingRoot;
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
  rollbackServedState = pkgs.writeText "vm-served-rollback-previous.desired.json" (builtins.toJSON {
    cluster = "local-test";
    generation = 31;
    mode = "vm-test";
    hosts.vm-single-host = host;
    releases = {
      previous-release = release {
        generation = 31;
        gitRev = "previous-rollback";
        doltCommit = "previous-rollback";
        site = previousSite;
        cdn = rollbackCdnServingRoot;
      };
      candidate-release = release {
        generation = 30;
        gitRev = "candidate-rollback";
        doltCommit = "candidate-rollback";
        site = candidateSite;
        cdn = candidateCdnServingRoot;
      };
    };
    environments.local-test = {
      enabled = true;
      strategy = "single_active";
      host = "vm-single-host";
      active_release = "previous-release";
      retained_releases = [ "candidate-release" ];
      serve = true;
    };
  });
in
pkgs.testers.runNixOSTest {
  name = "fishystuff-gitops-served-rollback-transition";

  nodes.machine =
    { ... }:
    {
      system.stateVersion = "25.11";
      networking.hostName = "vm-single-host";
      virtualisation.additionalPaths = [
        previousSite
        candidateSite
        previousCdnRoot
        candidateCdnRoot
        previousCdnServingRoot
        candidateCdnServingRoot
        rollbackCdnServingRoot
        candidateServedState
        rollbackServedState
      ];
      environment.systemPackages = [
        mgmtPackage
        pkgs.jq
      ];
    };

  testScript = ''
    start_all()

    machine.succeed("test -x ${mgmtPackage}/bin/mgmt")
    machine.succeed("jq -e '.environments.\"local-test\".active_release == \"candidate-release\" and .environments.\"local-test\".retained_releases == [\"previous-release\"]' ${candidateServedState}")
    machine.succeed("jq -e '.environments.\"local-test\".active_release == \"previous-release\" and .environments.\"local-test\".retained_releases == [\"candidate-release\"]' ${rollbackServedState}")
    machine.succeed("jq -e '.retained_roots == [\"${previousCdnRoot}\"]' ${candidateCdnServingRoot}/cdn-serving-manifest.json")
    machine.succeed("jq -e '.retained_roots == [\"${candidateCdnRoot}\"]' ${rollbackCdnServingRoot}/cdn-serving-manifest.json")

    run_mgmt = "env FISHYSTUFF_GITOPS_STATE_FILE={state} ${mgmtPackage}/bin/mgmt run --hostname vm-single-host --tmp-prefix --no-pgp --client-urls=http://127.0.0.1:2379 --server-urls=http://127.0.0.1:2380 --advertise-client-urls=http://127.0.0.1:2379 --advertise-server-urls=http://127.0.0.1:2380 --converged-timeout=-1 lang ${gitopsSrc}/main.mcl >{log} 2>&1 & echo $! >{pid}"

    machine.succeed(run_mgmt.format(state="${candidateServedState}", log="/tmp/fishystuff-gitops-rollback-candidate.log", pid="/tmp/fishystuff-gitops-rollback-candidate.pid"))
    active = "/var/lib/fishystuff/gitops-test/active/local-test.json"
    machine.wait_for_file(active)
    machine.wait_until_succeeds(f"jq -e '.desired_generation == 30 and .release_id == \"candidate-release\" and .site_content == \"${candidateSite}\" and .cdn_runtime_content == \"${candidateCdnServingRoot}\" and .site_link == \"/var/lib/fishystuff/gitops-test/served/site\" and .cdn_link == \"/var/lib/fishystuff/gitops-test/served/cdn\" and .retained_release_ids == [\"previous-release\"] and .route_state == \"selected_local_symlinks\"' {active}")
    machine.succeed("test \"$(readlink /var/lib/fishystuff/gitops-test/served/site)\" = \"${candidateSite}\"")
    machine.succeed("test \"$(readlink /var/lib/fishystuff/gitops-test/served/cdn)\" = \"${candidateCdnServingRoot}\"")
    machine.succeed("kill $(cat /tmp/fishystuff-gitops-rollback-candidate.pid) || true")

    machine.succeed(run_mgmt.format(state="${rollbackServedState}", log="/tmp/fishystuff-gitops-rollback-previous.log", pid="/tmp/fishystuff-gitops-rollback-previous.pid"))
    machine.wait_until_succeeds(f"jq -e '.desired_generation == 31 and .release_id == \"previous-release\" and .site_content == \"${previousSite}\" and .cdn_runtime_content == \"${rollbackCdnServingRoot}\" and .site_link == \"/var/lib/fishystuff/gitops-test/served/site\" and .cdn_link == \"/var/lib/fishystuff/gitops-test/served/cdn\" and .retained_release_ids == [\"candidate-release\"] and .route_state == \"selected_local_symlinks\"' {active}")
    machine.succeed("test \"$(readlink /var/lib/fishystuff/gitops-test/served/site)\" = \"${previousSite}\"")
    machine.succeed("test \"$(readlink /var/lib/fishystuff/gitops-test/served/cdn)\" = \"${rollbackCdnServingRoot}\"")
    machine.succeed("kill $(cat /tmp/fishystuff-gitops-rollback-previous.pid) || true")

    machine.fail("systemctl is-active fishystuff-api.service")
    machine.fail("systemctl is-active fishystuff-dolt.service")
    machine.fail("systemctl is-active fishystuff-edge.service")
    machine.succeed("test ! -e /srv/fishystuff")
    machine.succeed("test ! -e /var/lib/fishystuff/mgmt")
  '';
}
