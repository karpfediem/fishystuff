{
  pkgs,
  mgmtPackage,
  fishystuffDeployPackage,
  edgeServiceBundleProductionGitopsHandoff,
  gitopsSrc,
}:
let
  previousApi = pkgs.writeText "fishystuff-gitops-production-edge-previous-api" "previous api\n";
  candidateApi = pkgs.writeText "fishystuff-gitops-production-edge-candidate-api" "candidate api\n";
  previousDoltService =
    pkgs.writeText "fishystuff-gitops-production-edge-previous-dolt-service" "previous dolt service\n";
  candidateDoltService =
    pkgs.writeText "fishystuff-gitops-production-edge-candidate-dolt-service" "candidate dolt service\n";
  previousSite = pkgs.runCommand "fishystuff-gitops-production-edge-previous-site" { } ''
    mkdir -p "$out"
    printf 'production edge previous site\n' > "$out/index.html"
  '';
  candidateSite = pkgs.runCommand "fishystuff-gitops-production-edge-candidate-site" { } ''
    mkdir -p "$out"
    printf 'production edge candidate site\n' > "$out/index.html"
  '';
  previousCdnRoot = pkgs.runCommand "fishystuff-gitops-production-edge-previous-cdn-current" { } ''
    mkdir -p "$out/map"
    printf '{"module":"fishystuff_ui_bevy.production-edge-previous.js","wasm":"fishystuff_ui_bevy_bg.production-edge-previous.wasm"}\n' > "$out/map/runtime-manifest.json"
    printf 'production edge previous module\n' > "$out/map/fishystuff_ui_bevy.production-edge-previous.js"
    printf 'production edge previous wasm\n' > "$out/map/fishystuff_ui_bevy_bg.production-edge-previous.wasm"
  '';
  candidateCdnRoot = pkgs.runCommand "fishystuff-gitops-production-edge-candidate-cdn-current" { } ''
    mkdir -p "$out/map"
    printf '{"module":"fishystuff_ui_bevy.production-edge-candidate.js","wasm":"fishystuff_ui_bevy_bg.production-edge-candidate.wasm"}\n' > "$out/map/runtime-manifest.json"
    printf 'production edge candidate module\n' > "$out/map/fishystuff_ui_bevy.production-edge-candidate.js"
    printf 'production edge candidate wasm\n' > "$out/map/fishystuff_ui_bevy_bg.production-edge-candidate.wasm"
  '';
  previousCdnServingRoot = pkgs.callPackage ../../../nix/packages/cdn-serving-root.nix {
    currentRoot = previousCdnRoot;
  };
  candidateCdnServingRoot = pkgs.callPackage ../../../nix/packages/cdn-serving-root.nix {
    currentRoot = candidateCdnRoot;
    previousRoots = [ previousCdnRoot ];
  };
  release =
    {
      releaseId,
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
          enabled = true;
          store_path = "${api}";
          gcroot_path = "/nix/var/nix/gcroots/fishystuff/gitops/${releaseId}/api";
        };
        site = {
          enabled = true;
          store_path = "${site}";
          gcroot_path = "/nix/var/nix/gcroots/fishystuff/gitops/${releaseId}/site";
        };
        cdn_runtime = {
          enabled = true;
          store_path = "${cdn}";
          gcroot_path = "/nix/var/nix/gcroots/fishystuff/gitops/${releaseId}/cdn-runtime";
        };
        dolt_service = {
          enabled = true;
          store_path = "${doltService}";
          gcroot_path = "/nix/var/nix/gcroots/fishystuff/gitops/${releaseId}/dolt-service";
        };
      };
      dolt = {
        repository = "fishystuff/fishystuff";
        commit = doltCommit;
        branch_context = "main";
        mode = "read_only";
        materialization = "metadata_only";
      };
    };
  releaseIdentity =
    releaseId: generation: gitRev: doltCommit: api: site: cdn: doltService:
    "release=${releaseId};generation=${toString generation};git_rev=${gitRev};dolt_commit=${doltCommit};dolt_repository=fishystuff/fishystuff;dolt_branch_context=main;dolt_mode=read_only;api=${api};site=${site};cdn_runtime=${cdn};dolt_service=${doltService}";
  candidateReleaseId = "production-edge-candidate-release";
  previousReleaseId = "previous-production-edge-release";
  candidateDoltCommit = "production-edge-candidate-dolt";
  candidateReleaseIdentity = releaseIdentity candidateReleaseId 12 "production-edge-candidate" candidateDoltCommit candidateApi candidateSite candidateCdnServingRoot candidateDoltService;
  previousReleaseIdentity =
    releaseIdentity previousReleaseId 11 "production-edge-previous" "production-edge-previous-dolt" previousApi previousSite previousCdnServingRoot previousDoltService;
  desiredState = pkgs.writeText "production-edge-handoff.desired.json" (builtins.toJSON {
    cluster = "production";
    generation = 12;
    mode = "local-apply";
    hosts.production-single-host = {
      enabled = true;
      role = "single-site";
      hostname = "production-single-host";
    };
    releases = {
      "${previousReleaseId}" = release {
        releaseId = previousReleaseId;
        generation = 11;
        gitRev = "production-edge-previous";
        doltCommit = "production-edge-previous-dolt";
        api = previousApi;
        site = previousSite;
        cdn = previousCdnServingRoot;
        doltService = previousDoltService;
      };
      "${candidateReleaseId}" = release {
        releaseId = candidateReleaseId;
        generation = 12;
        gitRev = "production-edge-candidate";
        doltCommit = candidateDoltCommit;
        api = candidateApi;
        site = candidateSite;
        cdn = candidateCdnServingRoot;
        doltService = candidateDoltService;
      };
    };
    environments.production = {
      enabled = true;
      strategy = "single_active";
      host = "production-single-host";
      active_release = candidateReleaseId;
      retained_releases = [ previousReleaseId ];
      api_upstream = "http://127.0.0.1:18092";
      admission_probe = {
        kind = "api_meta";
        probe_name = "api-meta";
        url = "http://127.0.0.1:18092/api/v1/meta";
        expected_status = 200;
        timeout_ms = 2000;
      };
      serve = true;
    };
  });
