{
  pkgs,
  mgmtPackage,
  gitopsSrc,
  desiredState,
  apiArtifact,
  siteArtifact,
  cdnRuntimeArtifact,
  doltServiceArtifact,
}:
pkgs.testers.runNixOSTest {
  name = "fishystuff-gitops-generated-served-candidate";

  nodes.machine =
    { ... }:
    {
      system.stateVersion = "25.11";
      networking.hostName = "vm-single-host";
      virtualisation.additionalPaths = [
        desiredState
        apiArtifact
        siteArtifact
        cdnRuntimeArtifact
        doltServiceArtifact
      ];
      environment.systemPackages = [
        mgmtPackage
        pkgs.jq
      ];
    };

  testScript = ''
    start_all()

    machine.succeed("test -x ${mgmtPackage}/bin/mgmt")
    machine.succeed("jq -e '.mode == \"vm-test\" and .generation == 7 and .environments.\"local-test\".serve == true and ([.releases[.environments.\"local-test\".active_release].closures[] | .enabled] | all)' ${desiredState}")

    release_id = machine.succeed("jq -r '.environments.\"local-test\".active_release' ${desiredState}").strip()
    assert release_id != "example-release"
    expected_release_identity = f"release={release_id};generation=7;git_rev=serve-fixture;dolt_commit=serve-fixture;dolt_repository=fishystuff/fishystuff;dolt_branch_context=local-test;dolt_mode=read_only;api=${apiArtifact};site=${siteArtifact};cdn_runtime=${cdnRuntimeArtifact};dolt_service=${doltServiceArtifact}"

    machine.succeed("env FISHYSTUFF_GITOPS_STATE_FILE=${desiredState} ${mgmtPackage}/bin/mgmt run --hostname vm-single-host --tmp-prefix --no-pgp --client-urls=http://127.0.0.1:2379 --server-urls=http://127.0.0.1:2380 --advertise-client-urls=http://127.0.0.1:2379 --advertise-server-urls=http://127.0.0.1:2380 --converged-timeout=-1 lang ${gitopsSrc}/main.mcl >/tmp/fishystuff-gitops-mgmt.log 2>&1 & echo $! >/tmp/fishystuff-gitops-mgmt.pid")

    status = "/var/lib/fishystuff/gitops-test/status/local-test.json"
    active = "/var/lib/fishystuff/gitops-test/active/local-test.json"
    instance = f"/var/lib/fishystuff/gitops-test/instances/local-test-{release_id}.json"
    admission = "/run/fishystuff/gitops-test/admission/local-test.json"

    machine.wait_for_file(status)
    machine.wait_for_file(active)
    machine.wait_for_file(instance)
    machine.wait_for_file(admission)

    machine.succeed(f"jq -e '.desired_generation == 7 and .release_id == \"{release_id}\" and .release_identity == \"{expected_release_identity}\" and .environment == \"local-test\" and .host == \"vm-single-host\" and .phase == \"served\" and .admission_state == \"passed_fixture\" and .served == true and .retained_release_ids == []' {status}")
    machine.succeed(f"jq -e '.environment == \"local-test\" and .host == \"vm-single-host\" and .release_id == \"{release_id}\" and .release_identity == \"{expected_release_identity}\" and .instance_name == \"local-test-{release_id}\" and .site_content == \"${siteArtifact}\" and .cdn_runtime_content == \"${cdnRuntimeArtifact}\" and .retained_release_ids == [] and .admission_state == \"passed_fixture\" and .served == true and .route_state == \"selected_local_fixture\"' {active}")
    machine.succeed(f"jq -e '.serve_requested == true and .release_id == \"{release_id}\" and .release_identity == \"{expected_release_identity}\" and .api_bundle == \"${apiArtifact}\" and .dolt_service_bundle == \"${doltServiceArtifact}\" and .site_content == \"${siteArtifact}\" and .cdn_runtime_content == \"${cdnRuntimeArtifact}\" and .retained_release_ids == []' {instance}")
    machine.succeed(f"jq -e '.release_identity == \"{expected_release_identity}\" and .site_content == \"${siteArtifact}\" and .cdn_runtime_content == \"${cdnRuntimeArtifact}\" and .cdn_runtime_module == \"fishystuff_ui_bevy.fixture.js\" and .cdn_runtime_wasm == \"fishystuff_ui_bevy_bg.fixture.wasm\" and .serving_artifacts_checked == true and .admission_state == \"passed_fixture\" and .probe == \"local-fixture\"' {admission}")

    machine.succeed("test \"$(cat ${apiArtifact})\" = \"api fixture\"")
    machine.succeed("test \"$(cat ${doltServiceArtifact})\" = \"dolt service fixture\"")
    machine.succeed("test \"$(cat ${siteArtifact}/index.html)\" = \"served fixture site\"")
    machine.succeed("jq -e '.module == \"fishystuff_ui_bevy.fixture.js\" and .wasm == \"fishystuff_ui_bevy_bg.fixture.wasm\"' ${cdnRuntimeArtifact}/map/runtime-manifest.json")
    machine.succeed("test \"$(cat ${cdnRuntimeArtifact}/map/fishystuff_ui_bevy.fixture.js)\" = \"fixture module\"")
    machine.succeed("test \"$(cat ${cdnRuntimeArtifact}/map/fishystuff_ui_bevy_bg.fixture.wasm)\" = \"fixture wasm\"")

    machine.succeed("kill $(cat /tmp/fishystuff-gitops-mgmt.pid)")

    machine.fail("systemctl is-active fishystuff-api.service")
    machine.fail("systemctl is-active fishystuff-dolt.service")
    machine.fail("systemctl is-active fishystuff-edge.service")
    machine.succeed("test ! -e /srv/fishystuff")
    machine.succeed("test ! -e /var/lib/fishystuff/mgmt")
    machine.succeed("test ! -e /var/lib/fishystuff/gitops/gcroots")
    machine.succeed("! find /var/lib/fishystuff/gitops-test /run/fishystuff/gitops-test -type f -print0 | xargs -0 grep -E 'beta\\.fishystuff\\.fish|production|cloudflare|hcloud|ssh '")
  '';
}
