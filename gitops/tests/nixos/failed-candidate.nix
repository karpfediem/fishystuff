{
  pkgs,
  mgmtPackage,
  gitopsSrc,
}:
let
  desiredState = pkgs.writeText "vm-failed-candidate.desired.json" (builtins.toJSON {
    cluster = "local-test";
    generation = 15;
    mode = "vm-test";
    hosts.vm-single-host = {
      enabled = true;
      role = "single-site";
      hostname = "vm-single-host";
    };
    releases.failed-candidate-release = {
      generation = 15;
      git_rev = "failed-candidate";
      dolt_commit = "failed-candidate";
      closures = {
        api = {
          enabled = false;
          store_path = "";
          gcroot_path = "";
        };
        site = {
          enabled = false;
          store_path = "";
          gcroot_path = "";
        };
        cdn_runtime = {
          enabled = false;
          store_path = "";
          gcroot_path = "";
        };
        dolt_service = {
          enabled = false;
          store_path = "";
          gcroot_path = "";
        };
      };
      dolt = {
        repository = "fishystuff/fishystuff";
        commit = "failed-candidate";
        branch_context = "local-test";
        mode = "read_only";
      };
    };
    environments.local-test = {
      enabled = true;
      strategy = "single_active";
      host = "vm-single-host";
      active_release = "failed-candidate-release";
      retained_releases = [ ];
      serve = false;
      admission_fixture_state = "failed_fixture";
    };
  });
in
pkgs.testers.runNixOSTest {
  name = "fishystuff-gitops-failed-candidate";

  nodes.machine =
    { ... }:
    {
      system.stateVersion = "25.11";
      networking.hostName = "vm-single-host";
      virtualisation.additionalPaths = [
        desiredState
      ];
      environment.systemPackages = [
        mgmtPackage
        pkgs.jq
      ];
    };

  testScript = ''
    start_all()

    machine.succeed("test -x ${mgmtPackage}/bin/mgmt")
    machine.succeed("jq -e '.mode == \"vm-test\" and .environments.\"local-test\".admission_fixture_state == \"failed_fixture\" and .environments.\"local-test\".serve == false' ${desiredState}")
    machine.succeed("env FISHYSTUFF_GITOPS_STATE_FILE=${desiredState} ${mgmtPackage}/bin/mgmt run --hostname vm-single-host --tmp-prefix --no-pgp --client-urls=http://127.0.0.1:2379 --server-urls=http://127.0.0.1:2380 --advertise-client-urls=http://127.0.0.1:2379 --advertise-server-urls=http://127.0.0.1:2380 --converged-timeout=-1 lang ${gitopsSrc}/main.mcl >/tmp/fishystuff-gitops-failed-candidate.log 2>&1 & echo $! >/tmp/fishystuff-gitops-failed-candidate.pid")

    status = "/var/lib/fishystuff/gitops-test/status/local-test.json"
    instance = "/var/lib/fishystuff/gitops-test/instances/local-test-failed-candidate-release.json"
    admission = "/run/fishystuff/gitops-test/admission/local-test.json"

    machine.wait_for_file(status)
    machine.wait_for_file(instance)
    machine.wait_for_file(admission)

    machine.succeed(f"jq -e '.desired_generation == 15 and .release_id == \"failed-candidate-release\" and .environment == \"local-test\" and .host == \"vm-single-host\" and .phase == \"candidate\" and .admission_state == \"failed_fixture\" and .served == false and .failure_reason == \"admission_failed\"' {status}")
    machine.succeed(f"jq -e '.desired_generation == 15 and .release_id == \"failed-candidate-release\" and .serve_requested == false' {instance}")
    machine.succeed(f"jq -e '.release_id == \"failed-candidate-release\" and .admission_state == \"failed_fixture\" and .serving_artifacts_checked == false and .probe == \"local-fixture\"' {admission}")
    machine.succeed("test ! -e /var/lib/fishystuff/gitops-test/active/local-test.json")
    machine.succeed("test ! -e /var/lib/fishystuff/gitops-test/served/site")
    machine.succeed("test ! -e /var/lib/fishystuff/gitops-test/served/cdn")
    machine.succeed("test ! -e /run/fishystuff/gitops-test/routes/local-test.json")
    machine.succeed("kill $(cat /tmp/fishystuff-gitops-failed-candidate.pid) || true")

    machine.fail("systemctl is-active fishystuff-api.service")
    machine.fail("systemctl is-active fishystuff-dolt.service")
    machine.fail("systemctl is-active fishystuff-edge.service")
    machine.succeed("test ! -e /srv/fishystuff")
    machine.succeed("test ! -e /var/lib/fishystuff/mgmt")
  '';
}
