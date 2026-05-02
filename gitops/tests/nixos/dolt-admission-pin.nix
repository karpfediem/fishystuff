{
  pkgs,
  mgmtPackage,
  fishystuffDeployPackage,
  gitopsSrc,
}:
pkgs.testers.runNixOSTest {
  name = "fishystuff-gitops-dolt-admission-pin";

  nodes.machine =
    { ... }:
    {
      system.stateVersion = "25.11";
      networking.hostName = "vm-single-host";
      virtualisation.memorySize = 4096;
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

    work = "/tmp/fishystuff-gitops-dolt-admission-pin"
    desired = f"{work}/desired.json"
    mgmt_log = "/tmp/fishystuff-gitops-dolt-admission-pin.log"
    mgmt_pid = "/tmp/fishystuff-gitops-dolt-admission-pin.pid"
    dolt_status = "/run/fishystuff/gitops-test/dolt/local-test-example-release.json"
    admission = "/run/fishystuff/gitops-test/admission/local-test.json"
    status = "/var/lib/fishystuff/gitops-test/status/local-test.json"
    cache = "/var/lib/fishystuff/gitops-test/dolt-cache/fishystuff"
    release_ref = "fishystuff/gitops/example-release"

    def wait_for_gitops_file(path):
      try:
        machine.wait_for_file(path, timeout=120)
      except Exception:
        _, output = machine.execute(f"echo '--- mgmt log head ---'; head -120 {mgmt_log} 2>/dev/null || true; echo '--- mgmt log tail ---'; tail -240 {mgmt_log} 2>/dev/null || true; echo '--- gitops state tree ---'; find /var/lib/fishystuff/gitops-test /run/fishystuff/gitops-test -maxdepth 6 -ls 2>/dev/null || true; echo '--- mgmt process ---'; ps -ef | grep '[m]gmt' || true")
        print(output)
        raise

    def start_mgmt():
      machine.succeed(f"env FISHYSTUFF_GITOPS_STATE_FILE={desired} ${mgmtPackage}/bin/mgmt run --hostname vm-single-host --tmp-prefix --no-pgp --client-urls=http://127.0.0.1:2379 --server-urls=http://127.0.0.1:2380 --advertise-client-urls=http://127.0.0.1:2379 --advertise-server-urls=http://127.0.0.1:2380 lang ${gitopsSrc}/main.mcl >{mgmt_log} 2>&1 & echo $! >{mgmt_pid}")

    def stop_mgmt():
      machine.succeed(f"kill $(cat {mgmt_pid}) || true")
      machine.wait_until_succeeds(f"! kill -0 $(cat {mgmt_pid}) 2>/dev/null", timeout=30)

    def write_desired(generation, git_rev, commit_file, pk, expected_value):
      machine.succeed(textwrap.dedent(f"""
        set -euo pipefail
        commit="$(cat {commit_file})"
        cat > {desired} <<EOF
        {{
          "cluster": "local-test",
          "generation": {generation},
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
              "generation": {generation},
              "git_rev": "{git_rev}",
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
              "serve": false,
              "admission_probe": {{
                "kind": "dolt_sql_fixture",
                "sql": "select v from t as of '{release_ref}' where pk = {pk}",
                "expected_scalar": "{expected_value}"
              }}
            }}
          }}
        }}
        EOF
      """))

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

    write_desired(1, "dolt-admission-one", f"{work}/commit1", 1, "one")
    start_mgmt()
    wait_for_gitops_file(dolt_status)
    wait_for_gitops_file(admission)
    wait_for_gitops_file(status)
    machine.wait_until_succeeds(f"jq -e --arg commit \"$(cat {work}/commit1)\" '.state == \"pinned\" and .verified_commit == $commit' {dolt_status}")
    machine.wait_until_succeeds(f"jq -e --arg commit \"$(cat {work}/commit1)\" '.admission_state == \"passed_fixture\" and .probe == \"dolt-sql-fixture\" and .dolt_commit == $commit and .dolt_verified_commit == $commit and .probe_value == \"one\" and .expected_scalar == \"one\"' {admission}")
    machine.wait_until_succeeds(f"jq -e --arg commit \"$(cat {work}/commit1)\" '.desired_generation == 1 and .admission_state == \"passed_fixture\" and .dolt_commit == $commit' {status}")
    machine.succeed(f"touch {cache}/cache-survives-admission")
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

    write_desired(2, "dolt-admission-two", f"{work}/commit2", 2, "two")
    start_mgmt()
    machine.wait_until_succeeds(f"jq -e --arg commit \"$(cat {work}/commit2)\" '.state == \"pinned\" and .verified_commit == $commit' {dolt_status}")
    machine.wait_until_succeeds(f"jq -e --arg commit \"$(cat {work}/commit2)\" '.admission_state == \"passed_fixture\" and .probe == \"dolt-sql-fixture\" and .dolt_commit == $commit and .dolt_verified_commit == $commit and .probe_value == \"two\" and .expected_scalar == \"two\"' {admission}")
    machine.wait_until_succeeds(f"jq -e --arg commit \"$(cat {work}/commit2)\" '.desired_generation == 2 and .admission_state == \"passed_fixture\" and .dolt_commit == $commit' {status}")
    machine.succeed(f"test -f {cache}/cache-survives-admission")
    stop_mgmt()

    machine.fail("systemctl is-active fishystuff-api.service")
    machine.fail("systemctl is-active fishystuff-dolt.service")
    machine.fail("systemctl is-active fishystuff-edge.service")
    machine.succeed("test ! -e /srv/fishystuff")
    machine.succeed("test ! -e /var/lib/fishystuff/mgmt")
    machine.succeed("! find /var/lib/fishystuff/gitops-test /run/fishystuff/gitops-test -type f -print0 | xargs -0 grep -E 'beta\\.fishystuff\\.fish|production|cloudflare|hcloud|ssh '")
  '';
}
