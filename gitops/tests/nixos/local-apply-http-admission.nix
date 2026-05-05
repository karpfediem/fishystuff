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
  candidateApiServer = pkgs.writeText "fishystuff-gitops-candidate-api-server.py" ''
    import argparse
    import json
    import os
    import sys
    from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer

    parser = argparse.ArgumentParser()
    parser.add_argument("--bind", required=True)
    args = parser.parse_args()

    host, port_text = args.bind.rsplit(":", 1)
    port = int(port_text)

    required_env = [
        "FISHYSTUFF_RELEASE_ID",
        "FISHYSTUFF_RELEASE_IDENTITY",
        "FISHYSTUFF_DOLT_COMMIT",
        "FISHYSTUFF_DEPLOYMENT_ENVIRONMENT",
    ]
    missing = [name for name in required_env if os.environ.get(name) is None]
    if missing:
        sys.stderr.write("missing deployment identity env vars: " + ", ".join(missing) + "\n")
        sys.exit(1)

    class Handler(BaseHTTPRequestHandler):
        def do_GET(self):
            if self.path != "/api/v1/meta":
                self.send_response(404)
                self.end_headers()
                return

            body = json.dumps({
                "state": "ok",
                "release_id": os.environ["FISHYSTUFF_RELEASE_ID"],
                "release_identity": os.environ["FISHYSTUFF_RELEASE_IDENTITY"],
                "dolt_commit": os.environ["FISHYSTUFF_DOLT_COMMIT"],
                "deployment_environment": os.environ["FISHYSTUFF_DEPLOYMENT_ENVIRONMENT"],
            }).encode("utf-8")
            self.send_response(200)
            self.send_header("content-type", "application/json")
            self.send_header("content-length", str(len(body)))
            self.end_headers()
            self.wfile.write(body)

        def log_message(self, format, *args):
            return

    ThreadingHTTPServer((host, port), Handler).serve_forever()
  '';
  candidateApiPackage = pkgs.writeShellApplication {
    name = "fishystuff_server";
    runtimeInputs = [ pkgs.python3 ];
    text = ''
      exec python3 ${candidateApiServer} "$@"
    '';
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
      api_service = "fishystuff-gitops-candidate-api-local-test";
      admission_probe = {
        kind = "api_meta";
        probe_name = "api-meta";
        url = "http://127.0.0.1:18082/api/v1/meta";
        expected_status = 200;
        timeout_ms = 2000;
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
      ];
      systemd.services.fishystuff-gitops-candidate-api-local-test = {
        serviceConfig = {
          EnvironmentFile = "/var/lib/fishystuff/gitops/api/local-test.env";
          ExecStart = "${candidateApiPackage}/bin/fishystuff_server --bind 127.0.0.1:18082";
          Restart = "always";
          RestartSec = "1s";
        };
      };
    };

  testScript = ''
    start_all()

    mgmt_log = "/tmp/fishystuff-gitops-local-apply-http-admission.log"
    mgmt_pid = "/tmp/fishystuff-gitops-local-apply-http-admission.pid"
    api_config = "/var/lib/fishystuff/gitops/api/local-test.json"
    api_env = "/var/lib/fishystuff/gitops/api/local-test.env"
    status = "/var/lib/fishystuff/gitops/status/local-test.json"
    active = "/var/lib/fishystuff/gitops/active/local-test.json"
    route = "/run/fishystuff/gitops/routes/local-test.json"
    admission = "/run/fishystuff/gitops/admission/local-test.json"
    request = "/var/lib/fishystuff/gitops/admission/requests/local-test-local-apply-http-release-http-api-meta.json"
    instance = "/var/lib/fishystuff/gitops/instances/local-test-local-apply-http-release.json"
    rollback_set = "/var/lib/fishystuff/gitops/rollback-set/local-test.json"
    candidate_service = "fishystuff-gitops-candidate-api-local-test"

    def dump_gitops_debug():
      _, output = machine.execute(f"echo '--- mgmt log head ---'; head -120 {mgmt_log} 2>/dev/null || true; echo '--- mgmt log tail ---'; tail -240 {mgmt_log} 2>/dev/null || true; echo '--- api config ---'; cat {api_config} 2>/dev/null || true; echo '--- api env ---'; cat {api_env} 2>/dev/null || true; echo '--- admission request ---'; cat {request} 2>/dev/null || true; echo '--- candidate api status ---'; systemctl status {candidate_service} --no-pager 2>&1 || true; echo '--- candidate api journal ---'; journalctl -u {candidate_service} --no-pager -n 120 2>&1 || true; echo '--- probe curl ---'; curl -v http://127.0.0.1:18082/api/v1/meta 2>&1 || true; echo '--- gitops state tree ---'; find /var/lib/fishystuff/gitops /run/fishystuff/gitops -maxdepth 6 -ls 2>/dev/null || true; echo '--- mgmt process ---'; ps -ef | grep '[m]gmt' || true")
      print(output)

    def wait_for_gitops_file(path):
      try:
        machine.wait_for_file(path, timeout=180)
      except Exception:
        dump_gitops_debug()
        raise

    def wait_for_gitops_command(command, timeout=60):
      try:
        machine.wait_until_succeeds(command, timeout=timeout)
      except Exception:
        dump_gitops_debug()
        raise

    machine.succeed("test -x ${mgmtPackage}/bin/mgmt")
    machine.succeed("test -x ${fishystuffDeployPackage}/bin/fishystuff_deploy")
    machine.succeed("test -x /run/current-system/sw/bin/fishystuff_deploy")
    machine.succeed("systemctl cat fishystuff-gitops-candidate-api-local-test | grep -Fx 'EnvironmentFile=/var/lib/fishystuff/gitops/api/local-test.env'")
    machine.succeed("systemctl show fishystuff-gitops-candidate-api-local-test -p ExecStart --value | grep -F '/bin/fishystuff_server --bind 127.0.0.1:18082'")
    machine.succeed("jq -e '.mode == \"local-apply\" and .environments.\"local-test\".serve == true and .environments.\"local-test\".api_upstream == \"http://127.0.0.1:18082\" and .environments.\"local-test\".api_service == \"fishystuff-gitops-candidate-api-local-test\" and .environments.\"local-test\".admission_probe.kind == \"api_meta\"' ${desiredState}")
    machine.fail("systemctl is-active fishystuff-gitops-candidate-api-local-test")
    machine.succeed(f"env FISHYSTUFF_GITOPS_ENABLE_LOCAL_APPLY=1 FISHYSTUFF_GITOPS_STATE_FILE=${desiredState} ${mgmtPackage}/bin/mgmt run --hostname vm-single-host --tmp-prefix --no-pgp --client-urls=http://127.0.0.1:2379 --server-urls=http://127.0.0.1:2380 --advertise-client-urls=http://127.0.0.1:2379 --advertise-server-urls=http://127.0.0.1:2380 --converged-timeout=-1 lang ${gitopsSrc}/main.mcl >{mgmt_log} 2>&1 & echo $! >{mgmt_pid}")

    wait_for_gitops_file(api_config)
    wait_for_gitops_file(api_env)
    wait_for_gitops_command("systemctl is-active fishystuff-gitops-candidate-api-local-test")
    wait_for_gitops_command("curl -fsS http://127.0.0.1:18082/api/v1/meta | jq -e '.release_id == \"local-apply-http-release\" and .release_identity == \"${expectedReleaseIdentity}\" and .dolt_commit == \"local-apply-http-admission\" and .deployment_environment == \"local-test\"'")
    wait_for_gitops_file(request)
    wait_for_gitops_file(admission)
    wait_for_gitops_file(status)
    wait_for_gitops_file(active)
    wait_for_gitops_file(route)
    wait_for_gitops_file(instance)
    wait_for_gitops_file(rollback_set)
    machine.succeed(f"jq -e '.desired_generation == 62 and .environment == \"local-test\" and .host == \"vm-single-host\" and .release_id == \"local-apply-http-release\" and .release_identity == \"${expectedReleaseIdentity}\" and .api_bundle == \"${apiArtifact}\" and .api_upstream == \"http://127.0.0.1:18082\" and .service_name == \"fishystuff-gitops-candidate-api-local-test\" and .dolt_commit == \"local-apply-http-admission\" and .state == \"candidate_api_config\"' {api_config}")
    machine.succeed(f"grep -Fx \"FISHYSTUFF_RELEASE_ID='local-apply-http-release'\" {api_env}")
    machine.succeed(f"grep -Fx \"FISHYSTUFF_RELEASE_IDENTITY='${expectedReleaseIdentity}'\" {api_env}")
    machine.succeed(f"grep -Fx \"FISHYSTUFF_DOLT_COMMIT='local-apply-http-admission'\" {api_env}")
    machine.succeed(f"grep -Fx \"FISHYSTUFF_DEPLOYMENT_ENVIRONMENT='local-test'\" {api_env}")
    machine.succeed("pid=$(systemctl show fishystuff-gitops-candidate-api-local-test -p MainPID --value); tr '\\0' '\\n' < /proc/$pid/environ | grep -Fx 'FISHYSTUFF_RELEASE_ID=local-apply-http-release'")
    machine.succeed("pid=$(systemctl show fishystuff-gitops-candidate-api-local-test -p MainPID --value); tr '\\0' '\\n' < /proc/$pid/environ | grep -Fx 'FISHYSTUFF_DOLT_COMMIT=local-apply-http-admission'")
    machine.succeed(f"jq -e '.environment == \"local-test\" and .host == \"vm-single-host\" and .release_id == \"local-apply-http-release\" and .probe_name == \"api-meta\" and .url == \"http://127.0.0.1:18082/api/v1/meta\" and .expected_status == 200 and .timeout_ms == 2000 and .expected_scalars.\"/release_id\" == \"local-apply-http-release\" and .expected_scalars.\"/release_identity\" == \"${expectedReleaseIdentity}\" and .expected_scalars.\"/dolt_commit\" == \"local-apply-http-admission\"' {request}")
    machine.succeed(f"jq -e '.environment == \"local-test\" and .host == \"vm-single-host\" and .release_id == \"local-apply-http-release\" and .release_identity == \"${expectedReleaseIdentity}\" and .probe_name == \"api-meta\" and .url == \"http://127.0.0.1:18082/api/v1/meta\" and .expected_status == 200 and .observed_status == 200 and .expected_scalars.\"/release_id\" == \"local-apply-http-release\" and .expected_scalars.\"/release_identity\" == \"${expectedReleaseIdentity}\" and .expected_scalars.\"/dolt_commit\" == \"local-apply-http-admission\" and .scalars.\"/release_id\" == \"local-apply-http-release\" and .scalars.\"/release_identity\" == \"${expectedReleaseIdentity}\" and .scalars.\"/dolt_commit\" == \"local-apply-http-admission\" and .admission_state == \"passed_fixture\" and .probe == \"http-json-scalars\"' {admission}")
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

    machine.fail("systemctl is-active fishystuff-api.service")
    machine.fail("systemctl is-active fishystuff-dolt.service")
    machine.fail("systemctl is-active fishystuff-edge.service")
    machine.succeed("test ! -e /srv/fishystuff")
    machine.succeed("test ! -e /var/lib/fishystuff/mgmt")
    machine.succeed("! find /var/lib/fishystuff/gitops /run/fishystuff/gitops -type f -print0 | xargs -0 grep -E 'beta\\.fishystuff\\.fish|production|cloudflare|hcloud|ssh '")
  '';
}
