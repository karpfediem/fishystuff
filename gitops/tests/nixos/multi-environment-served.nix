{
  pkgs,
  mgmtPackage,
  gitopsSrc,
}:
let
  mkText = name: content: pkgs.writeText "fishystuff-gitops-${name}" content;
  mkSite = name: content: pkgs.runCommand "fishystuff-gitops-${name}" { } ''
    mkdir -p "$out"
    printf '%s\n' ${pkgs.lib.escapeShellArg content} > "$out/index.html"
  '';
  mkCdnCurrent = name: moduleName: wasmName: content: pkgs.runCommand "fishystuff-gitops-${name}" { } ''
    mkdir -p "$out/map"
    printf '{"module":"%s","wasm":"%s"}\n' ${pkgs.lib.escapeShellArg moduleName} ${pkgs.lib.escapeShellArg wasmName} > "$out/map/runtime-manifest.json"
    printf '%s\n' ${pkgs.lib.escapeShellArg "${content} runtime"} > "$out/map/${moduleName}"
    printf '%s\n' ${pkgs.lib.escapeShellArg "${content} wasm"} > "$out/map/${wasmName}"
  '';

  previousApi = mkText "multi-served-previous-api" "previous api\n";
  previousDoltService = mkText "multi-served-previous-dolt-service" "previous dolt service\n";
  previousSite = mkSite "multi-served-previous-site" "previous site";
  previousCdnCurrent = mkCdnCurrent
    "multi-served-previous-cdn"
    "fishystuff_ui_bevy.previous.js"
    "fishystuff_ui_bevy_bg.previous.wasm"
    "previous";

  activeAApi = mkText "multi-served-a-api" "preview a api\n";
  activeADoltService = mkText "multi-served-a-dolt-service" "preview a dolt service\n";
  activeASite = mkSite "multi-served-a-site" "preview a site";
  activeACdnCurrent = mkCdnCurrent
    "multi-served-a-cdn-current"
    "fishystuff_ui_bevy.preview-a.js"
    "fishystuff_ui_bevy_bg.preview-a.wasm"
    "preview a";
  activeACdnServing = pkgs.callPackage ../../../nix/packages/cdn-serving-root.nix {
    currentRoot = activeACdnCurrent;
    previousRoots = [ previousCdnCurrent ];
  };

  activeBApi = mkText "multi-served-b-api" "preview b api\n";
  activeBDoltService = mkText "multi-served-b-dolt-service" "preview b dolt service\n";
  activeBSite = mkSite "multi-served-b-site" "preview b site";
  activeBCdnCurrent = mkCdnCurrent
    "multi-served-b-cdn-current"
    "fishystuff_ui_bevy.preview-b.js"
    "fishystuff_ui_bevy_bg.preview-b.wasm"
    "preview b";
  activeBCdnServing = pkgs.callPackage ../../../nix/packages/cdn-serving-root.nix {
    currentRoot = activeBCdnCurrent;
    previousRoots = [ previousCdnCurrent ];
  };

  emptyRoot = "";
  artifact = storePath: {
    enabled = false;
    store_path = "${storePath}";
    gcroot_path = emptyRoot;
  };
  release = {
    generation,
    gitRev,
    doltCommit,
    api,
    site,
    cdn,
    doltService,
  }: {
    inherit generation;
    git_rev = gitRev;
    dolt_commit = doltCommit;
    closures = {
      api = artifact api;
      site = artifact site;
      cdn_runtime = artifact cdn;
      dolt_service = artifact doltService;
    };
    dolt = {
      repository = "fishystuff/fishystuff";
      commit = doltCommit;
      branch_context = "preview";
      mode = "read_only";
    };
  };

  identity = {
    releaseId,
    generation,
    gitRev,
    doltCommit,
    api,
    site,
    cdn,
    doltService,
  }: "release=${releaseId};generation=${toString generation};git_rev=${gitRev};dolt_commit=${doltCommit};dolt_repository=fishystuff/fishystuff;dolt_branch_context=preview;dolt_mode=read_only;api=${api};site=${site};cdn_runtime=${cdn};dolt_service=${doltService}";

  activeAIdentity = identity {
    releaseId = "preview-a-release";
    generation = 51;
    gitRev = "preview-a-served";
    doltCommit = "preview-a-served";
    api = activeAApi;
    site = activeASite;
    cdn = activeACdnServing;
    doltService = activeADoltService;
  };
  activeBIdentity = identity {
    releaseId = "preview-b-release";
    generation = 52;
    gitRev = "preview-b-served";
    doltCommit = "preview-b-served";
    api = activeBApi;
    site = activeBSite;
    cdn = activeBCdnServing;
    doltService = activeBDoltService;
  };

  desiredState = pkgs.writeText "vm-multi-environment-served.desired.json" (builtins.toJSON {
    cluster = "preview-local";
    generation = 50;
    mode = "vm-test";
    hosts.local-preview-host = {
      enabled = true;
      role = "single-site";
      hostname = "vm-single-host";
    };
    releases = {
      preview-a-release = release {
        generation = 51;
        gitRev = "preview-a-served";
        doltCommit = "preview-a-served";
        api = activeAApi;
        site = activeASite;
        cdn = activeACdnServing;
        doltService = activeADoltService;
      };
      preview-b-release = release {
        generation = 52;
        gitRev = "preview-b-served";
        doltCommit = "preview-b-served";
        api = activeBApi;
        site = activeBSite;
        cdn = activeBCdnServing;
        doltService = activeBDoltService;
      };
      shared-previous-release = release {
        generation = 49;
        gitRev = "shared-previous";
        doltCommit = "shared-previous";
        api = previousApi;
        site = previousSite;
        cdn = previousCdnCurrent;
        doltService = previousDoltService;
      };
    };
    environments = {
      preview-branch-a = {
        enabled = true;
        strategy = "single_active";
        host = "local-preview-host";
        active_release = "preview-a-release";
        retained_releases = [ "shared-previous-release" ];
        serve = true;
      };
      preview-branch-b = {
        enabled = true;
        strategy = "single_active";
        host = "local-preview-host";
        active_release = "preview-b-release";
        retained_releases = [ "shared-previous-release" ];
        serve = true;
      };
    };
  });
