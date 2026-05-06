{
  pkgs,
  mgmtPackage,
  fishystuffDeployPackage,
  gitopsSrc,
  desiredState,
  activeApiArtifact,
  activeSiteArtifact,
  activeCdnRuntimeArtifact,
  activeCdnRuntimeCurrentArtifact,
  activeDoltServiceArtifact,
  candidateApiArtifact,
  candidateSiteArtifact,
  candidateCdnRuntimeArtifact,
  candidateCdnRuntimeCurrentArtifact,
  candidateDoltServiceArtifact,
}:
pkgs.testers.runNixOSTest {
  name = "fishystuff-gitops-production-rollback-transition";

  nodes.machine =
    { ... }:
    {
      system.stateVersion = "25.11";
      networking.hostName = "production-single-host";
      virtualisation.memorySize = 12288;
      virtualisation.additionalPaths = [
        desiredState
        activeApiArtifact
        activeSiteArtifact
        activeCdnRuntimeArtifact
        activeCdnRuntimeCurrentArtifact
        activeDoltServiceArtifact
        candidateApiArtifact
        candidateSiteArtifact
        candidateCdnRuntimeArtifact
        candidateCdnRuntimeCurrentArtifact
        candidateDoltServiceArtifact
      ];
      environment.systemPackages = [
        mgmtPackage
        fishystuffDeployPackage
        pkgs.jq
      ];
    };

  testScript = ''
    start_all()

    machine.succeed("test -x ${mgmtPackage}/bin/mgmt")
    machine.succeed("test -x ${fishystuffDeployPackage}/bin/fishystuff_deploy")
    machine.succeed("jq -e '.cluster == \"production\" and .mode == \"vm-test\" and .generation == 3 and .environments.production.serve == true and .environments.production.host == \"production-single-host\" and .environments.production.active_release == \"previous-production-release\" and .environments.production.transition.kind == \"rollback\" and .environments.production.transition.reason == \"production rollback fixture\"' ${desiredState}")

    release_id = "previous-production-release"
    candidate_release_id = machine.succeed("jq -r '.environments.production.transition.from_release' ${desiredState}").strip()
    machine.succeed(f"jq -e --arg candidate_release_id \"{candidate_release_id}\" '.environments.production.retained_releases == [$candidate_release_id] and .releases[$candidate_release_id].generation == 2 and .releases[$candidate_release_id].dolt.branch_context == \"main\"' ${desiredState}")

    expected_release_identity = f"release={release_id};generation=1;git_rev=previous-production-vm-serve-fixture;dolt_commit=previous-production-vm-serve-fixture;dolt_repository=fishystuff/fishystuff;dolt_branch_context=main;dolt_mode=read_only;api=${activeApiArtifact};site=${activeSiteArtifact};cdn_runtime=${activeCdnRuntimeArtifact};dolt_service=${activeDoltServiceArtifact}"

    machine.succeed("env FISHYSTUFF_GITOPS_STATE_FILE=${desiredState} ${mgmtPackage}/bin/mgmt run --hostname production-single-host --tmp-prefix --no-pgp --client-urls=http://127.0.0.1:2379 --server-urls=http://127.0.0.1:2380 --advertise-client-urls=http://127.0.0.1:2379 --advertise-server-urls=http://127.0.0.1:2380 --converged-timeout=-1 lang ${gitopsSrc}/main.mcl >/tmp/fishystuff-gitops-production-rollback-mgmt.log 2>&1 & echo $! >/tmp/fishystuff-gitops-production-rollback-mgmt.pid")

    status = "/var/lib/fishystuff/gitops-test/status/production.json"
    active = "/var/lib/fishystuff/gitops-test/active/production.json"
    route = "/run/fishystuff/gitops-test/routes/production.json"
    instance = f"/var/lib/fishystuff/gitops-test/instances/production-{release_id}.json"
    admission = "/run/fishystuff/gitops-test/admission/production.json"
    rollback = "/var/lib/fishystuff/gitops-test/rollback/production.json"
    rollback_set = "/var/lib/fishystuff/gitops-test/rollback-set/production.json"
    candidate_rollback_member = f"/var/lib/fishystuff/gitops-test/rollback-set/production/{candidate_release_id}.json"

    machine.wait_for_file(status)
    machine.wait_for_file(active)
    machine.wait_for_file(route)
    machine.wait_for_file(instance)
    machine.wait_for_file(admission)
    machine.wait_for_file(rollback)
    machine.wait_for_file(rollback_set)
    machine.wait_for_file(candidate_rollback_member)

    machine.succeed(f"jq -e --arg candidate_release_id \"{candidate_release_id}\" '.desired_generation == 3 and .release_id == \"{release_id}\" and .release_identity == \"{expected_release_identity}\" and .environment == \"production\" and .host == \"production-single-host\" and .phase == \"served\" and .admission_state == \"passed_fixture\" and .served == true and .retained_release_ids == [$candidate_release_id] and .rollback_available == true and .rollback_primary_release_id == $candidate_release_id and .rollback_retained_count == 1 and .transition_kind == \"rollback\" and .rollback_from_release == $candidate_release_id and .rollback_to_release == \"{release_id}\" and .rollback_reason == \"production rollback fixture\"' {status}")
    machine.succeed(f"jq -e --arg candidate_release_id \"{candidate_release_id}\" '.desired_generation == 3 and .environment == \"production\" and .host == \"production-single-host\" and .release_id == \"{release_id}\" and .release_identity == \"{expected_release_identity}\" and .instance_name == \"production-{release_id}\" and .site_content == \"${activeSiteArtifact}\" and .cdn_runtime_content == \"${activeCdnRuntimeArtifact}\" and .site_link == \"/var/lib/fishystuff/gitops-test/served/production/site\" and .cdn_link == \"/var/lib/fishystuff/gitops-test/served/production/cdn\" and .retained_release_ids == [$candidate_release_id] and .admission_state == \"passed_fixture\" and .served == true and .transition_kind == \"rollback\" and .rollback_from_release == $candidate_release_id and .rollback_to_release == \"{release_id}\" and .rollback_reason == \"production rollback fixture\" and .route_state == \"selected_local_symlinks\"' {active}")
    machine.succeed(f"jq -e '.desired_generation == 3 and .environment == \"production\" and .host == \"production-single-host\" and .release_id == \"{release_id}\" and .release_identity == \"{expected_release_identity}\" and .site_root == \"/var/lib/fishystuff/gitops-test/served/production/site\" and .cdn_root == \"/var/lib/fishystuff/gitops-test/served/production/cdn\" and .served == true and .state == \"selected_local_route\"' {route}")
    machine.succeed(f"jq -e --arg candidate_release_id \"{candidate_release_id}\" '.serve_requested == true and .release_id == \"{release_id}\" and .release_identity == \"{expected_release_identity}\" and .api_bundle == \"${activeApiArtifact}\" and .dolt_service_bundle == \"${activeDoltServiceArtifact}\" and .site_content == \"${activeSiteArtifact}\" and .cdn_runtime_content == \"${activeCdnRuntimeArtifact}\" and .retained_release_ids == [$candidate_release_id]' {instance}")
    machine.succeed(f"jq -e '.release_identity == \"{expected_release_identity}\" and .site_content == \"${activeSiteArtifact}\" and .cdn_runtime_content == \"${activeCdnRuntimeArtifact}\" and .cdn_runtime_module == \"fishystuff_ui_bevy.previous-fixture.js\" and .cdn_runtime_wasm == \"fishystuff_ui_bevy_bg.previous-fixture.wasm\" and .cdn_serving_current_root == \"${activeCdnRuntimeCurrentArtifact}\" and .cdn_serving_retained_root_count == 1 and .serving_artifacts_checked == true and .admission_state == \"passed_fixture\" and .probe == \"local-fixture\"' {admission}")
    machine.succeed(f"jq -e --arg candidate_release_id \"{candidate_release_id}\" '.desired_generation == 3 and .current_release_id == \"{release_id}\" and .rollback_release_id == $candidate_release_id and .rollback_api_bundle == \"${candidateApiArtifact}\" and .rollback_dolt_service_bundle == \"${candidateDoltServiceArtifact}\" and .rollback_site_content == \"${candidateSiteArtifact}\" and .rollback_cdn_runtime_content == \"${candidateCdnRuntimeArtifact}\" and .rollback_dolt_commit == \"production-vm-serve-fixture\" and .rollback_available == true and .rollback_state == \"retained_hot_release\"' {rollback}")
    machine.succeed(f"jq -e --arg candidate_release_id \"{candidate_release_id}\" '.desired_generation == 3 and .current_release_id == \"{release_id}\" and .retained_release_count == 1 and .retained_release_ids == [$candidate_release_id] and .retained_release_document_paths == [\"{candidate_rollback_member}\"] and .rollback_set_available == true and .rollback_set_state == \"retained_hot_release_set\"' {rollback_set}")
    machine.succeed(f"jq -e --arg candidate_release_id \"{candidate_release_id}\" '.desired_generation == 3 and .current_release_id == \"{release_id}\" and .release_id == $candidate_release_id and .api_bundle == \"${candidateApiArtifact}\" and .dolt_service_bundle == \"${candidateDoltServiceArtifact}\" and .site_content == \"${candidateSiteArtifact}\" and .cdn_runtime_content == \"${candidateCdnRuntimeArtifact}\" and .dolt_commit == \"production-vm-serve-fixture\" and .dolt_materialization == \"metadata_only\" and .rollback_member_state == \"retained_hot_release\"' {candidate_rollback_member}")

    machine.succeed("test \"$(readlink /var/lib/fishystuff/gitops-test/served/production/site)\" = \"${activeSiteArtifact}\"")
    machine.succeed("test \"$(readlink /var/lib/fishystuff/gitops-test/served/production/cdn)\" = \"${activeCdnRuntimeArtifact}\"")
    machine.succeed("test -f ${activeSiteArtifact}/index.html")
    machine.succeed("jq -e '.module == \"fishystuff_ui_bevy.previous-fixture.js\" and .wasm == \"fishystuff_ui_bevy_bg.previous-fixture.wasm\"' ${activeCdnRuntimeArtifact}/map/runtime-manifest.json")
    machine.succeed("jq -e '.schema_version == 1 and .current_root == \"${activeCdnRuntimeCurrentArtifact}\" and .retained_roots == [\"${candidateCdnRuntimeCurrentArtifact}\"] and .retained_root_count == 1' ${activeCdnRuntimeArtifact}/cdn-serving-manifest.json")

    machine.succeed(f"${fishystuffDeployPackage}/bin/fishystuff_deploy gitops check-served --status {status} --active {active} --rollback-set {rollback_set} --rollback {rollback} --environment production --host production-single-host --release-id {release_id}")
    machine.succeed(f"${fishystuffDeployPackage}/bin/fishystuff_deploy gitops summary-served --status {status} --active {active} --rollback-set {rollback_set} --rollback {rollback} --environment production --host production-single-host --release-id {release_id} | grep -Fx 'served_release: {release_id}'")
    machine.succeed(f"${fishystuffDeployPackage}/bin/fishystuff_deploy gitops summary-served --status {status} --active {active} --rollback-set {rollback_set} --rollback {rollback} --environment production --host production-single-host --release-id {release_id} | grep -Fx \"retained_rollback_releases: {candidate_release_id}\"")

    machine.succeed("kill $(cat /tmp/fishystuff-gitops-production-rollback-mgmt.pid) || true")

    machine.fail("systemctl is-active fishystuff-api.service")
    machine.fail("systemctl is-active fishystuff-dolt.service")
    machine.fail("systemctl is-active fishystuff-edge.service")
    machine.succeed("test ! -e /srv/fishystuff")
    machine.succeed("test ! -e /var/lib/fishystuff/gitops")
    machine.succeed("test ! -e /var/lib/fishystuff/mgmt")
    machine.succeed("test ! -e /var/lib/fishystuff/gitops/gcroots")
    machine.succeed("test ! -e /nix/var/nix/gcroots/fishystuff/gitops")
    machine.succeed("! find /var/lib/fishystuff/gitops-test /run/fishystuff/gitops-test -type f -print0 | xargs -0 grep -E 'beta\\.fishystuff\\.fish|cloudflare|hcloud|ssh '")
  '';
}
