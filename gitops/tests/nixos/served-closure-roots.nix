{
  pkgs,
  mgmtPackage,
  gitopsSrc,
}:
let
  activeApiArtifact = pkgs.writeText "fishystuff-gitops-served-closure-active-api" "active api\n";
  activeDoltServiceArtifact = pkgs.writeText "fishystuff-gitops-served-closure-active-dolt-service" "active dolt service\n";
  retainedApiArtifact = pkgs.writeText "fishystuff-gitops-served-closure-retained-api" "retained api\n";
  retainedDoltServiceArtifact = pkgs.writeText "fishystuff-gitops-served-closure-retained-dolt-service" "retained dolt service\n";
  activeSiteArtifact = pkgs.runCommand "fishystuff-gitops-served-closure-active-site" { } ''
    mkdir -p "$out"
    printf 'active served closure site\n' > "$out/index.html"
  '';
  retainedSiteArtifact = pkgs.runCommand "fishystuff-gitops-served-closure-retained-site" { } ''
    mkdir -p "$out"
    printf 'retained served closure site\n' > "$out/index.html"
  '';
  activeCdnCurrentRoot = pkgs.runCommand "fishystuff-gitops-served-closure-active-cdn-current" { } ''
    mkdir -p "$out/map"
    printf '{"module":"fishystuff_ui_bevy.active.js","wasm":"fishystuff_ui_bevy_bg.active.wasm"}\n' > "$out/map/runtime-manifest.json"
    printf 'active runtime\n' > "$out/map/fishystuff_ui_bevy.active.js"
    printf 'active wasm\n' > "$out/map/fishystuff_ui_bevy_bg.active.wasm"
  '';
  retainedCdnCurrentRoot = pkgs.runCommand "fishystuff-gitops-served-closure-retained-cdn-current" { } ''
    mkdir -p "$out/map"
    printf '{"module":"fishystuff_ui_bevy.retained.js","wasm":"fishystuff_ui_bevy_bg.retained.wasm"}\n' > "$out/map/runtime-manifest.json"
    printf 'retained runtime\n' > "$out/map/fishystuff_ui_bevy.retained.js"
    printf 'retained wasm\n' > "$out/map/fishystuff_ui_bevy_bg.retained.wasm"
  '';
  retainedCdnServingRoot = pkgs.callPackage ../../../nix/packages/cdn-serving-root.nix {
    currentRoot = retainedCdnCurrentRoot;
  };
  activeCdnServingRoot = pkgs.callPackage ../../../nix/packages/cdn-serving-root.nix {
    currentRoot = activeCdnCurrentRoot;
    previousRoots = [ retainedCdnCurrentRoot ];
  };
  artifact =
    releaseId: name: storePath:
    {
      enabled = true;
      store_path = "${storePath}";
      gcroot_path = "/nix/var/nix/gcroots/fishystuff/gitops-test/${releaseId}/${name}";
    };
  release =
    {
      generation,
      releaseId,
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
        api = artifact releaseId "api" api;
        site = artifact releaseId "site" site;
        cdn_runtime = artifact releaseId "cdn-runtime" cdn;
        dolt_service = artifact releaseId "dolt-service" doltService;
      };
      dolt = {
        repository = "fishystuff/fishystuff";
        commit = doltCommit;
        branch_context = "local-test";
        mode = "read_only";
      };
    };
  expectedReleaseIdentity = "release=active-release;generation=40;git_rev=active-served-closure;dolt_commit=active-served-closure;dolt_repository=fishystuff/fishystuff;dolt_branch_context=local-test;dolt_mode=read_only;api=${activeApiArtifact};site=${activeSiteArtifact};cdn_runtime=${activeCdnServingRoot};dolt_service=${activeDoltServiceArtifact}";
  desiredState = pkgs.writeText "vm-served-closure-roots.desired.json" (builtins.toJSON {
    cluster = "local-test";
    generation = 40;
    mode = "vm-test-closures";
    hosts.vm-single-host = {
      enabled = true;
      role = "single-site";
      hostname = "vm-single-host";
    };
    releases = {
      active-release = release {
        generation = 40;
        releaseId = "active-release";
        gitRev = "active-served-closure";
        doltCommit = "active-served-closure";
        api = activeApiArtifact;
        site = activeSiteArtifact;
        cdn = activeCdnServingRoot;
        doltService = activeDoltServiceArtifact;
      };
      retained-release = release {
        generation = 39;
        releaseId = "retained-release";
        gitRev = "retained-served-closure";
        doltCommit = "retained-served-closure";
        api = retainedApiArtifact;
        site = retainedSiteArtifact;
        cdn = retainedCdnServingRoot;
        doltService = retainedDoltServiceArtifact;
      };
    };
    environments.local-test = {
      enabled = true;
      strategy = "single_active";
      host = "vm-single-host";
      active_release = "active-release";
      retained_releases = [ "retained-release" ];
      serve = true;
    };
  });
