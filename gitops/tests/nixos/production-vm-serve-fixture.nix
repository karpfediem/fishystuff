{
  pkgs,
  mgmtPackage,
  gitopsSrc,
  desiredState,
  apiArtifact,
  siteArtifact,
  cdnRuntimeArtifact,
  cdnRuntimeCurrentArtifact,
  doltServiceArtifact,
  previousApiArtifact,
  previousSiteArtifact,
  previousCdnRuntimeArtifact,
  previousCdnRuntimeCurrentArtifact,
  previousDoltServiceArtifact,
}:
pkgs.testers.runNixOSTest {
  name = "fishystuff-gitops-production-vm-serve-fixture";

  nodes.machine =
    { ... }:
    {
      system.stateVersion = "25.11";
      networking.hostName = "production-single-host";
      virtualisation.memorySize = 12288;
      virtualisation.additionalPaths = [
        desiredState
        apiArtifact
        siteArtifact
        cdnRuntimeArtifact
        cdnRuntimeCurrentArtifact
        doltServiceArtifact
        previousApiArtifact
        previousSiteArtifact
        previousCdnRuntimeArtifact
        previousCdnRuntimeCurrentArtifact
        previousDoltServiceArtifact
      ];
      environment.systemPackages = [
        mgmtPackage
        pkgs.jq
      ];
    };

  testScript = ''
    start_all()

    machine.succeed("test -x ${mgmtPackage}/bin/mgmt")
    machine.succeed("jq -e '.cluster == \"production\" and .mode == \"vm-test\" and .generation == 2 and .environments.production.serve == true and .environments.production.host == \"production-single-host\" and .environments.production.retained_releases == [\"previous-production-release\"]' ${desiredState}")

    release_id = machine.succeed("jq -r '.environments.production.active_release' ${desiredState}").strip()
    assert release_id != "example-release"
    expected_release_identity = f"release={release_id};generation=2;git_rev=production-vm-serve-fixture;dolt_commit=production-vm-serve-fixture;dolt_repository=fishystuff/fishystuff;dolt_branch_context=main;dolt_mode=read_only;api=${apiArtifact};site=${siteArtifact};cdn_runtime=${cdnRuntimeArtifact};dolt_service=${doltServiceArtifact}"

    machine.succeed("env FISHYSTUFF_GITOPS_STATE_FILE=${desiredState} ${mgmtPackage}/bin/mgmt run --hostname production-single-host --tmp-prefix --no-pgp --client-urls=http://127.0.0.1:2379 --server-urls=http://127.0.0.1:2380 --advertise-client-urls=http://127.0.0.1:2379 --advertise-server-urls=http://127.0.0.1:2380 --converged-timeout=-1 lang ${gitopsSrc}/main.mcl >/tmp/fishystuff-gitops-production-mgmt.log 2>&1 & echo $! >/tmp/fishystuff-gitops-production-mgmt.pid")

    status = "/var/lib/fishystuff/gitops-test/status/production.json"
    active = "/var/lib/fishystuff/gitops-test/active/production.json"
    route = "/run/fishystuff/gitops-test/routes/production.json"
    instance = f"/var/lib/fishystuff/gitops-test/instances/production-{release_id}.json"
    admission = "/run/fishystuff/gitops-test/admission/production.json"
    rollback = "/var/lib/fishystuff/gitops-test/rollback/production.json"
    rollback_set = "/var/lib/fishystuff/gitops-test/rollback-set/production.json"
    previous_rollback_member = "/var/lib/fishystuff/gitops-test/rollback-set/production/previous-production-release.json"

    machine.wait_for_file(status)
    machine.wait_for_file(active)
    machine.wait_for_file(route)
    machine.wait_for_file(instance)
    machine.wait_for_file(admission)
    machine.wait_for_file(rollback)
    machine.wait_for_file(rollback_set)
    machine.wait_for_file(previous_rollback_member)

    machine.succeed(f"jq -e '.desired_generation == 2 and .release_id == \"{release_id}\" and .release_identity == \"{expected_release_identity}\" and .environment == \"production\" and .host == \"production-single-host\" and .phase == \"served\" and .admission_state == \"passed_fixture\" and .served == true and .retained_release_ids == [\"previous-production-release\"] and .rollback_available == true and .rollback_primary_release_id == \"previous-production-release\" and .rollback_retained_count == 1' {status}")
    machine.succeed(f"jq -e '.desired_generation == 2 and .environment == \"production\" and .host == \"production-single-host\" and .release_id == \"{release_id}\" and .release_identity == \"{expected_release_identity}\" and .instance_name == \"production-{release_id}\" and .site_content == \"${siteArtifact}\" and .cdn_runtime_content == \"${cdnRuntimeArtifact}\" and .site_link == \"/var/lib/fishystuff/gitops-test/served/production/site\" and .cdn_link == \"/var/lib/fishystuff/gitops-test/served/production/cdn\" and .retained_release_ids == [\"previous-production-release\"] and .admission_state == \"passed_fixture\" and .served == true and .route_state == \"selected_local_symlinks\"' {active}")
    machine.succeed(f"jq -e '.desired_generation == 2 and .environment == \"production\" and .host == \"production-single-host\" and .release_id == \"{release_id}\" and .release_identity == \"{expected_release_identity}\" and .site_root == \"/var/lib/fishystuff/gitops-test/served/production/site\" and .cdn_root == \"/var/lib/fishystuff/gitops-test/served/production/cdn\" and .served == true and .state == \"selected_local_route\"' {route}")
    machine.succeed(f"jq -e '.serve_requested == true and .release_id == \"{release_id}\" and .release_identity == \"{expected_release_identity}\" and .api_bundle == \"${apiArtifact}\" and .dolt_service_bundle == \"${doltServiceArtifact}\" and .site_content == \"${siteArtifact}\" and .cdn_runtime_content == \"${cdnRuntimeArtifact}\" and .retained_release_ids == [\"previous-production-release\"]' {instance}")
    machine.succeed(f"jq -e '.release_identity == \"{expected_release_identity}\" and .site_content == \"${siteArtifact}\" and .cdn_runtime_content == \"${cdnRuntimeArtifact}\" and .cdn_runtime_module == \"fishystuff_ui_bevy.fixture.js\" and .cdn_runtime_wasm == \"fishystuff_ui_bevy_bg.fixture.wasm\" and .cdn_serving_current_root == \"${cdnRuntimeCurrentArtifact}\" and .cdn_serving_retained_root_count == 1 and .serving_artifacts_checked == true and .admission_state == \"passed_fixture\" and .probe == \"local-fixture\"' {admission}")
    machine.succeed(f"jq -e '.desired_generation == 2 and .current_release_id == \"{release_id}\" and .rollback_release_id == \"previous-production-release\" and .rollback_api_bundle == \"${previousApiArtifact}\" and .rollback_dolt_service_bundle == \"${previousDoltServiceArtifact}\" and .rollback_site_content == \"${previousSiteArtifact}\" and .rollback_cdn_runtime_content == \"${previousCdnRuntimeArtifact}\" and .rollback_dolt_commit == \"previous-production-vm-serve-fixture\" and .rollback_available == true' {rollback}")
    machine.succeed(f"jq -e '.desired_generation == 2 and .current_release_id == \"{release_id}\" and .retained_release_count == 1 and .retained_release_ids == [\"previous-production-release\"] and .retained_release_document_paths == [\"{previous_rollback_member}\"] and .rollback_set_available == true' {rollback_set}")

    machine.succeed("test \"$(readlink /var/lib/fishystuff/gitops-test/served/production/site)\" = \"${siteArtifact}\"")
    machine.succeed("test \"$(readlink /var/lib/fishystuff/gitops-test/served/production/cdn)\" = \"${cdnRuntimeArtifact}\"")
    machine.succeed("test -f ${siteArtifact}/index.html")
    machine.succeed("jq -e '.module == \"fishystuff_ui_bevy.fixture.js\" and .wasm == \"fishystuff_ui_bevy_bg.fixture.wasm\"' ${cdnRuntimeArtifact}/map/runtime-manifest.json")
    machine.succeed("jq -e '.schema_version == 1 and .current_root == \"${cdnRuntimeCurrentArtifact}\" and .retained_root_count == 1' ${cdnRuntimeArtifact}/cdn-serving-manifest.json")
    machine.succeed("jq -e '.current_root == \"${previousCdnRuntimeCurrentArtifact}\" and .retained_root_count == 0' ${previousCdnRuntimeArtifact}/cdn-serving-manifest.json")

    machine.succeed("kill $(cat /tmp/fishystuff-gitops-production-mgmt.pid) || true")

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
