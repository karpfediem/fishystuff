{
  pkgs,
  mgmtPackage,
  gitopsSrc,
}:
let
  closureSet = {
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
  desiredState = pkgs.writeText "vm-failed-served-candidate-refusal.desired.json" (builtins.toJSON {
    cluster = "local-test";
    generation = 16;
    mode = "vm-test";
    hosts.vm-single-host = {
      enabled = true;
      role = "single-site";
      hostname = "vm-single-host";
    };
    releases.candidate-release = {
      generation = 16;
      git_rev = "failed-served-candidate";
      dolt_commit = "failed-served-candidate";
      closures = closureSet;
      dolt = {
        repository = "fishystuff/fishystuff";
        commit = "failed-served-candidate";
        branch_context = "local-test";
        mode = "read_only";
      };
    };
    releases.previous-release = {
      generation = 15;
      git_rev = "previous-failed-served-candidate";
      dolt_commit = "previous-failed-served-candidate";
      closures = closureSet;
      dolt = {
        repository = "fishystuff/fishystuff";
        commit = "previous-failed-served-candidate";
        branch_context = "local-test";
        mode = "read_only";
      };
    };
    environments.local-test = {
      enabled = true;
      strategy = "single_active";
      host = "vm-single-host";
      active_release = "candidate-release";
      retained_releases = [ "previous-release" ];
      serve = true;
      admission_fixture_state = "failed_fixture";
    };
  });
in
pkgs.testers.runNixOSTest {
  name = "fishystuff-gitops-failed-served-candidate-refusal";

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
    machine.succeed("jq -e '.mode == \"vm-test\" and .environments.\"local-test\".serve == true and .environments.\"local-test\".admission_fixture_state == \"failed_fixture\" and .environments.\"local-test\".retained_releases == [\"previous-release\"]' ${desiredState}")
    machine.fail("timeout 15s env FISHYSTUFF_GITOPS_STATE_FILE=${desiredState} ${mgmtPackage}/bin/mgmt run --hostname vm-single-host --tmp-prefix --no-pgp --client-urls=http://127.0.0.1:2379 --server-urls=http://127.0.0.1:2380 --advertise-client-urls=http://127.0.0.1:2379 --advertise-server-urls=http://127.0.0.1:2380 --converged-timeout=-1 lang ${gitopsSrc}/main.mcl >/tmp/fishystuff-gitops-failed-served-candidate-refusal.log 2>&1")
    machine.succeed("grep -F 'serving requires passed admission' /tmp/fishystuff-gitops-failed-served-candidate-refusal.log")
    machine.succeed("test ! -e /var/lib/fishystuff/gitops-test/status/local-test.json")
    machine.succeed("test ! -e /var/lib/fishystuff/gitops-test/instances/local-test-candidate-release.json")
    machine.succeed("test ! -e /var/lib/fishystuff/gitops-test/active/local-test.json")
    machine.succeed("test ! -e /var/lib/fishystuff/gitops-test/served/site")
    machine.succeed("test ! -e /var/lib/fishystuff/gitops-test/served/cdn")
    machine.succeed("test ! -e /run/fishystuff/gitops-test/routes/local-test.json")
    machine.succeed("test ! -e /srv/fishystuff")
    machine.succeed("test ! -e /var/lib/fishystuff/mgmt")
  '';
}
