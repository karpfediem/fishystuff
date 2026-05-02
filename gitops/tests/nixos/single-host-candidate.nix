{
  pkgs,
  mgmtPackage,
  gitopsSrc,
}:
pkgs.testers.runNixOSTest {
  name = "fishystuff-gitops-single-host-candidate";

  nodes.machine =
    { ... }:
    {
      system.stateVersion = "25.11";
      networking.hostName = "vm-single-host";
      environment.systemPackages = [
        mgmtPackage
        pkgs.jq
      ];
    };

  testScript = ''
    start_all()

    machine.succeed("test -x ${mgmtPackage}/bin/mgmt")
    machine.succeed("env FISHYSTUFF_GITOPS_STATE_FILE=${gitopsSrc}/fixtures/vm-single-host.example.desired.json ${mgmtPackage}/bin/mgmt run --hostname vm-single-host --tmp-prefix --no-pgp --client-urls=http://127.0.0.1:2379 --server-urls=http://127.0.0.1:2380 --advertise-client-urls=http://127.0.0.1:2379 --advertise-server-urls=http://127.0.0.1:2380 --converged-timeout=-1 lang ${gitopsSrc}/main.mcl >/tmp/fishystuff-gitops-mgmt.log 2>&1 & echo $! >/tmp/fishystuff-gitops-mgmt.pid")

    status = "/var/lib/fishystuff/gitops-test/status/local-test.json"
    instance = "/var/lib/fishystuff/gitops-test/instances/local-test-example-release.json"
    admission = "/run/fishystuff/gitops-test/admission/local-test.json"
    marker = "/run/fishystuff/gitops-test/candidates/local-test-example-release.ready"

    machine.wait_for_file(status)
    machine.wait_for_file(instance)
    machine.wait_for_file(admission)
    machine.wait_for_file(marker)
    machine.succeed(f"jq -e '.desired_generation == 1 and .release_id == \"example-release\" and .release_identity == \"release=example-release;generation=1;git_rev=example;dolt_commit=example;dolt_repository=fishystuff/fishystuff;dolt_branch_context=local-test;dolt_mode=read_only;api=;site=;cdn_runtime=;dolt_service=\" and .environment == \"local-test\" and .host == \"vm-single-host\" and (.admission_state == \"passed_fixture\" or .admission_state == \"not_run\")' {status}")
    machine.succeed(f"jq -e '.instance_name == \"local-test-example-release\" and .release_id == \"example-release\" and .release_identity == \"release=example-release;generation=1;git_rev=example;dolt_commit=example;dolt_repository=fishystuff/fishystuff;dolt_branch_context=local-test;dolt_mode=read_only;api=;site=;cdn_runtime=;dolt_service=\" and .environment == \"local-test\" and .host == \"vm-single-host\"' {instance}")
    machine.succeed(f"jq -e '.release_identity == \"release=example-release;generation=1;git_rev=example;dolt_commit=example;dolt_repository=fishystuff/fishystuff;dolt_branch_context=local-test;dolt_mode=read_only;api=;site=;cdn_runtime=;dolt_service=\" and .admission_state == \"passed_fixture\" and .probe == \"local-fixture\"' {admission}")
    machine.succeed("test ! -e /var/lib/fishystuff/gitops-test/active/local-test.json")
    machine.succeed("kill $(cat /tmp/fishystuff-gitops-mgmt.pid) || true")

    machine.fail("systemctl is-active fishystuff-api.service")
    machine.fail("systemctl is-active fishystuff-dolt.service")
    machine.fail("systemctl is-active fishystuff-edge.service")
    machine.succeed("test ! -e /srv/fishystuff")
    machine.succeed("test ! -e /var/lib/fishystuff/mgmt")
    machine.succeed("! find /var/lib/fishystuff/gitops-test /run/fishystuff/gitops-test -type f -print0 | xargs -0 grep -E 'beta\\.fishystuff\\.fish|production|cloudflare|hcloud|ssh '")
  '';
}
