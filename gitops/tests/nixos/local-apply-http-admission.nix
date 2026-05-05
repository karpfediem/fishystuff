{
  pkgs,
  mgmtPackage,
  fishystuffDeployPackage,
  gitopsSrc,
}:
let
  apiArtifact = pkgs.writeText "fishystuff-gitops-local-apply-http-api-bundle" "local apply http api bundle\n";
  doltServiceArtifact = pkgs.writeText "fishystuff-gitops-local-apply-http-dolt-service-bundle" "local apply http dolt service bundle\n";
  previousApiArtifact = pkgs.writeText "fishystuff-gitops-local-apply-http-previous-api-bundle" "previous local apply http api bundle\n";
  previousDoltServiceArtifact = pkgs.writeText "fishystuff-gitops-local-apply-http-previous-dolt-service-bundle" "previous local apply http dolt service bundle\n";
  siteArtifact = pkgs.runCommand "fishystuff-gitops-local-apply-http-site-content" { } ''
    mkdir -p "$out"
    printf 'local apply http site\n' > "$out/index.html"
  '';
  previousSiteArtifact = pkgs.runCommand "fishystuff-gitops-local-apply-http-previous-site-content" { } ''
    mkdir -p "$out"
    printf 'previous local apply http site\n' > "$out/index.html"
  '';
  currentCdnRoot = pkgs.runCommand "fishystuff-gitops-local-apply-http-current-cdn-root" { } ''
    mkdir -p "$out/map"
    printf '{"module":"fishystuff_ui_bevy.local_apply_http.js","wasm":"fishystuff_ui_bevy_bg.local_apply_http.wasm"}\n' > "$out/map/runtime-manifest.json"
    printf 'local apply http runtime\n' > "$out/map/fishystuff_ui_bevy.local_apply_http.js"
    printf 'local apply http source map\n' > "$out/map/fishystuff_ui_bevy.local_apply_http.js.map"
    printf 'local apply http wasm\n' > "$out/map/fishystuff_ui_bevy_bg.local_apply_http.wasm"
    printf 'local apply http wasm source map\n' > "$out/map/fishystuff_ui_bevy_bg.local_apply_http.wasm.map"
  '';
  previousCdnRoot = pkgs.runCommand "fishystuff-gitops-local-apply-http-previous-cdn-root" { } ''
    mkdir -p "$out/map"
    printf '{"module":"fishystuff_ui_bevy.previous_local_apply_http.js","wasm":"fishystuff_ui_bevy_bg.previous_local_apply_http.wasm"}\n' > "$out/map/runtime-manifest.json"
    printf 'previous local apply http runtime\n' > "$out/map/fishystuff_ui_bevy.previous_local_apply_http.js"
    printf 'previous local apply http source map\n' > "$out/map/fishystuff_ui_bevy.previous_local_apply_http.js.map"
    printf 'previous local apply http wasm\n' > "$out/map/fishystuff_ui_bevy_bg.previous_local_apply_http.wasm"
    printf 'previous local apply http wasm source map\n' > "$out/map/fishystuff_ui_bevy_bg.previous_local_apply_http.wasm.map"
  '';
  cdnServingRoot = pkgs.callPackage ../../../nix/packages/cdn-serving-root.nix {
    currentRoot = currentCdnRoot;
    previousRoots = [ previousCdnRoot ];
  };
  expectedReleaseIdentity = "release=local-apply-http-release;generation=62;git_rev=local-apply-http-admission;dolt_commit=local-apply-http-admission;dolt_repository=fishystuff/fishystuff;dolt_branch_context=local-test;dolt_mode=read_only;api=${apiArtifact};site=${siteArtifact};cdn_runtime=${cdnServingRoot};dolt_service=${doltServiceArtifact}";
  desiredState = pkgs.writeText "vm-local-apply-http-admission.desired.json" (builtins.toJSON {
    cluster = "local-test";
    generation = 62;
    mode = "local-apply";
    hosts.vm-single-host = {
      enabled = true;
      role = "single-site";
      hostname = "vm-single-host";
    };
    releases.local-apply-http-release = {
      generation = 62;
      git_rev = "local-apply-http-admission";
      dolt_commit = "local-apply-http-admission";
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
          store_path = "${cdnServingRoot}";
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
        commit = "local-apply-http-admission";
        branch_context = "local-test";
        mode = "read_only";
      };
    };
    releases.previous-release = {
      generation = 61;
      git_rev = "previous-local-apply-http-admission";
      dolt_commit = "previous-local-apply-http-admission";
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
          store_path = "${previousCdnRoot}";
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
        commit = "previous-local-apply-http-admission";
        branch_context = "local-test";
        mode = "read_only";
      };
    };
    environments.local-test = {
      enabled = true;
      strategy = "single_active";
      host = "vm-single-host";
      active_release = "local-apply-http-release";
      retained_releases = [ "previous-release" ];
      serve = true;
      api_upstream = "http://127.0.0.1:18082";
      admission_probe = {
        kind = "http_json_scalar";
        probe_name = "api-meta";
        url = "http://127.0.0.1:18082/api/v1/meta";
        expected_status = 200;
        timeout_ms = 2000;
        json_pointer = "/release_id";
        expected_scalar = "local-apply-http-release";
      };
    };
  });
