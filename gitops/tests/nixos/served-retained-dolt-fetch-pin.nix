{
  pkgs,
  mgmtPackage,
  fishystuffDeployPackage,
  gitopsSrc,
}:
let
  previousApi = pkgs.writeText "fishystuff-gitops-retained-dolt-previous-api" "previous api\n";
  candidateApi = pkgs.writeText "fishystuff-gitops-retained-dolt-candidate-api" "candidate api\n";
  previousDoltService = pkgs.writeText "fishystuff-gitops-retained-dolt-previous-dolt-service" "previous dolt service\n";
  candidateDoltService = pkgs.writeText "fishystuff-gitops-retained-dolt-candidate-dolt-service" "candidate dolt service\n";
  previousSite = pkgs.runCommand "fishystuff-gitops-retained-dolt-previous-site" { } ''
    mkdir -p "$out"
    printf 'previous retained Dolt site\n' > "$out/index.html"
  '';
  candidateSite = pkgs.runCommand "fishystuff-gitops-retained-dolt-candidate-site" { } ''
    mkdir -p "$out"
    printf 'candidate retained Dolt site\n' > "$out/index.html"
  '';
  previousCdnRoot = pkgs.runCommand "fishystuff-gitops-retained-dolt-previous-cdn-current" { } ''
    mkdir -p "$out/map"
    printf '{"module":"fishystuff_ui_bevy.previous.js","wasm":"fishystuff_ui_bevy_bg.previous.wasm"}\n' > "$out/map/runtime-manifest.json"
    printf 'previous retained Dolt module\n' > "$out/map/fishystuff_ui_bevy.previous.js"
    printf 'previous retained Dolt wasm\n' > "$out/map/fishystuff_ui_bevy_bg.previous.wasm"
  '';
  candidateCdnRoot = pkgs.runCommand "fishystuff-gitops-retained-dolt-candidate-cdn-current" { } ''
    mkdir -p "$out/map"
    printf '{"module":"fishystuff_ui_bevy.candidate.js","wasm":"fishystuff_ui_bevy_bg.candidate.wasm"}\n' > "$out/map/runtime-manifest.json"
    printf 'candidate retained Dolt module\n' > "$out/map/fishystuff_ui_bevy.candidate.js"
    printf 'candidate retained Dolt wasm\n' > "$out/map/fishystuff_ui_bevy_bg.candidate.wasm"
  '';
  candidateCdnServingRoot = pkgs.callPackage ../../../nix/packages/cdn-serving-root.nix {
    currentRoot = candidateCdnRoot;
    previousRoots = [ previousCdnRoot ];
  };