in
pkgs.testers.runNixOSTest {
  name = "fishystuff-gitops-served-closure-roots";

  nodes.machine =
    { ... }:
    {
      system.stateVersion = "25.11";
      networking.hostName = "vm-single-host";
      virtualisation.memorySize = 12288;
      virtualisation.additionalPaths = [
        activeApiArtifact
        activeDoltServiceArtifact
        retainedApiArtifact
        retainedDoltServiceArtifact
        activeSiteArtifact
        retainedSiteArtifact
        activeCdnCurrentRoot
        retainedCdnCurrentRoot
        activeCdnServingRoot
        retainedCdnServingRoot
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
    machine.succeed("jq -e '.mode == \"vm-test-closures\" and .environments.\"local-test\".serve == true and .environments.\"local-test\".active_release == \"active-release\" and .environments.\"local-test\".retained_releases == [\"retained-release\"]' ${desiredState}")
    machine.succeed("jq -e '.retained_roots == [\"${retainedCdnCurrentRoot}\"]' ${activeCdnServingRoot}/cdn-serving-manifest.json")
    machine.succeed("jq -e '.current_root == \"${retainedCdnCurrentRoot}\"' ${retainedCdnServingRoot}/cdn-serving-manifest.json")
    machine.succeed("env FISHYSTUFF_GITOPS_STATE_FILE=${desiredState} ${mgmtPackage}/bin/mgmt run --hostname vm-single-host --tmp-prefix --no-pgp --client-urls=http://127.0.0.1:2379 --server-urls=http://127.0.0.1:2380 --advertise-client-urls=http://127.0.0.1:2379 --advertise-server-urls=http://127.0.0.1:2380 --converged-timeout=-1 lang ${gitopsSrc}/main.mcl >/tmp/fishystuff-gitops-served-closure-roots.log 2>&1 & echo $! >/tmp/fishystuff-gitops-served-closure-roots.pid")

    roots = {
      "active-release/api": "${activeApiArtifact}",
      "active-release/site": "${activeSiteArtifact}",
      "active-release/cdn-runtime": "${activeCdnServingRoot}",
      "active-release/dolt-service": "${activeDoltServiceArtifact}",
      "retained-release/api": "${retainedApiArtifact}",
      "retained-release/site": "${retainedSiteArtifact}",
      "retained-release/cdn-runtime": "${retainedCdnServingRoot}",
      "retained-release/dolt-service": "${retainedDoltServiceArtifact}",
    }

    for name, target in roots.items():
      root = f"/nix/var/nix/gcroots/fishystuff/gitops-test/{name}"
      machine.succeed(f"bash -c 'deadline=$((SECONDS + 300)); until test -L {root}; do if ! kill -0 $(cat /tmp/fishystuff-gitops-served-closure-roots.pid); then cat /tmp/fishystuff-gitops-served-closure-roots.log; exit 1; fi; if [ \"$SECONDS\" -ge \"$deadline\" ]; then cat /tmp/fishystuff-gitops-served-closure-roots.log; exit 1; fi; sleep 1; done'")
      machine.succeed(f"test \"$(readlink {root})\" = \"{target}\"")
      machine.succeed(f"nix-store --gc --print-roots | grep -F {root}")
      machine.succeed(f"nix-store --verify-path {target}")

    status = "/var/lib/fishystuff/gitops-test/status/local-test.json"
    active = "/var/lib/fishystuff/gitops-test/active/local-test.json"
    route = "/run/fishystuff/gitops-test/routes/local-test.json"
    instance = "/var/lib/fishystuff/gitops-test/instances/local-test-active-release.json"
    admission = "/run/fishystuff/gitops-test/admission/local-test.json"

    machine.wait_for_file(status)
    machine.wait_for_file(active)
    machine.wait_for_file(route)
    machine.wait_for_file(instance)
    machine.wait_for_file(admission)

    machine.succeed(f"jq -e '.desired_generation == 40 and .release_id == \"active-release\" and .release_identity == \"${expectedReleaseIdentity}\" and .environment == \"local-test\" and .host == \"vm-single-host\" and .phase == \"served\" and .admission_state == \"passed_fixture\" and .served == true and .retained_release_ids == [\"retained-release\"]' {status}")
    machine.succeed(f"jq -e '.desired_generation == 40 and .release_id == \"active-release\" and .release_identity == \"${expectedReleaseIdentity}\" and .site_content == \"${activeSiteArtifact}\" and .cdn_runtime_content == \"${activeCdnServingRoot}\" and .retained_release_ids == [\"retained-release\"] and .route_state == \"selected_local_symlinks\"' {active}")
    machine.succeed(f"jq -e '.desired_generation == 40 and .release_id == \"active-release\" and .release_identity == \"${expectedReleaseIdentity}\" and .site_root == \"/var/lib/fishystuff/gitops-test/served/local-test/site\" and .cdn_root == \"/var/lib/fishystuff/gitops-test/served/local-test/cdn\" and .served == true and .state == \"selected_local_route\"' {route}")
    machine.succeed(f"jq -e '.serve_requested == true and .release_id == \"active-release\" and .release_identity == \"${expectedReleaseIdentity}\" and .api_bundle == \"${activeApiArtifact}\" and .dolt_service_bundle == \"${activeDoltServiceArtifact}\" and .site_content == \"${activeSiteArtifact}\" and .cdn_runtime_content == \"${activeCdnServingRoot}\" and .retained_release_ids == [\"retained-release\"]' {instance}")
    machine.succeed(f"jq -e '.release_identity == \"${expectedReleaseIdentity}\" and .site_content == \"${activeSiteArtifact}\" and .cdn_runtime_content == \"${activeCdnServingRoot}\" and .cdn_runtime_module == \"fishystuff_ui_bevy.active.js\" and .cdn_runtime_wasm == \"fishystuff_ui_bevy_bg.active.wasm\" and .cdn_serving_current_root == \"${activeCdnCurrentRoot}\" and .cdn_serving_retained_root_count == 1 and .serving_artifacts_checked == true and .admission_state == \"passed_fixture\" and .probe == \"local-fixture\"' {admission}")
    machine.succeed("test \"$(readlink /var/lib/fishystuff/gitops-test/served/local-test/site)\" = \"${activeSiteArtifact}\"")
    machine.succeed("test \"$(readlink /var/lib/fishystuff/gitops-test/served/local-test/cdn)\" = \"${activeCdnServingRoot}\"")
    machine.succeed("test \"$(cat ${activeCdnServingRoot}/map/fishystuff_ui_bevy.active.js)\" = \"active runtime\"")
    machine.succeed("test \"$(cat ${activeCdnServingRoot}/map/fishystuff_ui_bevy.retained.js)\" = \"retained runtime\"")

    machine.succeed("kill $(cat /tmp/fishystuff-gitops-served-closure-roots.pid) || true")

    machine.fail("systemctl is-active fishystuff-api.service")
    machine.fail("systemctl is-active fishystuff-dolt.service")
    machine.fail("systemctl is-active fishystuff-edge.service")
    machine.succeed("test ! -e /srv/fishystuff")
    machine.succeed("test ! -e /var/lib/fishystuff/mgmt")
    machine.succeed("! find /var/lib/fishystuff/gitops-test /run/fishystuff/gitops-test -type f -print0 | xargs -0 grep -E 'beta\\.fishystuff\\.fish|production|cloudflare|hcloud|ssh '")
  '';
}