in
pkgs.testers.runNixOSTest {
  name = "fishystuff-gitops-local-apply-http-admission";

  nodes.machine =
    { ... }:
    {
      system.stateVersion = "25.11";
      networking.hostName = "vm-single-host";
      virtualisation.memorySize = 12288;
      virtualisation.additionalPaths = [
        apiArtifact
        doltServiceArtifact
        previousApiArtifact
        previousDoltServiceArtifact
        siteArtifact
        previousSiteArtifact
        currentCdnRoot
        previousCdnRoot
        cdnServingRoot
        desiredState
      ];
      environment.systemPackages = [
        fishystuffDeployPackage
        mgmtPackage
        pkgs.curl
        pkgs.jq
        pkgs.python3
      ];
    };

  testScript = ''
    start_all()

    machine.succeed("test -x ${mgmtPackage}/bin/mgmt")
    machine.succeed("test -x ${fishystuffDeployPackage}/bin/fishystuff_deploy")
    machine.succeed("test -x /run/current-system/sw/bin/fishystuff_deploy")
    machine.succeed("jq -e '.mode == \"local-apply\" and .environments.\"local-test\".serve == true and .environments.\"local-test\".api_upstream == \"http://127.0.0.1:18082\" and .environments.\"local-test\".admission_probe.kind == \"http_json_scalar\"' ${desiredState}")
    machine.succeed("mkdir -p /tmp/fishystuff-gitops-http-probe/api/v1 && printf '{\"release_id\":\"local-apply-http-release\",\"dolt_commit\":\"local-apply-http-admission\",\"state\":\"ok\"}\n' >/tmp/fishystuff-gitops-http-probe/api/v1/meta")
    machine.succeed("python3 -m http.server 18082 --bind 127.0.0.1 --directory /tmp/fishystuff-gitops-http-probe >/tmp/fishystuff-gitops-http-probe.log 2>&1 & echo $! >/tmp/fishystuff-gitops-http-probe.pid")
    machine.wait_until_succeeds("curl -fsS http://127.0.0.1:18082/api/v1/meta | jq -e '.release_id == \"local-apply-http-release\"'", timeout=30)
    machine.succeed("env FISHYSTUFF_GITOPS_ENABLE_LOCAL_APPLY=1 FISHYSTUFF_GITOPS_STATE_FILE=${desiredState} ${mgmtPackage}/bin/mgmt run --hostname vm-single-host --tmp-prefix --no-pgp --client-urls=http://127.0.0.1:2379 --server-urls=http://127.0.0.1:2380 --advertise-client-urls=http://127.0.0.1:2379 --advertise-server-urls=http://127.0.0.1:2380 --converged-timeout=-1 lang ${gitopsSrc}/main.mcl >/tmp/fishystuff-gitops-local-apply-http-admission.log 2>&1 & echo $! >/tmp/fishystuff-gitops-local-apply-http-admission.pid")

    status = "/var/lib/fishystuff/gitops/status/local-test.json"
    active = "/var/lib/fishystuff/gitops/active/local-test.json"
    route = "/run/fishystuff/gitops/routes/local-test.json"
    admission = "/run/fishystuff/gitops/admission/local-test.json"
    request = "/var/lib/fishystuff/gitops/admission/requests/local-test-local-apply-http-release-http-json-scalar.json"
    instance = "/var/lib/fishystuff/gitops/instances/local-test-local-apply-http-release.json"
    rollback_set = "/var/lib/fishystuff/gitops/rollback-set/local-test.json"

    machine.wait_for_file(request)
    machine.wait_for_file(admission)
    machine.wait_for_file(status)
    machine.wait_for_file(active)
    machine.wait_for_file(route)
    machine.wait_for_file(instance)
    machine.wait_for_file(rollback_set)
    machine.succeed(f"jq -e '.environment == \"local-test\" and .host == \"vm-single-host\" and .release_id == \"local-apply-http-release\" and .probe_name == \"api-meta\" and .url == \"http://127.0.0.1:18082/api/v1/meta\" and .expected_status == 200 and .timeout_ms == 2000 and .json_pointer == \"/release_id\" and .expected_scalar == \"local-apply-http-release\"' {request}")
    machine.succeed(f"jq -e '.environment == \"local-test\" and .host == \"vm-single-host\" and .release_id == \"local-apply-http-release\" and .release_identity == \"${expectedReleaseIdentity}\" and .probe_name == \"api-meta\" and .url == \"http://127.0.0.1:18082/api/v1/meta\" and .expected_status == 200 and .observed_status == 200 and .json_pointer == \"/release_id\" and .scalar == \"local-apply-http-release\" and .expected_scalar == \"local-apply-http-release\" and .admission_state == \"passed_fixture\" and .probe == \"http-json-scalar\"' {admission}")
    machine.succeed(f"jq -e '.desired_generation == 62 and .release_id == \"local-apply-http-release\" and .release_identity == \"${expectedReleaseIdentity}\" and .environment == \"local-test\" and .host == \"vm-single-host\" and .phase == \"served\" and .admission_state == \"passed_fixture\" and .served == true and .retained_release_ids == [\"previous-release\"]' {status}")
    machine.succeed(f"jq -e '.desired_generation == 62 and .environment == \"local-test\" and .host == \"vm-single-host\" and .release_id == \"local-apply-http-release\" and .release_identity == \"${expectedReleaseIdentity}\" and .api_upstream == \"http://127.0.0.1:18082\" and .site_link == \"/var/lib/fishystuff/gitops/served/local-test/site\" and .cdn_link == \"/var/lib/fishystuff/gitops/served/local-test/cdn\" and .admission_state == \"passed_fixture\" and .served == true and .route_state == \"selected_local_symlinks\"' {active}")
    machine.succeed(f"jq -e '.desired_generation == 62 and .environment == \"local-test\" and .host == \"vm-single-host\" and .release_id == \"local-apply-http-release\" and .release_identity == \"${expectedReleaseIdentity}\" and .api_upstream == \"http://127.0.0.1:18082\" and .active_path == \"/var/lib/fishystuff/gitops/active/local-test.json\" and .site_root == \"/var/lib/fishystuff/gitops/served/local-test/site\" and .cdn_root == \"/var/lib/fishystuff/gitops/served/local-test/cdn\" and .served == true and .state == \"selected_local_route\"' {route}")
    machine.succeed(f"jq -e '.desired_generation == 62 and .release_id == \"local-apply-http-release\" and .api_upstream == \"http://127.0.0.1:18082\" and .serve_requested == true' {instance}")
    machine.succeed("test \"$(readlink /var/lib/fishystuff/gitops/served/local-test/site)\" = \"${siteArtifact}\"")
    machine.succeed("test \"$(readlink /var/lib/fishystuff/gitops/served/local-test/cdn)\" = \"${cdnServingRoot}\"")
    machine.succeed("test ! -e /var/lib/fishystuff/gitops-test")
    machine.succeed("test ! -e /run/fishystuff/gitops-test")
    machine.succeed("test ! -e /tmp/fishystuff-gitops-test")
    machine.succeed("kill $(cat /tmp/fishystuff-gitops-local-apply-http-admission.pid) || true")
    machine.succeed("kill $(cat /tmp/fishystuff-gitops-http-probe.pid) || true")

    machine.fail("systemctl is-active fishystuff-api.service")
    machine.fail("systemctl is-active fishystuff-dolt.service")
    machine.fail("systemctl is-active fishystuff-edge.service")
    machine.succeed("test ! -e /srv/fishystuff")
    machine.succeed("test ! -e /var/lib/fishystuff/mgmt")
    machine.succeed("! find /var/lib/fishystuff/gitops /run/fishystuff/gitops -type f -print0 | xargs -0 grep -E 'beta\\.fishystuff\\.fish|production|cloudflare|hcloud|ssh '")
  '';
}