in
pkgs.testers.runNixOSTest {
  name = "fishystuff-gitops-production-edge-handoff";

  nodes.machine =
    { ... }:
    {
      system.stateVersion = "25.11";
      networking.hostName = "production-single-host";
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
        desiredState
        edgeServiceBundleProductionGitopsHandoff
      ];
      environment.systemPackages = [
        fishystuffDeployPackage
        mgmtPackage
        pkgs.curl
        pkgs.jq
        pkgs.openssl
        pkgs.python3
      ];
    };

  testScript = ''
    import json
    import textwrap

    start_all()

    mgmt_log = "/tmp/fishystuff-gitops-production-edge-mgmt.log"
    mgmt_pid = "/tmp/fishystuff-gitops-production-edge-mgmt.pid"
    api_pid = "/tmp/fishystuff-gitops-production-edge-api.pid"
    caddy_pid = "/tmp/fishystuff-gitops-production-edge-caddy.pid"
    active = "/var/lib/fishystuff/gitops/active/production.json"
    route = "/run/fishystuff/gitops/routes/production.json"
    status = "/var/lib/fishystuff/gitops/status/production.json"
    admission = "/run/fishystuff/gitops/admission/production.json"
    rollback = "/var/lib/fishystuff/gitops/rollback/production.json"
    rollback_set = "/var/lib/fishystuff/gitops/rollback-set/production.json"
    active_roots = "/run/fishystuff/gitops/roots/production-${candidateReleaseId}.json"
    previous_roots = "/run/fishystuff/gitops/roots/production-${previousReleaseId}.json"
    caddy = "${edgeServiceBundleProductionGitopsHandoff}/artifacts/exe/main"
    caddyfile = "${edgeServiceBundleProductionGitopsHandoff}/artifacts/config/base"
    site_url = "https://fishystuff.fish"
    api_url = "https://api.fishystuff.fish"
    cdn_url = "https://cdn.fishystuff.fish"
    resolve = "--resolve fishystuff.fish:443:127.0.0.1 --resolve api.fishystuff.fish:443:127.0.0.1 --resolve cdn.fishystuff.fish:443:127.0.0.1"

    def dump_debug():
      _, output = machine.execute(f"echo '--- caddyfile ---'; cat {caddyfile}; echo '--- mgmt log tail ---'; tail -260 {mgmt_log} 2>/dev/null || true; echo '--- status ---'; cat {status} 2>/dev/null || true; echo '--- active ---'; cat {active} 2>/dev/null || true; echo '--- admission ---'; cat {admission} 2>/dev/null || true; echo '--- route ---'; cat {route} 2>/dev/null || true; echo '--- rollback ---'; cat {rollback} 2>/dev/null || true; echo '--- rollback set ---'; cat {rollback_set} 2>/dev/null || true; echo '--- roots ---'; cat {active_roots} 2>/dev/null || true; cat {previous_roots} 2>/dev/null || true; echo '--- caddy log ---'; cat /tmp/fishystuff-gitops-production-edge-caddy.log 2>/dev/null || true; echo '--- api log ---'; cat /tmp/fishystuff-gitops-production-edge-api.log 2>/dev/null || true; echo '--- tree ---'; find /var/lib/fishystuff/gitops /run/fishystuff/gitops -maxdepth 5 -ls 2>/dev/null || true")
      print(output)

    def wait_for(command, timeout=180):
      try:
        machine.wait_until_succeeds(command, timeout=timeout)
      except Exception:
        dump_debug()
        raise

    machine.succeed("test -x ${mgmtPackage}/bin/mgmt")
    machine.succeed("test -x /run/current-system/sw/bin/fishystuff_deploy")
    machine.succeed(f"test -x {caddy}")
    machine.succeed(f"grep -F 'root * /var/lib/fishystuff/gitops/served/production/site' {caddyfile}")
    machine.succeed(f"grep -F 'root * /var/lib/fishystuff/gitops/served/production/cdn' {caddyfile}")
    machine.succeed(f"grep -F 'reverse_proxy 127.0.0.1:18092' {caddyfile}")
    machine.succeed(f"! grep -F '/srv/fishystuff' {caddyfile}")
    machine.succeed(f"! grep -F 'beta.fishystuff.fish' {caddyfile}")
    machine.succeed("jq -e '.retained_roots == [\"${previousCdnRoot}\"]' ${candidateCdnServingRoot}/cdn-serving-manifest.json")
    machine.succeed("jq -e '.cluster == \"production\" and .mode == \"local-apply\" and .environments.production.serve == true and .environments.production.api_upstream == \"http://127.0.0.1:18092\"' ${desiredState}")

    meta = {
      "release_id": "${candidateReleaseId}",
      "release_identity": "${candidateReleaseIdentity}",
      "dolt_commit": "${candidateDoltCommit}",
    }
    machine.succeed("cat > /tmp/fishystuff-gitops-production-edge-api.py <<'PY'\n" + textwrap.dedent(f"""
      import json
      from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer

      META = {json.dumps(meta)!r}

      class Handler(BaseHTTPRequestHandler):
          def do_GET(self):
              if self.path == "/api/v1/meta":
                  body = META.encode("utf-8")
                  self.send_response(200)
                  self.send_header("Content-Type", "application/json")
                  self.send_header("Content-Length", str(len(body)))
                  self.end_headers()
                  self.wfile.write(body)
                  return
              self.send_response(404)
              self.end_headers()

          def log_message(self, fmt, *args):
              pass

      ThreadingHTTPServer(("127.0.0.1", 18092), Handler).serve_forever()
    """) + "\nPY")
    machine.succeed("python3 /tmp/fishystuff-gitops-production-edge-api.py >/tmp/fishystuff-gitops-production-edge-api.log 2>&1 & echo $! > /tmp/fishystuff-gitops-production-edge-api.pid")
    wait_for("curl -fsS http://127.0.0.1:18092/api/v1/meta | jq -e '.release_id == \"${candidateReleaseId}\"'")

    machine.succeed("env FISHYSTUFF_GITOPS_ENABLE_LOCAL_APPLY=1 FISHYSTUFF_GITOPS_STATE_FILE=${desiredState} ${mgmtPackage}/bin/mgmt run --hostname production-single-host --tmp-prefix --no-pgp --client-urls=http://127.0.0.1:2379 --server-urls=http://127.0.0.1:2380 --advertise-client-urls=http://127.0.0.1:2379 --advertise-server-urls=http://127.0.0.1:2380 --converged-timeout=-1 lang ${gitopsSrc}/main.mcl > /tmp/fishystuff-gitops-production-edge-mgmt.log 2>&1 & echo $! > /tmp/fishystuff-gitops-production-edge-mgmt.pid")
    machine.wait_for_file(active, timeout=240)
    machine.wait_for_file(route, timeout=240)
    machine.wait_for_file(status, timeout=240)
    machine.wait_for_file(admission, timeout=240)
    machine.wait_for_file(active_roots, timeout=240)
    machine.wait_for_file(previous_roots, timeout=240)
    wait_for("jq -e '.desired_generation == 12 and .release_id == \"${candidateReleaseId}\" and .release_identity == \"${candidateReleaseIdentity}\" and .served == true and .admission_state == \"passed_fixture\" and .api_upstream == \"http://127.0.0.1:18092\"' " + active)
    wait_for("jq -e '.desired_generation == 12 and .release_id == \"${candidateReleaseId}\" and .phase == \"served\" and .rollback_primary_release_id == \"${previousReleaseId}\" and .rollback_retained_count == 1 and .served == true' " + status)
    wait_for("jq -e '.release_id == \"${candidateReleaseId}\" and .probe == \"http-json-scalars\" and .url == \"http://127.0.0.1:18092/api/v1/meta\" and .admission_state == \"passed_fixture\"' " + admission)
    wait_for("jq -e '.release_id == \"${candidateReleaseId}\" and .site_root == \"/var/lib/fishystuff/gitops/served/production/site\" and .cdn_root == \"/var/lib/fishystuff/gitops/served/production/cdn\" and .api_upstream == \"http://127.0.0.1:18092\" and .state == \"selected_local_route\"' " + route)
    wait_for("jq -e '.rollback_release_id == \"${previousReleaseId}\" and .rollback_release_identity == \"${previousReleaseIdentity}\" and .rollback_available == true' " + rollback)
    wait_for("jq -e '.roots_ready == true and .release_id == \"${candidateReleaseId}\" and .state == \"roots_ready\"' " + active_roots)
    wait_for("jq -e '.roots_ready == true and .release_id == \"${previousReleaseId}\" and .state == \"roots_ready\"' " + previous_roots)
    machine.succeed("test \"$(readlink /var/lib/fishystuff/gitops/served/production/site)\" = \"${candidateSite}\"")
    machine.succeed("test \"$(readlink /var/lib/fishystuff/gitops/served/production/cdn)\" = \"${candidateCdnServingRoot}\"")
    machine.succeed("test ! -e /srv/fishystuff")

    machine.succeed("mkdir -p /tmp/fishystuff-gitops-edge-credentials")
    machine.succeed("openssl req -x509 -newkey rsa:2048 -nodes -keyout /tmp/fishystuff-gitops-edge-credentials/privkey.pem -out /tmp/fishystuff-gitops-edge-credentials/fullchain.pem -days 1 -subj '/CN=fishystuff.fish' -addext 'subjectAltName=DNS:fishystuff.fish,DNS:api.fishystuff.fish,DNS:cdn.fishystuff.fish,DNS:telemetry.fishystuff.fish' >/tmp/fishystuff-gitops-production-edge-openssl.log 2>&1")
    machine.succeed(f"env CREDENTIALS_DIRECTORY=/tmp/fishystuff-gitops-edge-credentials {caddy} run --config {caddyfile} --adapter caddyfile >/tmp/fishystuff-gitops-production-edge-caddy.log 2>&1 & echo $! > {caddy_pid}")
    wait_for(f"curl -kfsS {resolve} {site_url}/ | grep -Fx 'production edge candidate site'")
    wait_for(f"curl -kfsS {resolve} {cdn_url}/map/runtime-manifest.json | jq -e '.module == \"fishystuff_ui_bevy.production-edge-candidate.js\" and .wasm == \"fishystuff_ui_bevy_bg.production-edge-candidate.wasm\"'")
    machine.succeed(f"test \"$(curl -kfsS {resolve} {cdn_url}/map/fishystuff_ui_bevy.production-edge-candidate.js)\" = \"production edge candidate module\"")
    machine.succeed(f"test \"$(curl -kfsS {resolve} {cdn_url}/map/fishystuff_ui_bevy.production-edge-previous.js)\" = \"production edge previous module\"")
    machine.succeed(f"curl -kfsS {resolve} {api_url}/api/v1/meta | jq -e '.release_id == \"${candidateReleaseId}\" and .release_identity == \"${candidateReleaseIdentity}\" and .dolt_commit == \"${candidateDoltCommit}\"'")
    machine.succeed(f"curl -kfsSI {resolve} https://cdn.fishystuff.fish/map/fishystuff_ui_bevy.production-edge-candidate.js | grep -Fi 'cache-control: public, max-age=31536000, immutable'")
    machine.succeed(f"curl -kfsSI {resolve} https://cdn.fishystuff.fish/map/runtime-manifest.json | grep -Fi 'cache-control: no-store'")

    machine.succeed(f"kill $(cat {mgmt_pid}) || true")
    machine.succeed(f"kill $(cat {caddy_pid}) || true")
    machine.succeed(f"kill $(cat {api_pid}) || true")
    machine.fail("systemctl is-active fishystuff-api.service")
    machine.fail("systemctl is-active fishystuff-dolt.service")
    machine.fail("systemctl is-active fishystuff-edge.service")
    machine.succeed("! find /var/lib/fishystuff/gitops /run/fishystuff/gitops -type f -print0 | xargs -0 grep -E 'beta\\.fishystuff\\.fish|cloudflare|hcloud|ssh '")
  '';
}