in
pkgs.testers.runNixOSTest {
  name = "fishystuff-gitops-multi-environment-served";

  nodes.machine =
    { ... }:
    {
      system.stateVersion = "25.11";
      networking.hostName = "vm-single-host";
      virtualisation.memorySize = 2048;
      virtualisation.additionalPaths = [
        previousApi
        previousDoltService
        previousSite
        previousCdnCurrent
        activeAApi
        activeADoltService
        activeASite
        activeACdnCurrent
        activeACdnServing
        activeBApi
        activeBDoltService
        activeBSite
        activeBCdnCurrent
        activeBCdnServing
      ];
      environment.systemPackages = [
        mgmtPackage
        pkgs.jq
      ];
    };

  testScript = ''
    start_all()

    machine.succeed("test -x ${mgmtPackage}/bin/mgmt")
    machine.succeed("jq -e '.mode == \"vm-test\" and (.environments | keys | length) == 2 and .environments.\"preview-branch-a\".serve == true and .environments.\"preview-branch-b\".serve == true' ${desiredState}")
    machine.succeed("env FISHYSTUFF_GITOPS_STATE_FILE=${desiredState} ${mgmtPackage}/bin/mgmt run --hostname vm-single-host --tmp-prefix --no-pgp --client-urls=http://127.0.0.1:2379 --server-urls=http://127.0.0.1:2380 --advertise-client-urls=http://127.0.0.1:2379 --advertise-server-urls=http://127.0.0.1:2380 --converged-timeout=-1 lang ${gitopsSrc}/main.mcl >/tmp/fishystuff-gitops-multi-served.log 2>&1 & echo $! >/tmp/fishystuff-gitops-multi-served.pid")

    cases = [
        ("preview-branch-a", "preview-a-release", "${activeAIdentity}", "${activeASite}", "${activeACdnServing}", "${activeACdnCurrent}", "fishystuff_ui_bevy.preview-a.js", "fishystuff_ui_bevy_bg.preview-a.wasm"),
        ("preview-branch-b", "preview-b-release", "${activeBIdentity}", "${activeBSite}", "${activeBCdnServing}", "${activeBCdnCurrent}", "fishystuff_ui_bevy.preview-b.js", "fishystuff_ui_bevy_bg.preview-b.wasm"),
    ]

    for env, release, identity, site, cdn, current_root, module_name, wasm_name in cases:
        status = f"/var/lib/fishystuff/gitops-test/status/{env}.json"
        active = f"/var/lib/fishystuff/gitops-test/active/{env}.json"
        route = f"/run/fishystuff/gitops-test/routes/{env}.json"
        instance = f"/var/lib/fishystuff/gitops-test/instances/{env}-{release}.json"
        admission = f"/run/fishystuff/gitops-test/admission/{env}.json"
        site_link = f"/var/lib/fishystuff/gitops-test/served/{env}/site"
        cdn_link = f"/var/lib/fishystuff/gitops-test/served/{env}/cdn"

        machine.wait_for_file(status)
        machine.wait_for_file(active)
        machine.wait_for_file(route)
        machine.wait_for_file(instance)
        machine.wait_for_file(admission)
        machine.succeed(f"jq -e '.desired_generation == 50 and .release_id == \"{release}\" and .release_identity == \"{identity}\" and .environment == \"{env}\" and .host == \"vm-single-host\" and .phase == \"served\" and .admission_state == \"passed_fixture\" and .served == true and .retained_release_ids == [\"shared-previous-release\"]' {status}")
        machine.succeed(f"jq -e '.desired_generation == 50 and .release_id == \"{release}\" and .release_identity == \"{identity}\" and .site_content == \"{site}\" and .cdn_runtime_content == \"{cdn}\" and .site_link == \"{site_link}\" and .cdn_link == \"{cdn_link}\" and .retained_release_ids == [\"shared-previous-release\"] and .served == true and .route_state == \"selected_local_symlinks\"' {active}")
        machine.succeed(f"jq -e '.desired_generation == 50 and .release_id == \"{release}\" and .release_identity == \"{identity}\" and .site_root == \"{site_link}\" and .cdn_root == \"{cdn_link}\" and .served == true and .state == \"selected_local_route\"' {route}")
        machine.succeed(f"jq -e '.serve_requested == true and .release_id == \"{release}\" and .release_identity == \"{identity}\" and .site_content == \"{site}\" and .cdn_runtime_content == \"{cdn}\" and .retained_release_ids == [\"shared-previous-release\"]' {instance}")
        machine.succeed(f"jq -e '.release_identity == \"{identity}\" and .site_content == \"{site}\" and .cdn_runtime_content == \"{cdn}\" and .cdn_runtime_module == \"{module_name}\" and .cdn_runtime_wasm == \"{wasm_name}\" and .cdn_serving_current_root == \"{current_root}\" and .cdn_serving_retained_root_count == 1 and .serving_artifacts_checked == true and .admission_state == \"passed_fixture\"' {admission}")
        machine.succeed(f"test \"$(readlink {site_link})\" = \"{site}\"")
        machine.succeed(f"test \"$(readlink {cdn_link})\" = \"{cdn}\"")

    machine.succeed("test ! -e /var/lib/fishystuff/gitops-test/served/site")
    machine.succeed("test ! -e /var/lib/fishystuff/gitops-test/served/cdn")
    machine.succeed("test \"$(readlink /var/lib/fishystuff/gitops-test/served/preview-branch-a/site)\" != \"$(readlink /var/lib/fishystuff/gitops-test/served/preview-branch-b/site)\"")
    machine.succeed("test \"$(readlink /var/lib/fishystuff/gitops-test/served/preview-branch-a/cdn)\" != \"$(readlink /var/lib/fishystuff/gitops-test/served/preview-branch-b/cdn)\"")
    machine.succeed("test \"$(cat ${activeACdnServing}/map/fishystuff_ui_bevy.previous.js)\" = \"previous runtime\"")
    machine.succeed("test \"$(cat ${activeBCdnServing}/map/fishystuff_ui_bevy.previous.js)\" = \"previous runtime\"")
    machine.succeed("kill $(cat /tmp/fishystuff-gitops-multi-served.pid) || true")

    machine.fail("systemctl is-active fishystuff-api.service")
    machine.fail("systemctl is-active fishystuff-dolt.service")
    machine.fail("systemctl is-active fishystuff-edge.service")
    machine.succeed("test ! -e /srv/fishystuff")
    machine.succeed("test ! -e /var/lib/fishystuff/mgmt")
    machine.succeed("! find /var/lib/fishystuff/gitops-test /run/fishystuff/gitops-test -type f -print0 | xargs -0 grep -E 'beta\\.fishystuff\\.fish|production|cloudflare|hcloud|ssh '")
  '';
}
