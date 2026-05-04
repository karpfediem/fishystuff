{
  pkgs,
  mgmtPackage,
  fishystuffDeployPackage,
  gitopsSrc,
}:
pkgs.testers.runNixOSTest {
  name = "fishystuff-gitops-dolt-fetch-pin";

  nodes.machine =
    { ... }:
    {
      system.stateVersion = "25.11";
      networking.hostName = "vm-single-host";
      virtualisation.memorySize = 8192;
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

    work = "/tmp/fishystuff-gitops-dolt-fetch-pin"
    desired = f"{work}/desired.json"
    mgmt_log = "/tmp/fishystuff-gitops-dolt-fetch-pin.log"
    mgmt_pid = "/tmp/fishystuff-gitops-dolt-fetch-pin.pid"
    dolt_status = "/run/fishystuff/gitops-test/dolt/local-test-example-release.json"
    status = "/var/lib/fishystuff/gitops-test/status/local-test.json"
    admission = "/run/fishystuff/gitops-test/admission/local-test.json"
    instance = "/var/lib/fishystuff/gitops-test/instances/local-test-example-release.json"
    rollback = "/var/lib/fishystuff/gitops-test/rollback/local-test.json"
    cache = "/var/lib/fishystuff/gitops-test/dolt-cache/fishystuff"
    release_ref = "fishystuff/gitops/example-release"

    def wait_for_gitops_file(path):
      try:
        machine.wait_for_file(path, timeout=120)
      except Exception:
        _, output = machine.execute(f"echo '--- mgmt log head ---'; head -120 {mgmt_log} 2>/dev/null || true; echo '--- mgmt log tail ---'; tail -200 {mgmt_log} 2>/dev/null || true; echo '--- gitops state tree ---'; find /var/lib/fishystuff/gitops-test /run/fishystuff/gitops-test -maxdepth 5 -ls 2>/dev/null || true; echo '--- mgmt process ---'; ps -ef | grep '[m]gmt' || true")
        print(output)
        raise

    def start_mgmt():
      machine.succeed(f"env FISHYSTUFF_GITOPS_STATE_FILE={desired} ${mgmtPackage}/bin/mgmt run --hostname vm-single-host --tmp-prefix --no-pgp --client-urls=http://127.0.0.1:2379 --server-urls=http://127.0.0.1:2380 --advertise-client-urls=http://127.0.0.1:2379 --advertise-server-urls=http://127.0.0.1:2380 --converged-timeout=-1 lang ${gitopsSrc}/main.mcl >{mgmt_log} 2>&1 & echo $! >{mgmt_pid}")

    def stop_mgmt():
      machine.succeed(f"kill $(cat {mgmt_pid}) || true")
      machine.wait_until_succeeds(f"! kill -0 $(cat {mgmt_pid}) 2>/dev/null", timeout=30)

    machine.succeed("test -x ${mgmtPackage}/bin/mgmt")
    machine.succeed("test -x ${fishystuffDeployPackage}/bin/fishystuff_deploy")
    machine.succeed("test -x /run/current-system/sw/bin/dolt")
    machine.succeed("test -x /run/current-system/sw/bin/fishystuff_deploy")
    machine.succeed(f"mkdir -p {work}/source {work}/remote {work}/home")
    machine.succeed(f"env HOME={work}/home dolt config --global --add versioncheck.disabled true || true")
    machine.succeed(f"env HOME={work}/home dolt config --global --add metrics.disabled true || true")
    machine.succeed(textwrap.dedent(f"""
      set -euo pipefail
      export HOME={work}/home
      cd {work}/source
      dolt init --name "FishyStuff GitOps Test" --email fishystuff-gitops@example.invalid
      dolt sql -q "create table t (pk int primary key, v varchar(20)); insert into t values (1, 'one');"
      dolt add t
      dolt commit -m commit-one
      dolt remote add origin file://{work}/remote
      dolt push origin main
      dolt sql -r csv -q "select dolt_hashof('main') as hash" | tail -n 1 > {work}/commit1
    """))
    machine.succeed(textwrap.dedent(f"""
      set -euo pipefail
      commit="$(cat {work}/commit1)"
      cat > {desired} <<EOF
      {{
        "cluster": "local-test",
        "generation": 1,
        "mode": "vm-test",
        "hosts": {{
          "vm-single-host": {{
            "enabled": true,
            "role": "single-site",
            "hostname": "vm-single-host"
          }}
        }},
        "releases": {{
          "example-release": {{
            "generation": 1,
            "git_rev": "dolt-fetch-pin-one",
            "dolt_commit": "$commit",
            "closures": {{
              "api": {{"enabled": false, "store_path": "", "gcroot_path": ""}},
              "site": {{"enabled": false, "store_path": "", "gcroot_path": ""}},
              "cdn_runtime": {{"enabled": false, "store_path": "", "gcroot_path": ""}},
              "dolt_service": {{"enabled": false, "store_path": "", "gcroot_path": ""}}
            }},
            "dolt": {{
              "repository": "fishystuff/fishystuff",
              "commit": "$commit",
              "branch_context": "main",
              "mode": "read_only",
              "materialization": "fetch_pin",
              "remote_url": "file://{work}/remote",
              "cache_dir": "{cache}",
              "release_ref": "{release_ref}"
            }}
          }}
        }},
        "environments": {{
          "local-test": {{
            "enabled": true,
            "strategy": "single_active",
            "host": "vm-single-host",
            "active_release": "example-release",
            "retained_releases": [],
            "serve": false
          }}
        }}
      }}
      EOF
    """))

    start_mgmt()

    wait_for_gitops_file(dolt_status)
    wait_for_gitops_file(status)
    wait_for_gitops_file(admission)
    wait_for_gitops_file(instance)
    machine.wait_until_succeeds(f"jq -e --arg commit \"$(cat {work}/commit1)\" '.state == \"pinned\" and .requested_commit == $commit and .verified_commit == $commit and .materialization == \"fetch_pin\" and .cache_dir == \"{cache}\" and .release_ref == \"{release_ref}\"' {dolt_status}")
    machine.wait_until_succeeds(f"jq -e --arg commit \"$(cat {work}/commit1)\" '.desired_generation == 1 and .dolt_commit == $commit and .dolt_materialization == \"fetch_pin\" and .dolt_cache_dir == \"{cache}\" and .dolt_release_ref == \"{release_ref}\"' {status}")
    machine.succeed(f"test ! -e {rollback}")
    machine.succeed(f"cd {cache} && test \"$(dolt sql -r csv -q \"select dolt_hashof('{release_ref}') as hash\" | tail -n 1)\" = \"$(cat {work}/commit1)\"")
    machine.succeed(f"test ! -e {cache}/.fishystuff-dolt-snapshot-source")
    machine.succeed(f"touch {cache}/cache-survives-fetch")
    stop_mgmt()

    machine.succeed(textwrap.dedent(f"""
      set -euo pipefail
      export HOME={work}/home
      cd {work}/source
      dolt sql -q "insert into t values (2, 'two');"
      dolt add t
      dolt commit -m commit-two
      dolt push origin main
      dolt sql -r csv -q "select dolt_hashof('main') as hash" | tail -n 1 > {work}/commit2
    """))
    machine.succeed(textwrap.dedent(f"""
      set -euo pipefail
      commit="$(cat {work}/commit2)"
      cat > {desired} <<EOF
      {{
        "cluster": "local-test",
        "generation": 2,
        "mode": "vm-test",
        "hosts": {{
          "vm-single-host": {{
            "enabled": true,
            "role": "single-site",
            "hostname": "vm-single-host"
          }}
        }},
        "releases": {{
          "example-release": {{
            "generation": 2,
            "git_rev": "dolt-fetch-pin-two",
            "dolt_commit": "$commit",
            "closures": {{
              "api": {{"enabled": false, "store_path": "", "gcroot_path": ""}},
              "site": {{"enabled": false, "store_path": "", "gcroot_path": ""}},
              "cdn_runtime": {{"enabled": false, "store_path": "", "gcroot_path": ""}},
              "dolt_service": {{"enabled": false, "store_path": "", "gcroot_path": ""}}
            }},
            "dolt": {{
              "repository": "fishystuff/fishystuff",
              "commit": "$commit",
              "branch_context": "main",
              "mode": "read_only",
              "materialization": "fetch_pin",
              "remote_url": "file://{work}/remote",
              "cache_dir": "{cache}",
              "release_ref": "{release_ref}"
            }}
          }}
        }},
        "environments": {{
          "local-test": {{
            "enabled": true,
            "strategy": "single_active",
            "host": "vm-single-host",
            "active_release": "example-release",
            "retained_releases": [],
            "serve": false
          }}
        }}
      }}
      EOF
    """))

    start_mgmt()
    machine.wait_until_succeeds(f"jq -e --arg commit \"$(cat {work}/commit2)\" '.state == \"pinned\" and .verified_commit == $commit' {dolt_status}")
    machine.wait_until_succeeds(f"jq -e --arg commit \"$(cat {work}/commit2)\" '.desired_generation == 2 and .dolt_commit == $commit' {status}")
    machine.succeed(f"test ! -e {rollback}")
    machine.succeed(f"cd {cache} && test \"$(dolt sql -r csv -q \"select dolt_hashof('{release_ref}') as hash\" | tail -n 1)\" = \"$(cat {work}/commit2)\"")
    machine.succeed(f"test -f {cache}/cache-survives-fetch")
    stop_mgmt()

    machine.fail("systemctl is-active fishystuff-api.service")
    machine.fail("systemctl is-active fishystuff-dolt.service")
    machine.fail("systemctl is-active fishystuff-edge.service")
    machine.succeed("test ! -e /srv/fishystuff")
    machine.succeed("test ! -e /var/lib/fishystuff/mgmt")
    machine.succeed("! find /var/lib/fishystuff/gitops-test /run/fishystuff/gitops-test -type f -print0 | xargs -0 grep -E 'beta\\.fishystuff\\.fish|production|cloudflare|hcloud|ssh '")
  '';
}