in
pkgs.testers.runNixOSTest {
  name = "fishystuff-gitops-served-retained-dolt-fetch-pin";

  nodes.machine =
    { ... }:
    {
      system.stateVersion = "25.11";
      networking.hostName = "vm-single-host";
      virtualisation.memorySize = 8192;
      virtualisation.additionalPaths = [
        previousApi
        candidateApi
        previousDoltService
        candidateDoltService
        previousSite
        candidateSite
        previousCdnRoot
        candidateCdnRoot
        candidateCdnServingRoot
      ];
      environment.systemPackages = [
        fishystuffDeployPackage
        mgmtPackage
        pkgs.dolt
        pkgs.jq
      ];
    };

  testScript = ''
    import textwrap

    start_all()

    work = "/tmp/fishystuff-gitops-served-retained-dolt-fetch-pin"
    desired = f"{work}/desired.json"
    mgmt_log = "/tmp/fishystuff-gitops-served-retained-dolt-fetch-pin.log"
    mgmt_pid = "/tmp/fishystuff-gitops-served-retained-dolt-fetch-pin.pid"
    cache = "/var/lib/fishystuff/gitops-test/dolt-cache/fishystuff"
    candidate_ref = "fishystuff/gitops/candidate-release"
    previous_ref = "fishystuff/gitops/previous-release"
    candidate_dolt_status = "/run/fishystuff/gitops-test/dolt/local-test-candidate-release.json"
    previous_dolt_status = "/run/fishystuff/gitops-test/dolt/local-test-previous-release.json"
    active = "/var/lib/fishystuff/gitops-test/active/local-test.json"
    status = "/var/lib/fishystuff/gitops-test/status/local-test.json"
    rollback = "/var/lib/fishystuff/gitops-test/rollback/local-test.json"
    route = "/run/fishystuff/gitops-test/routes/local-test.json"

    def wait_for_gitops_file(path):
      try:
        machine.wait_for_file(path, timeout=120)
      except Exception:
        _, output = machine.execute(f"echo '--- mgmt log head ---'; head -120 {mgmt_log} 2>/dev/null || true; echo '--- mgmt log tail ---'; tail -240 {mgmt_log} 2>/dev/null || true; echo '--- gitops state tree ---'; find /var/lib/fishystuff/gitops-test /run/fishystuff/gitops-test -maxdepth 6 -ls 2>/dev/null || true; echo '--- mgmt process ---'; ps -ef | grep '[m]gmt' || true")
        print(output)
        raise

    machine.succeed("test -x ${mgmtPackage}/bin/mgmt")
    machine.succeed("test -x ${fishystuffDeployPackage}/bin/fishystuff_deploy")
    machine.succeed("test -x /run/current-system/sw/bin/dolt")
    machine.succeed(f"mkdir -p {work}/source {work}/remote {work}/home")
    machine.succeed(f"env HOME={work}/home dolt config --global --add versioncheck.disabled true || true")
    machine.succeed(f"env HOME={work}/home dolt config --global --add metrics.disabled true || true")
    machine.succeed(textwrap.dedent(f"""
      set -euo pipefail
      export HOME={work}/home
      cd {work}/source
      dolt init --name "FishyStuff GitOps Test" --email fishystuff-gitops@example.invalid
      dolt sql -q "create table t (pk int primary key, v varchar(20)); insert into t values (1, 'previous');"
      dolt add t
      dolt commit -m previous-release
      dolt remote add origin file://{work}/remote
      dolt push origin main
      dolt sql -r csv -q "select dolt_hashof('main') as hash" | tail -n 1 > {work}/commit1
      dolt sql -q "insert into t values (2, 'candidate');"
      dolt add t
      dolt commit -m candidate-release
      dolt push origin main
      dolt sql -r csv -q "select dolt_hashof('main') as hash" | tail -n 1 > {work}/commit2
    """))
    machine.succeed(textwrap.dedent(f"""
      set -euo pipefail
      previous_commit="$(cat {work}/commit1)"
      candidate_commit="$(cat {work}/commit2)"
      cat > {desired} <<EOF
      {{
        "cluster": "local-test",
        "generation": 50,
        "mode": "vm-test",
        "hosts": {{
          "vm-single-host": {{
            "enabled": true,
            "role": "single-site",
            "hostname": "vm-single-host"
          }}
        }},
        "releases": {{
          "previous-release": {{
            "generation": 49,
            "git_rev": "previous-retained-dolt-fetch-pin",
            "dolt_commit": "$previous_commit",
            "closures": {{
              "api": {{"enabled": false, "store_path": "${previousApi}", "gcroot_path": ""}},
              "site": {{"enabled": false, "store_path": "${previousSite}", "gcroot_path": ""}},
              "cdn_runtime": {{"enabled": false, "store_path": "${previousCdnRoot}", "gcroot_path": ""}},
              "dolt_service": {{"enabled": false, "store_path": "${previousDoltService}", "gcroot_path": ""}}
            }},
            "dolt": {{
              "repository": "fishystuff/fishystuff",
              "commit": "$previous_commit",
              "branch_context": "main",
              "mode": "read_only",
              "materialization": "fetch_pin",
              "remote_url": "file://{work}/remote",
              "cache_dir": "{cache}",
              "release_ref": "{previous_ref}"
            }}
          }},
          "candidate-release": {{
            "generation": 50,
            "git_rev": "candidate-retained-dolt-fetch-pin",
            "dolt_commit": "$candidate_commit",
            "closures": {{
              "api": {{"enabled": false, "store_path": "${candidateApi}", "gcroot_path": ""}},
              "site": {{"enabled": false, "store_path": "${candidateSite}", "gcroot_path": ""}},
              "cdn_runtime": {{"enabled": false, "store_path": "${candidateCdnServingRoot}", "gcroot_path": ""}},
              "dolt_service": {{"enabled": false, "store_path": "${candidateDoltService}", "gcroot_path": ""}}
            }},
            "dolt": {{
              "repository": "fishystuff/fishystuff",
              "commit": "$candidate_commit",
              "branch_context": "main",
              "mode": "read_only",
              "materialization": "fetch_pin",
              "remote_url": "file://{work}/remote",
              "cache_dir": "{cache}",
              "release_ref": "{candidate_ref}"
            }}
          }}
        }},
        "environments": {{
          "local-test": {{
            "enabled": true,
            "strategy": "single_active",
            "host": "vm-single-host",
            "active_release": "candidate-release",
            "retained_releases": ["previous-release"],
            "serve": true
          }}
        }}
      }}
      EOF
    """))

    machine.succeed(f"env FISHYSTUFF_GITOPS_STATE_FILE={desired} ${mgmtPackage}/bin/mgmt run --hostname vm-single-host --tmp-prefix --no-pgp --client-urls=http://127.0.0.1:2379 --server-urls=http://127.0.0.1:2380 --advertise-client-urls=http://127.0.0.1:2379 --advertise-server-urls=http://127.0.0.1:2380 --converged-timeout=-1 lang ${gitopsSrc}/main.mcl >{mgmt_log} 2>&1 & echo $! >{mgmt_pid}")

    wait_for_gitops_file(candidate_dolt_status)
    wait_for_gitops_file(previous_dolt_status)
    wait_for_gitops_file(active)
    wait_for_gitops_file(status)
    wait_for_gitops_file(rollback)
    wait_for_gitops_file(route)
    machine.wait_until_succeeds(f"jq -e --arg commit \"$(cat {work}/commit2)\" '.release_id == \"candidate-release\" and .requested_commit == $commit and .verified_commit == $commit and .cache_dir == \"{cache}\" and .release_ref == \"{candidate_ref}\" and .state == \"pinned\"' {candidate_dolt_status}")
    machine.wait_until_succeeds(f"jq -e --arg commit \"$(cat {work}/commit1)\" '.release_id == \"previous-release\" and .requested_commit == $commit and .verified_commit == $commit and .cache_dir == \"{cache}\" and .release_ref == \"{previous_ref}\" and .state == \"pinned\"' {previous_dolt_status}")
    machine.wait_until_succeeds(f"jq -e '.desired_generation == 50 and .release_id == \"candidate-release\" and .retained_release_ids == [\"previous-release\"] and .served == true' {active}")
    machine.wait_until_succeeds(f"jq -e '.desired_generation == 50 and .release_id == \"candidate-release\" and .phase == \"served\" and .served == true and .retained_release_ids == [\"previous-release\"]' {status}")
    machine.wait_until_succeeds(f"jq -e --arg commit \"$(cat {work}/commit1)\" '.desired_generation == 50 and .current_release_id == \"candidate-release\" and .rollback_release_id == \"previous-release\" and .rollback_dolt_commit == $commit and .rollback_dolt_materialization == \"fetch_pin\" and .rollback_dolt_cache_dir == \"{cache}\" and .rollback_dolt_release_ref == \"{previous_ref}\" and .rollback_available == true' {rollback}")
    machine.wait_until_succeeds(f"jq -e '.desired_generation == 50 and .release_id == \"candidate-release\" and .state == \"selected_local_route\"' {route}")
    machine.succeed(f"cd {cache} && test \"$(dolt sql -r csv -q \"select dolt_hashof('{candidate_ref}') as hash\" | tail -n 1)\" = \"$(cat {work}/commit2)\"")
    machine.succeed(f"cd {cache} && test \"$(dolt sql -r csv -q \"select dolt_hashof('{previous_ref}') as hash\" | tail -n 1)\" = \"$(cat {work}/commit1)\"")
    machine.succeed("test \"$(readlink /var/lib/fishystuff/gitops-test/served/local-test/site)\" = \"${candidateSite}\"")
    machine.succeed("test \"$(readlink /var/lib/fishystuff/gitops-test/served/local-test/cdn)\" = \"${candidateCdnServingRoot}\"")
    machine.succeed(f"kill $(cat {mgmt_pid}) || true")

    machine.fail("systemctl is-active fishystuff-api.service")
    machine.fail("systemctl is-active fishystuff-dolt.service")
    machine.fail("systemctl is-active fishystuff-edge.service")
    machine.succeed("test ! -e /srv/fishystuff")
    machine.succeed("test ! -e /var/lib/fishystuff/mgmt")
    machine.succeed("! find /var/lib/fishystuff/gitops-test /run/fishystuff/gitops-test -type f -print0 | xargs -0 grep -E 'beta\\.fishystuff\\.fish|production|cloudflare|hcloud|ssh '")
  '';
}
