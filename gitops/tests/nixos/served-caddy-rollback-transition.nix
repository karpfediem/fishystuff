{
  pkgs,
  mgmtPackage,
  gitopsSrc,
}:
let
  previousApi = pkgs.writeText "fishystuff-gitops-caddy-rollback-previous-api" "previous api\n";
  candidateApi = pkgs.writeText "fishystuff-gitops-caddy-rollback-candidate-api" "candidate api\n";
  previousDoltService = pkgs.writeText "fishystuff-gitops-caddy-rollback-previous-dolt-service" "previous dolt service\n";
  candidateDoltService = pkgs.writeText "fishystuff-gitops-caddy-rollback-candidate-dolt-service" "candidate dolt service\n";
  previousSite = pkgs.runCommand "fishystuff-gitops-caddy-rollback-previous-site" { } ''
    mkdir -p "$out"
    printf 'previous caddy rollback site\n' > "$out/index.html"
  '';
  candidateSite = pkgs.runCommand "fishystuff-gitops-caddy-rollback-candidate-site" { } ''
    mkdir -p "$out"
    printf 'candidate caddy rollback site\n' > "$out/index.html"
  '';
  previousCdnRoot = pkgs.runCommand "fishystuff-gitops-caddy-rollback-previous-cdn-current" { } ''
    mkdir -p "$out/map"
    printf '{"module":"fishystuff_ui_bevy.previous-caddy-rollback.js","wasm":"fishystuff_ui_bevy_bg.previous-caddy-rollback.wasm"}\n' > "$out/map/runtime-manifest.json"
    printf 'previous caddy rollback module\n' > "$out/map/fishystuff_ui_bevy.previous-caddy-rollback.js"
    printf 'previous caddy rollback wasm\n' > "$out/map/fishystuff_ui_bevy_bg.previous-caddy-rollback.wasm"
  '';
  candidateCdnRoot = pkgs.runCommand "fishystuff-gitops-caddy-rollback-candidate-cdn-current" { } ''
    mkdir -p "$out/map"
    printf '{"module":"fishystuff_ui_bevy.candidate-caddy-rollback.js","wasm":"fishystuff_ui_bevy_bg.candidate-caddy-rollback.wasm"}\n' > "$out/map/runtime-manifest.json"
    printf 'candidate caddy rollback module\n' > "$out/map/fishystuff_ui_bevy.candidate-caddy-rollback.js"
    printf 'candidate caddy rollback wasm\n' > "$out/map/fishystuff_ui_bevy_bg.candidate-caddy-rollback.wasm"
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
        materialization = "metadata_only";
        remote_url = "";
        cache_dir = "";
        release_ref = "";
      };
    };
  host = {
    enabled = true;
    role = "single-site";
    hostname = "vm-single-host";
  };
  candidateServedState = pkgs.writeText "vm-served-caddy-rollback-candidate.desired.json" (builtins.toJSON {
    cluster = "local-test";
    generation = 80;
    mode = "vm-test";
    hosts.vm-single-host = host;
    releases = {
      previous-release = release {
        generation = 79;
        gitRev = "previous-caddy-rollback";
        doltCommit = "previous-caddy-rollback";
        api = previousApi;
        site = previousSite;
        cdn = previousCdnServingRoot;
        doltService = previousDoltService;
      };
      candidate-release = release {
        generation = 80;
        gitRev = "candidate-caddy-rollback";
        doltCommit = "candidate-caddy-rollback";
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
  rollbackServedState = pkgs.writeText "vm-served-caddy-rollback-previous.desired.json" (builtins.toJSON {
    cluster = "local-test";
    generation = 81;
    mode = "vm-test";
    hosts.vm-single-host = host;
    releases = {
      previous-release = release {
        generation = 81;
        gitRev = "previous-caddy-rollback";
        doltCommit = "previous-caddy-rollback";
        api = previousApi;
        site = previousSite;
        cdn = rollbackCdnServingRoot;
        doltService = previousDoltService;
      };
      candidate-release = release {
        generation = 80;
        gitRev = "candidate-caddy-rollback";
        doltCommit = "candidate-caddy-rollback";
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
        reason = "caddy rollback transition test";
      };
    };
  });
  caddyfile = pkgs.writeText "fishystuff-gitops-caddy-rollback.Caddyfile" ''
    {
      admin off
      auto_https off
    }

    :18081 {
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
  name = "fishystuff-gitops-served-caddy-rollback-transition";

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
    machine.succeed("jq -e '.retained_roots == [\"${previousCdnRoot}\"]' ${candidateCdnServingRoot}/cdn-serving-manifest.json")
    machine.succeed("jq -e '.retained_roots == [\"${candidateCdnRoot}\"]' ${rollbackCdnServingRoot}/cdn-serving-manifest.json")

    run_mgmt = "env FISHYSTUFF_GITOPS_STATE_FILE={state} ${mgmtPackage}/bin/mgmt run --hostname vm-single-host --tmp-prefix --no-pgp --client-urls=http://127.0.0.1:2379 --server-urls=http://127.0.0.1:2380 --advertise-client-urls=http://127.0.0.1:2379 --advertise-server-urls=http://127.0.0.1:2380 --converged-timeout=-1 lang ${gitopsSrc}/main.mcl >{log} 2>&1 & echo $! >{pid}"
    active = "/var/lib/fishystuff/gitops-test/active/local-test.json"
    status = "/var/lib/fishystuff/gitops-test/status/local-test.json"
    rollback = "/var/lib/fishystuff/gitops-test/rollback/local-test.json"
    rollback_set = "/var/lib/fishystuff/gitops-test/rollback-set/local-test.json"
    route = "/run/fishystuff/gitops-test/routes/local-test.json"

    machine.succeed(run_mgmt.format(state="${candidateServedState}", log="/tmp/fishystuff-gitops-caddy-rollback-candidate.log", pid="/tmp/fishystuff-gitops-caddy-rollback-candidate.pid"))
    machine.wait_for_file(active)
    machine.wait_for_file(status)
    machine.wait_for_file(rollback)
    machine.wait_for_file(rollback_set)
    machine.wait_for_file(route)
    machine.wait_until_succeeds(f"jq -e '.desired_generation == 80 and .release_id == \"candidate-release\" and .site_content == \"${candidateSite}\" and .cdn_runtime_content == \"${candidateCdnServingRoot}\" and .transition_kind == \"activate\"' {active}")
    machine.wait_until_succeeds(f"jq -e '.desired_generation == 80 and .release_id == \"candidate-release\" and .state == \"selected_local_route\"' {route}")
    machine.succeed("test \"$(readlink /var/lib/fishystuff/gitops-test/served/local-test/site)\" = \"${candidateSite}\"")
    machine.succeed("test \"$(readlink /var/lib/fishystuff/gitops-test/served/local-test/cdn)\" = \"${candidateCdnServingRoot}\"")

    machine.succeed("${pkgs.caddy}/bin/caddy run --config ${caddyfile} --adapter caddyfile >/tmp/fishystuff-gitops-caddy-rollback.log 2>&1 & echo $! >/tmp/fishystuff-gitops-caddy-rollback.pid")
    machine.wait_until_succeeds("curl -fsS http://127.0.0.1:18081/ | grep -Fx 'candidate caddy rollback site'")
    machine.succeed("curl -fsS http://127.0.0.1:18081/cdn/map/runtime-manifest.json | jq -e '.module == \"fishystuff_ui_bevy.candidate-caddy-rollback.js\" and .wasm == \"fishystuff_ui_bevy_bg.candidate-caddy-rollback.wasm\"'")
    machine.succeed("test \"$(curl -fsS http://127.0.0.1:18081/cdn/map/fishystuff_ui_bevy.candidate-caddy-rollback.js)\" = \"candidate caddy rollback module\"")
    machine.succeed("test \"$(curl -fsS http://127.0.0.1:18081/cdn/map/fishystuff_ui_bevy.previous-caddy-rollback.js)\" = \"previous caddy rollback module\"")
    machine.succeed("kill $(cat /tmp/fishystuff-gitops-caddy-rollback-candidate.pid) || true")
    machine.succeed("timeout 15s bash -c 'pid=$(cat /tmp/fishystuff-gitops-caddy-rollback-candidate.pid); while kill -0 \"$pid\" 2>/dev/null; do sleep 0.2; done'")

    machine.succeed(run_mgmt.format(state="${rollbackServedState}", log="/tmp/fishystuff-gitops-caddy-rollback-previous.log", pid="/tmp/fishystuff-gitops-caddy-rollback-previous.pid"))
    machine.wait_until_succeeds(f"jq -e '.desired_generation == 81 and .release_id == \"previous-release\" and .site_content == \"${previousSite}\" and .cdn_runtime_content == \"${rollbackCdnServingRoot}\" and .transition_kind == \"rollback\" and .rollback_from_release == \"candidate-release\" and .rollback_to_release == \"previous-release\" and .rollback_reason == \"caddy rollback transition test\"' {active}")
    machine.wait_until_succeeds(f"jq -e '.desired_generation == 81 and .release_id == \"previous-release\" and .phase == \"served\" and .transition_kind == \"rollback\" and .rollback_from_release == \"candidate-release\" and .rollback_to_release == \"previous-release\"' {status}")
    machine.wait_until_succeeds(f"jq -e '.desired_generation == 81 and .current_release_id == \"previous-release\" and .rollback_release_id == \"candidate-release\" and .rollback_available == true' {rollback}")
    machine.wait_until_succeeds(f"jq -e '.desired_generation == 81 and .current_release_id == \"previous-release\" and .retained_release_ids == [\"candidate-release\"] and .rollback_set_available == true' {rollback_set}")
    machine.wait_until_succeeds(f"jq -e '.desired_generation == 81 and .release_id == \"previous-release\" and .state == \"selected_local_route\"' {route}")
    machine.wait_until_succeeds("curl -fsS http://127.0.0.1:18081/ | grep -Fx 'previous caddy rollback site'")
    machine.succeed("curl -fsS http://127.0.0.1:18081/cdn/map/runtime-manifest.json | jq -e '.module == \"fishystuff_ui_bevy.previous-caddy-rollback.js\" and .wasm == \"fishystuff_ui_bevy_bg.previous-caddy-rollback.wasm\"'")
    machine.succeed("test \"$(curl -fsS http://127.0.0.1:18081/cdn/map/fishystuff_ui_bevy.previous-caddy-rollback.js)\" = \"previous caddy rollback module\"")
    machine.succeed("test \"$(curl -fsS http://127.0.0.1:18081/cdn/map/fishystuff_ui_bevy.candidate-caddy-rollback.js)\" = \"candidate caddy rollback module\"")
    machine.succeed("test \"$(readlink /var/lib/fishystuff/gitops-test/served/local-test/site)\" = \"${previousSite}\"")
    machine.succeed("test \"$(readlink /var/lib/fishystuff/gitops-test/served/local-test/cdn)\" = \"${rollbackCdnServingRoot}\"")
    machine.succeed("kill $(cat /tmp/fishystuff-gitops-caddy-rollback-previous.pid) || true")
    machine.succeed("timeout 15s bash -c 'pid=$(cat /tmp/fishystuff-gitops-caddy-rollback-previous.pid); while kill -0 \"$pid\" 2>/dev/null; do sleep 0.2; done'")
    machine.succeed("kill $(cat /tmp/fishystuff-gitops-caddy-rollback.pid) || true")

    machine.fail("systemctl is-active fishystuff-api.service")
    machine.fail("systemctl is-active fishystuff-dolt.service")
    machine.fail("systemctl is-active fishystuff-edge.service")
    machine.succeed("test ! -e /srv/fishystuff")
    machine.succeed("test ! -e /var/lib/fishystuff/mgmt")
    machine.succeed("! find /var/lib/fishystuff/gitops-test /run/fishystuff/gitops-test -type f -print0 | xargs -0 grep -E 'beta\\.fishystuff\\.fish|production|cloudflare|hcloud|ssh '")
  '';
}
