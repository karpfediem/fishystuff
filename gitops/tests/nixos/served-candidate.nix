{
  pkgs,
  mgmtPackage,
  gitopsSrc,
}:
let
  siteArtifact = pkgs.runCommand "fishystuff-gitops-served-site-content" { } ''
    mkdir -p "$out"
    printf 'served site\n' > "$out/index.html"
  '';
  currentCdnRoot = pkgs.runCommand "fishystuff-gitops-served-current-cdn-root" { } ''
    mkdir -p "$out/map"
    printf 'current manifest\n' > "$out/map/runtime-manifest.json"
    printf 'current runtime\n' > "$out/map/fishystuff_ui_bevy.current.js"
    printf 'current source map\n' > "$out/map/fishystuff_ui_bevy.current.js.map"
  '';
  previousCdnRoot = pkgs.runCommand "fishystuff-gitops-served-previous-cdn-root" { } ''
    mkdir -p "$out/map"
    printf 'previous manifest\n' > "$out/map/runtime-manifest.json"
    printf 'previous runtime\n' > "$out/map/fishystuff_ui_bevy.previous.js"
    printf 'previous source map\n' > "$out/map/fishystuff_ui_bevy.previous.js.map"
  '';
  cdnServingRoot = pkgs.callPackage ../../../nix/packages/cdn-serving-root.nix {
    currentRoot = currentCdnRoot;
    previousRoots = [ previousCdnRoot ];
  };
  expectedReleaseIdentity = "release=example-release;generation=3;git_rev=served-test;dolt_commit=served-test;dolt_repository=fishystuff/fishystuff;dolt_branch_context=local-test;api=;site=${siteArtifact};cdn_runtime=${cdnServingRoot};dolt_service=";
  desiredState = pkgs.writeText "vm-served-candidate.example.desired.json" (builtins.toJSON {
    cluster = "local-test";
    generation = 3;
    mode = "vm-test";
    hosts.vm-single-host = {
      enabled = true;
      role = "single-site";
      hostname = "vm-single-host";
    };
    releases.example-release = {
      generation = 3;
      git_rev = "served-test";
      dolt_commit = "served-test";
      closures = {
        api = {
          enabled = false;
          store_path = "";
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
          store_path = "";
          gcroot_path = "";
        };
      };
      dolt = {
        repository = "fishystuff/fishystuff";
        commit = "served-test";
        branch_context = "local-test";
        mode = "read_only";
      };
    };
    releases.previous-release = {
      generation = 2;
      git_rev = "previous-served-test";
      dolt_commit = "previous-served-test";
      closures = {
        api = {
          enabled = false;
          store_path = "";
          gcroot_path = "";
        };
        site = {
          enabled = false;
          store_path = "";
          gcroot_path = "";
        };
        cdn_runtime = {
          enabled = false;
          store_path = "${previousCdnRoot}";
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
        commit = "previous-served-test";
        branch_context = "local-test";
        mode = "read_only";
      };
    };
    environments.local-test = {
      enabled = true;
      strategy = "single_active";
      host = "vm-single-host";
      active_release = "example-release";
      retained_releases = [ "previous-release" ];
      serve = true;
    };
  });
in
pkgs.testers.runNixOSTest {
  name = "fishystuff-gitops-served-candidate";

  nodes.machine =
    { ... }:
    {
      system.stateVersion = "25.11";
      networking.hostName = "vm-single-host";
      virtualisation.additionalPaths = [
        siteArtifact
        currentCdnRoot
        previousCdnRoot
        cdnServingRoot
      ];
      environment.systemPackages = [
        mgmtPackage
        pkgs.jq
      ];
    };

  testScript = ''
    start_all()

    machine.succeed("test -x ${mgmtPackage}/bin/mgmt")
    machine.succeed("env FISHYSTUFF_GITOPS_STATE_FILE=${desiredState} ${mgmtPackage}/bin/mgmt run --hostname vm-single-host --tmp-prefix --no-pgp --client-urls=http://127.0.0.1:2379 --server-urls=http://127.0.0.1:2380 --advertise-client-urls=http://127.0.0.1:2379 --advertise-server-urls=http://127.0.0.1:2380 --converged-timeout=-1 lang ${gitopsSrc}/main.mcl >/tmp/fishystuff-gitops-mgmt.log 2>&1 & echo $! >/tmp/fishystuff-gitops-mgmt.pid")

    status = "/var/lib/fishystuff/gitops-test/status/local-test.json"
    active = "/var/lib/fishystuff/gitops-test/active/local-test.json"
    instance = "/var/lib/fishystuff/gitops-test/instances/local-test-example-release.json"
    admission = "/run/fishystuff/gitops-test/admission/local-test.json"

    machine.wait_for_file(status)
    machine.wait_for_file(active)
    machine.wait_for_file(instance)
    machine.wait_for_file(admission)

    machine.succeed(f"jq -e '.desired_generation == 3 and .release_id == \"example-release\" and .release_identity == \"${expectedReleaseIdentity}\" and .environment == \"local-test\" and .host == \"vm-single-host\" and .phase == \"served\" and .admission_state == \"passed_fixture\" and .served == true and .retained_release_ids == [\"previous-release\"]' {status}")
    machine.succeed(f"jq -e '.environment == \"local-test\" and .host == \"vm-single-host\" and .release_id == \"example-release\" and .release_identity == \"${expectedReleaseIdentity}\" and .instance_name == \"local-test-example-release\" and .site_content == \"${siteArtifact}\" and .cdn_runtime_content == \"${cdnServingRoot}\" and .retained_release_ids == [\"previous-release\"] and .admission_state == \"passed_fixture\" and .served == true and .route_state == \"selected_local_fixture\"' {active}")
    machine.succeed(f"jq -e '.serve_requested == true and .release_id == \"example-release\" and .release_identity == \"${expectedReleaseIdentity}\" and .site_content == \"${siteArtifact}\" and .cdn_runtime_content == \"${cdnServingRoot}\" and .retained_release_ids == [\"previous-release\"]' {instance}")
    machine.succeed(f"jq -e '.release_identity == \"${expectedReleaseIdentity}\" and .admission_state == \"passed_fixture\" and .probe == \"local-fixture\"' {admission}")
    machine.succeed("test \"$(cat ${cdnServingRoot}/map/runtime-manifest.json)\" = \"current manifest\"")
    machine.succeed("test \"$(cat ${cdnServingRoot}/map/fishystuff_ui_bevy.current.js)\" = \"current runtime\"")
    machine.succeed("test \"$(cat ${cdnServingRoot}/map/fishystuff_ui_bevy.current.js.map)\" = \"current source map\"")
    machine.succeed("test \"$(cat ${cdnServingRoot}/map/fishystuff_ui_bevy.previous.js)\" = \"previous runtime\"")
    machine.succeed("test \"$(cat ${cdnServingRoot}/map/fishystuff_ui_bevy.previous.js.map)\" = \"previous source map\"")

    machine.succeed("kill $(cat /tmp/fishystuff-gitops-mgmt.pid)")

    machine.fail("systemctl is-active fishystuff-api.service")
    machine.fail("systemctl is-active fishystuff-dolt.service")
    machine.fail("systemctl is-active fishystuff-edge.service")
    machine.succeed("test ! -e /srv/fishystuff")
    machine.succeed("test ! -e /var/lib/fishystuff/mgmt")
    machine.succeed("! find /var/lib/fishystuff/gitops-test /run/fishystuff/gitops-test -type f -print0 | xargs -0 grep -E 'beta\\.fishystuff\\.fish|production|cloudflare|hcloud|ssh '")
  '';
}
