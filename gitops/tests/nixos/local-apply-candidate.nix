{
  pkgs,
  mgmtPackage,
  gitopsSrc,
}:
let
  desiredState = pkgs.writeText "vm-local-apply-candidate.desired.json" (builtins.toJSON {
    cluster = "local-test";
    generation = 61;
    mode = "local-apply";
    hosts.vm-single-host = {
      enabled = true;
      role = "single-site";
      hostname = "vm-single-host";
    };
    releases.local-apply-release = {
      generation = 61;
      git_rev = "local-apply-candidate";
      dolt_commit = "local-apply-candidate";
      closures = { };
      dolt = {
        repository = "fishystuff/fishystuff";
        commit = "local-apply-candidate";
        branch_context = "local-test";
        mode = "read_only";
      };
    };
    environments.local-test = {
      enabled = true;
      strategy = "single_active";
      host = "vm-single-host";
      active_release = "local-apply-release";
      retained_releases = [ ];
      serve = false;
    };
  });
in
pkgs.testers.runNixOSTest {
  name = "fishystuff-gitops-local-apply-candidate";

  nodes.machine =
    { ... }:
    {
      system.stateVersion = "25.11";
      networking.hostName = "vm-single-host";
      virtualisation.memorySize = 12288;
      virtualisation.additionalPaths = [ desiredState ];
      environment.systemPackages = [
        mgmtPackage
        pkgs.jq
      ];
    };

  testScript = ''
    start_all()

    machine.succeed("test -x ${mgmtPackage}/bin/mgmt")
    machine.succeed("jq -e '.mode == \"local-apply\" and .environments.\"local-test\".serve == false' ${desiredState}")
    machine.succeed("env FISHYSTUFF_GITOPS_ENABLE_LOCAL_APPLY=1 FISHYSTUFF_GITOPS_STATE_FILE=${desiredState} ${mgmtPackage}/bin/mgmt run --hostname vm-single-host --tmp-prefix --no-pgp --client-urls=http://127.0.0.1:2379 --server-urls=http://127.0.0.1:2380 --advertise-client-urls=http://127.0.0.1:2379 --advertise-server-urls=http://127.0.0.1:2380 --converged-timeout=-1 lang ${gitopsSrc}/main.mcl >/tmp/fishystuff-gitops-local-apply-candidate.log 2>&1 & echo $! >/tmp/fishystuff-gitops-local-apply-candidate.pid")

    status = "/var/lib/fishystuff/gitops/status/local-test.json"
    instance = "/var/lib/fishystuff/gitops/instances/local-test-local-apply-release.json"
    marker = "/run/fishystuff/gitops/candidates/local-test-local-apply-release.ready"

    machine.wait_for_file(status)
    machine.wait_for_file(instance)
    machine.wait_for_file(marker)
    machine.succeed(f"jq -e '.desired_generation == 61 and .release_id == \"local-apply-release\" and .environment == \"local-test\" and .host == \"vm-single-host\" and .phase == \"candidate\" and .admission_state == \"not_run\" and .served == false' {status}")
    machine.succeed(f"jq -e '.desired_generation == 61 and .instance_name == \"local-test-local-apply-release\" and .release_id == \"local-apply-release\" and .environment == \"local-test\" and .host == \"vm-single-host\" and .serve_requested == false' {instance}")
    machine.succeed("test ! -e /var/lib/fishystuff/gitops-test")
    machine.succeed("test ! -e /run/fishystuff/gitops-test")
    machine.succeed("test ! -e /tmp/fishystuff-gitops-test")
    machine.succeed("test ! -e /var/lib/fishystuff/gitops/active/local-test.json")
    machine.succeed("test ! -e /run/fishystuff/gitops/routes/local-test.json")
    machine.succeed("kill $(cat /tmp/fishystuff-gitops-local-apply-candidate.pid) || true")

    machine.fail("systemctl is-active fishystuff-api.service")
    machine.fail("systemctl is-active fishystuff-dolt.service")
    machine.fail("systemctl is-active fishystuff-edge.service")
    machine.succeed("test ! -e /srv/fishystuff")
    machine.succeed("test ! -e /var/lib/fishystuff/mgmt")
    machine.succeed("! find /var/lib/fishystuff/gitops /run/fishystuff/gitops -type f -print0 | xargs -0 grep -E 'beta\\.fishystuff\\.fish|production|cloudflare|hcloud|ssh '")
  '';
}
