{
  pkgs,
  mgmtPackage,
  fishystuffDeployPackage,
  gitopsSrc,
}:
let
  previousApi = pkgs.writeText "fishystuff-gitops-rollback-previous-api" "previous api\n";
  candidateApi = pkgs.writeText "fishystuff-gitops-rollback-candidate-api" "candidate api\n";
  previousDoltService = pkgs.writeText "fishystuff-gitops-rollback-previous-dolt-service" "previous dolt service\n";
  candidateDoltService = pkgs.writeText "fishystuff-gitops-rollback-candidate-dolt-service" "candidate dolt service\n";
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
        api = previousApi;
        site = previousSite;
        cdn = previousCdnServingRoot;
        doltService = previousDoltService;
      };
      candidate-release = release {
        generation = 30;
        gitRev = "candidate-rollback";
        doltCommit = "candidate-rollback";
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
        api = previousApi;
        site = previousSite;
        cdn = rollbackCdnServingRoot;
        doltService = previousDoltService;
      };
      candidate-release = release {
        generation = 30;
        gitRev = "candidate-rollback";
        doltCommit = "candidate-rollback";
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
      active_release = "previous-release";
      retained_releases = [ "candidate-release" ];
      serve = true;
      transition = {
        kind = "rollback";
        from_release = "candidate-release";
        reason = "operator-requested rollback test";
      };
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
      virtualisation.memorySize = 12288;
      virtualisation.additionalPaths = [
        previousApi
        candidateApi
        previousDoltService
        candidateDoltService
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
        fishystuffDeployPackage
        mgmtPackage
        pkgs.jq
      ];
    };

  testScript = ''
    start_all()

    machine.succeed("test -x ${mgmtPackage}/bin/mgmt")
    machine.succeed("test -x ${fishystuffDeployPackage}/bin/fishystuff_deploy")
    machine.succeed("jq -e '.environments.\"local-test\".active_release == \"candidate-release\" and .environments.\"local-test\".retained_releases == [\"previous-release\"]' ${candidateServedState}")
    machine.succeed("jq -e '.environments.\"local-test\".active_release == \"previous-release\" and .environments.\"local-test\".retained_releases == [\"candidate-release\"]' ${rollbackServedState}")
    machine.succeed("jq -e '.retained_roots == [\"${previousCdnRoot}\"]' ${candidateCdnServingRoot}/cdn-serving-manifest.json")
    machine.succeed("jq -e '.retained_roots == [\"${candidateCdnRoot}\"]' ${rollbackCdnServingRoot}/cdn-serving-manifest.json")

    run_mgmt = "env FISHYSTUFF_GITOPS_STATE_FILE={state} ${mgmtPackage}/bin/mgmt run --hostname vm-single-host --tmp-prefix --no-pgp --client-urls=http://127.0.0.1:2379 --server-urls=http://127.0.0.1:2380 --advertise-client-urls=http://127.0.0.1:2379 --advertise-server-urls=http://127.0.0.1:2380 --converged-timeout=-1 lang ${gitopsSrc}/main.mcl >{log} 2>&1 & echo $! >{pid}"

    machine.succeed(run_mgmt.format(state="${candidateServedState}", log="/tmp/fishystuff-gitops-rollback-candidate.log", pid="/tmp/fishystuff-gitops-rollback-candidate.pid"))
    active = "/var/lib/fishystuff/gitops-test/active/local-test.json"
    status = "/var/lib/fishystuff/gitops-test/status/local-test.json"
    rollback = "/var/lib/fishystuff/gitops-test/rollback/local-test.json"
    rollback_set = "/var/lib/fishystuff/gitops-test/rollback-set/local-test.json"
    previous_rollback_member = "/var/lib/fishystuff/gitops-test/rollback-set/local-test/previous-release.json"
    candidate_rollback_member = "/var/lib/fishystuff/gitops-test/rollback-set/local-test/candidate-release.json"
    route = "/run/fishystuff/gitops-test/routes/local-test.json"
    machine.wait_for_file(active)
    machine.wait_for_file(status)
    machine.wait_for_file(rollback)
    machine.wait_for_file(rollback_set)
    machine.wait_for_file(previous_rollback_member)
    machine.wait_for_file(route)
    machine.wait_until_succeeds(f"jq -e '.desired_generation == 30 and .release_id == \"candidate-release\" and .site_content == \"${candidateSite}\" and .cdn_runtime_content == \"${candidateCdnServingRoot}\" and .site_link == \"/var/lib/fishystuff/gitops-test/served/local-test/site\" and .cdn_link == \"/var/lib/fishystuff/gitops-test/served/local-test/cdn\" and .retained_release_ids == [\"previous-release\"] and .transition_kind == \"activate\" and .rollback_from_release == \"\" and .rollback_to_release == \"\" and .route_state == \"selected_local_symlinks\"' {active}")
    machine.wait_until_succeeds(f"jq -e '.desired_generation == 30 and .release_id == \"candidate-release\" and .phase == \"served\" and .retained_release_ids == [\"previous-release\"] and .rollback_available == true and .rollback_primary_release_id == \"previous-release\" and .rollback_retained_count == 1 and .transition_kind == \"activate\" and .rollback_from_release == \"\" and .rollback_to_release == \"\"' {status}")
    machine.wait_until_succeeds(f"jq -e '.desired_generation == 30 and .current_release_id == \"candidate-release\" and .rollback_release_id == \"previous-release\" and .rollback_api_bundle == \"${previousApi}\" and .rollback_dolt_service_bundle == \"${previousDoltService}\" and .rollback_site_content == \"${previousSite}\" and .rollback_cdn_runtime_content == \"${previousCdnServingRoot}\" and .rollback_dolt_commit == \"previous-rollback\" and .rollback_dolt_materialization == \"metadata_only\" and .rollback_dolt_cache_dir == \"\" and .rollback_dolt_release_ref == \"\" and .rollback_available == true and .rollback_state == \"retained_hot_release\"' {rollback}")
    machine.wait_until_succeeds(f"jq -e '.desired_generation == 30 and .current_release_id == \"candidate-release\" and .retained_release_count == 1 and .retained_release_ids == [\"previous-release\"] and .retained_release_document_paths == [\"{previous_rollback_member}\"] and .rollback_set_available == true and .rollback_set_state == \"retained_hot_release_set\"' {rollback_set}")
    machine.wait_until_succeeds(f"jq -e '.desired_generation == 30 and .current_release_id == \"candidate-release\" and .release_id == \"previous-release\" and .api_bundle == \"${previousApi}\" and .dolt_service_bundle == \"${previousDoltService}\" and .site_content == \"${previousSite}\" and .cdn_runtime_content == \"${previousCdnServingRoot}\" and .dolt_commit == \"previous-rollback\" and .dolt_materialization == \"metadata_only\" and .dolt_cache_dir == \"\" and .dolt_release_ref == \"\" and .dolt_status_path == \"\" and .rollback_member_state == \"retained_hot_release\"' {previous_rollback_member}")
    machine.wait_until_succeeds(f"jq -e '.desired_generation == 30 and .release_id == \"candidate-release\" and .site_root == \"/var/lib/fishystuff/gitops-test/served/local-test/site\" and .cdn_root == \"/var/lib/fishystuff/gitops-test/served/local-test/cdn\" and .state == \"selected_local_route\"' {route}")
    machine.wait_until_succeeds(f"${fishystuffDeployPackage}/bin/fishystuff_deploy gitops check-served --status {status} --active {active} --rollback-set {rollback_set} --environment local-test --host vm-single-host --release-id candidate-release")
    machine.wait_until_succeeds(f"${fishystuffDeployPackage}/bin/fishystuff_deploy gitops summary-served --status {status} --active {active} --rollback-set {rollback_set} --environment local-test --host vm-single-host --release-id candidate-release | grep -Fx 'served_release: candidate-release'")
    machine.wait_until_succeeds(f"${fishystuffDeployPackage}/bin/fishystuff_deploy gitops summary-served --status {status} --active {active} --rollback-set {rollback_set} --environment local-test --host vm-single-host --release-id candidate-release | grep -Fx 'retained_rollback_releases: previous-release'")
    machine.succeed("test \"$(readlink /var/lib/fishystuff/gitops-test/served/local-test/site)\" = \"${candidateSite}\"")
    machine.succeed("test \"$(readlink /var/lib/fishystuff/gitops-test/served/local-test/cdn)\" = \"${candidateCdnServingRoot}\"")
    machine.succeed("kill $(cat /tmp/fishystuff-gitops-rollback-candidate.pid) || true")
    machine.succeed("timeout 15s bash -c 'pid=$(cat /tmp/fishystuff-gitops-rollback-candidate.pid); while kill -0 \"$pid\" 2>/dev/null; do sleep 0.2; done'")

    machine.succeed(run_mgmt.format(state="${rollbackServedState}", log="/tmp/fishystuff-gitops-rollback-previous.log", pid="/tmp/fishystuff-gitops-rollback-previous.pid"))
    machine.wait_until_succeeds(f"jq -e '.desired_generation == 31 and .release_id == \"previous-release\" and .site_content == \"${previousSite}\" and .cdn_runtime_content == \"${rollbackCdnServingRoot}\" and .site_link == \"/var/lib/fishystuff/gitops-test/served/local-test/site\" and .cdn_link == \"/var/lib/fishystuff/gitops-test/served/local-test/cdn\" and .retained_release_ids == [\"candidate-release\"] and .transition_kind == \"rollback\" and .rollback_from_release == \"candidate-release\" and .rollback_to_release == \"previous-release\" and .rollback_reason == \"operator-requested rollback test\" and .route_state == \"selected_local_symlinks\"' {active}")
    machine.wait_until_succeeds(f"jq -e '.desired_generation == 31 and .release_id == \"previous-release\" and .phase == \"served\" and .retained_release_ids == [\"candidate-release\"] and .rollback_available == true and .rollback_primary_release_id == \"candidate-release\" and .rollback_retained_count == 1 and .transition_kind == \"rollback\" and .rollback_from_release == \"candidate-release\" and .rollback_to_release == \"previous-release\" and .rollback_reason == \"operator-requested rollback test\"' {status}")
    machine.wait_until_succeeds(f"jq -e '.desired_generation == 31 and .current_release_id == \"previous-release\" and .rollback_release_id == \"candidate-release\" and .rollback_api_bundle == \"${candidateApi}\" and .rollback_dolt_service_bundle == \"${candidateDoltService}\" and .rollback_site_content == \"${candidateSite}\" and .rollback_cdn_runtime_content == \"${candidateCdnServingRoot}\" and .rollback_dolt_commit == \"candidate-rollback\" and .rollback_dolt_materialization == \"metadata_only\" and .rollback_dolt_cache_dir == \"\" and .rollback_dolt_release_ref == \"\" and .rollback_available == true and .rollback_state == \"retained_hot_release\"' {rollback}")
    machine.wait_until_succeeds(f"jq -e '.desired_generation == 31 and .current_release_id == \"previous-release\" and .retained_release_count == 1 and .retained_release_ids == [\"candidate-release\"] and .retained_release_document_paths == [\"{candidate_rollback_member}\"] and .rollback_set_available == true and .rollback_set_state == \"retained_hot_release_set\"' {rollback_set}")
    machine.wait_until_succeeds(f"jq -e '.desired_generation == 31 and .current_release_id == \"previous-release\" and .release_id == \"candidate-release\" and .api_bundle == \"${candidateApi}\" and .dolt_service_bundle == \"${candidateDoltService}\" and .site_content == \"${candidateSite}\" and .cdn_runtime_content == \"${candidateCdnServingRoot}\" and .dolt_commit == \"candidate-rollback\" and .dolt_materialization == \"metadata_only\" and .dolt_cache_dir == \"\" and .dolt_release_ref == \"\" and .dolt_status_path == \"\" and .rollback_member_state == \"retained_hot_release\"' {candidate_rollback_member}")
    machine.wait_until_succeeds(f"jq -e '.desired_generation == 31 and .release_id == \"previous-release\" and .site_root == \"/var/lib/fishystuff/gitops-test/served/local-test/site\" and .cdn_root == \"/var/lib/fishystuff/gitops-test/served/local-test/cdn\" and .state == \"selected_local_route\"' {route}")
    machine.wait_until_succeeds(f"${fishystuffDeployPackage}/bin/fishystuff_deploy gitops check-served --status {status} --active {active} --rollback-set {rollback_set} --environment local-test --host vm-single-host --release-id previous-release")
    machine.wait_until_succeeds(f"${fishystuffDeployPackage}/bin/fishystuff_deploy gitops summary-served --status {status} --active {active} --rollback-set {rollback_set} --environment local-test --host vm-single-host --release-id previous-release | grep -Fx 'served_release: previous-release'")
    machine.wait_until_succeeds(f"${fishystuffDeployPackage}/bin/fishystuff_deploy gitops summary-served --status {status} --active {active} --rollback-set {rollback_set} --environment local-test --host vm-single-host --release-id previous-release | grep -Fx 'retained_rollback_releases: candidate-release'")
    machine.succeed("test \"$(readlink /var/lib/fishystuff/gitops-test/served/local-test/site)\" = \"${previousSite}\"")
    machine.succeed("test \"$(readlink /var/lib/fishystuff/gitops-test/served/local-test/cdn)\" = \"${rollbackCdnServingRoot}\"")
    machine.succeed("kill $(cat /tmp/fishystuff-gitops-rollback-previous.pid) || true")
    machine.succeed("timeout 15s bash -c 'pid=$(cat /tmp/fishystuff-gitops-rollback-previous.pid); while kill -0 \"$pid\" 2>/dev/null; do sleep 0.2; done'")

    machine.fail("systemctl is-active fishystuff-api.service")
    machine.fail("systemctl is-active fishystuff-dolt.service")
    machine.fail("systemctl is-active fishystuff-edge.service")
    machine.succeed("test ! -e /srv/fishystuff")
    machine.succeed("test ! -e /var/lib/fishystuff/mgmt")
  '';
}
