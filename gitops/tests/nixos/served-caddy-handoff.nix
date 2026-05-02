{
  pkgs,
  mgmtPackage,
  gitopsSrc,
}:
let
  oldApi = pkgs.writeText "fishystuff-gitops-caddy-old-api" "old api\n";
  previousApi = pkgs.writeText "fishystuff-gitops-caddy-previous-api" "previous api\n";
  candidateApi = pkgs.writeText "fishystuff-gitops-caddy-candidate-api" "candidate api\n";
  oldDoltService = pkgs.writeText "fishystuff-gitops-caddy-old-dolt-service" "old dolt service\n";
  previousDoltService = pkgs.writeText "fishystuff-gitops-caddy-previous-dolt-service" "previous dolt service\n";
  candidateDoltService = pkgs.writeText "fishystuff-gitops-caddy-candidate-dolt-service" "candidate dolt service\n";
  oldSite = pkgs.runCommand "fishystuff-gitops-caddy-old-site" { } ''
    mkdir -p "$out"
    printf 'old caddy site\n' > "$out/index.html"
  '';
  previousSite = pkgs.runCommand "fishystuff-gitops-caddy-previous-site" { } ''
    mkdir -p "$out"
    printf 'previous caddy site\n' > "$out/index.html"
  '';
  candidateSite = pkgs.runCommand "fishystuff-gitops-caddy-candidate-site" { } ''
    mkdir -p "$out"
    printf 'candidate caddy site\n' > "$out/index.html"
  '';
  oldCdnRoot = pkgs.runCommand "fishystuff-gitops-caddy-old-cdn-current" { } ''
    mkdir -p "$out/map"
    printf '{"module":"fishystuff_ui_bevy.old-caddy.js","wasm":"fishystuff_ui_bevy_bg.old-caddy.wasm"}\n' > "$out/map/runtime-manifest.json"
    printf 'old caddy module\n' > "$out/map/fishystuff_ui_bevy.old-caddy.js"
    printf 'old caddy wasm\n' > "$out/map/fishystuff_ui_bevy_bg.old-caddy.wasm"
  '';
  previousCdnRoot = pkgs.runCommand "fishystuff-gitops-caddy-previous-cdn-current" { } ''
    mkdir -p "$out/map"
    printf '{"module":"fishystuff_ui_bevy.previous-caddy.js","wasm":"fishystuff_ui_bevy_bg.previous-caddy.wasm"}\n' > "$out/map/runtime-manifest.json"
    printf 'previous caddy module\n' > "$out/map/fishystuff_ui_bevy.previous-caddy.js"
    printf 'previous caddy wasm\n' > "$out/map/fishystuff_ui_bevy_bg.previous-caddy.wasm"
  '';
  candidateCdnRoot = pkgs.runCommand "fishystuff-gitops-caddy-candidate-cdn-current" { } ''
    mkdir -p "$out/map"
    printf '{"module":"fishystuff_ui_bevy.candidate-caddy.js","wasm":"fishystuff_ui_bevy_bg.candidate-caddy.wasm"}\n' > "$out/map/runtime-manifest.json"
    printf 'candidate caddy module\n' > "$out/map/fishystuff_ui_bevy.candidate-caddy.js"
    printf 'candidate caddy wasm\n' > "$out/map/fishystuff_ui_bevy_bg.candidate-caddy.wasm"
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
  previousServedState = pkgs.writeText "vm-served-caddy-previous.desired.json" (builtins.toJSON {
    cluster = "local-test";
    generation = 70;
    mode = "vm-test";
    hosts.vm-single-host = host;
    releases = {
      old-release = release {
        generation = 69;
        gitRev = "old-caddy";
        doltCommit = "old-caddy";
        api = oldApi;
        site = oldSite;
        cdn = oldCdnServingRoot;
        doltService = oldDoltService;
      };
      previous-release = release {
        generation = 70;
        gitRev = "previous-caddy";
        doltCommit = "previous-caddy";
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
  candidateServedState = pkgs.writeText "vm-served-caddy-candidate.desired.json" (builtins.toJSON {
    cluster = "local-test";
    generation = 71;
    mode = "vm-test";
    hosts.vm-single-host = host;
    releases = {
      previous-release = release {
        generation = 70;
        gitRev = "previous-caddy";
        doltCommit = "previous-caddy";
        api = previousApi;
        site = previousSite;
        cdn = previousCdnServingRoot;
        doltService = previousDoltService;
      };
      candidate-release = release {
        generation = 71;
        gitRev = "candidate-caddy";
        doltCommit = "candidate-caddy";
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
  caddyfile = pkgs.writeText "fishystuff-gitops-caddy-handoff.Caddyfile" ''
    {
      admin off
      auto_https off
    }

    :18080 {
      handle_path /cdn/* {
        root * /var/lib/fishystuff/gitops-test/served/local-test/cdn
        file_server
      }

      handle {
        root * /var/lib/fishystuff/gitops-test/served/local-test/site
        try_files {path} {path}/index.html /index.html
        file_server
      }
    }
  '';
in
pkgs.testers.runNixOSTest {
  name = "fishystuff-gitops-served-caddy-handoff";

  nodes.machine =
    { ... }:
    {
      system.stateVersion = "25.11";
      networking.hostName = "vm-single-host";
      virtualisation.memorySize = 2048;
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
        caddyfile
      ];
      environment.systemPackages = [
        mgmtPackage
        pkgs.caddy
        pkgs.curl
        pkgs.jq
      ];
    };

  testScript = ''
    start_all()

    machine.succeed("test -x ${mgmtPackage}/bin/mgmt")
    machine.succeed("test -x ${pkgs.caddy}/bin/caddy")
    machine.succeed("jq -e '.retained_roots == [\"${oldCdnRoot}\"]' ${previousCdnServingRoot}/cdn-serving-manifest.json")
    machine.succeed("jq -e '.retained_roots == [\"${previousCdnRoot}\"]' ${candidateCdnServingRoot}/cdn-serving-manifest.json")

    run_mgmt = "env FISHYSTUFF_GITOPS_STATE_FILE={state} ${mgmtPackage}/bin/mgmt run --hostname vm-single-host --tmp-prefix --no-pgp --client-urls=http://127.0.0.1:2379 --server-urls=http://127.0.0.1:2380 --advertise-client-urls=http://127.0.0.1:2379 --advertise-server-urls=http://127.0.0.1:2380 --converged-timeout=-1 lang ${gitopsSrc}/main.mcl >{log} 2>&1 & echo $! >{pid}"
    active = "/var/lib/fishystuff/gitops-test/active/local-test.json"
    route = "/run/fishystuff/gitops-test/routes/local-test.json"

    machine.succeed(run_mgmt.format(state="${previousServedState}", log="/tmp/fishystuff-gitops-caddy-previous.log", pid="/tmp/fishystuff-gitops-caddy-previous.pid"))
    machine.wait_for_file(active)
    machine.wait_for_file(route)
    machine.wait_until_succeeds(f"jq -e '.desired_generation == 70 and .release_id == \"previous-release\" and .site_content == \"${previousSite}\" and .cdn_runtime_content == \"${previousCdnServingRoot}\"' {active}")
    machine.wait_until_succeeds(f"jq -e '.desired_generation == 70 and .release_id == \"previous-release\" and .active_path == \"{active}\" and .site_root == \"/var/lib/fishystuff/gitops-test/served/local-test/site\" and .cdn_root == \"/var/lib/fishystuff/gitops-test/served/local-test/cdn\"' {route}")
    machine.succeed("test \"$(readlink /var/lib/fishystuff/gitops-test/served/local-test/site)\" = \"${previousSite}\"")
    machine.succeed("test \"$(readlink /var/lib/fishystuff/gitops-test/served/local-test/cdn)\" = \"${previousCdnServingRoot}\"")

    machine.succeed("${pkgs.caddy}/bin/caddy run --config ${caddyfile} --adapter caddyfile >/tmp/fishystuff-gitops-caddy.log 2>&1 & echo $! >/tmp/fishystuff-gitops-caddy.pid")
    machine.wait_until_succeeds("curl -fsS http://127.0.0.1:18080/ | grep -Fx 'previous caddy site'")
    machine.succeed("curl -fsS http://127.0.0.1:18080/cdn/map/runtime-manifest.json | jq -e '.module == \"fishystuff_ui_bevy.previous-caddy.js\" and .wasm == \"fishystuff_ui_bevy_bg.previous-caddy.wasm\"'")
    machine.succeed("test \"$(curl -fsS http://127.0.0.1:18080/cdn/map/fishystuff_ui_bevy.previous-caddy.js)\" = \"previous caddy module\"")
    machine.succeed("test \"$(curl -fsS http://127.0.0.1:18080/cdn/map/fishystuff_ui_bevy.old-caddy.js)\" = \"old caddy module\"")
    machine.succeed("kill $(cat /tmp/fishystuff-gitops-caddy-previous.pid) || true")

    machine.succeed(run_mgmt.format(state="${candidateServedState}", log="/tmp/fishystuff-gitops-caddy-candidate.log", pid="/tmp/fishystuff-gitops-caddy-candidate.pid"))
    machine.wait_until_succeeds(f"jq -e '.desired_generation == 71 and .release_id == \"candidate-release\" and .site_content == \"${candidateSite}\" and .cdn_runtime_content == \"${candidateCdnServingRoot}\"' {active}")
    machine.wait_until_succeeds(f"jq -e '.desired_generation == 71 and .release_id == \"candidate-release\" and .active_path == \"{active}\" and .site_root == \"/var/lib/fishystuff/gitops-test/served/local-test/site\" and .cdn_root == \"/var/lib/fishystuff/gitops-test/served/local-test/cdn\"' {route}")
    machine.wait_until_succeeds("curl -fsS http://127.0.0.1:18080/ | grep -Fx 'candidate caddy site'")
    machine.succeed("curl -fsS http://127.0.0.1:18080/cdn/map/runtime-manifest.json | jq -e '.module == \"fishystuff_ui_bevy.candidate-caddy.js\" and .wasm == \"fishystuff_ui_bevy_bg.candidate-caddy.wasm\"'")
    machine.succeed("test \"$(curl -fsS http://127.0.0.1:18080/cdn/map/fishystuff_ui_bevy.candidate-caddy.js)\" = \"candidate caddy module\"")
    machine.succeed("test \"$(curl -fsS http://127.0.0.1:18080/cdn/map/fishystuff_ui_bevy.previous-caddy.js)\" = \"previous caddy module\"")
    machine.succeed("kill $(cat /tmp/fishystuff-gitops-caddy-candidate.pid) || true")
    machine.succeed("kill $(cat /tmp/fishystuff-gitops-caddy.pid) || true")

    machine.fail("systemctl is-active fishystuff-api.service")
    machine.fail("systemctl is-active fishystuff-dolt.service")
    machine.fail("systemctl is-active fishystuff-edge.service")
    machine.succeed("test ! -e /srv/fishystuff")
    machine.succeed("test ! -e /var/lib/fishystuff/mgmt")
    machine.succeed("! find /var/lib/fishystuff/gitops-test /run/fishystuff/gitops-test -type f -print0 | xargs -0 grep -E 'beta\\.fishystuff\\.fish|production|cloudflare|hcloud|ssh '")
  '';
}
