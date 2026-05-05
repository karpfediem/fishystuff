{
  pkgs,
  mgmtPackage,
  gitopsSrc,
}:
let
  desiredState = pkgs.writeText "vm-local-apply-without-optin-refusal.desired.json" (builtins.toJSON {
    cluster = "local-test";
    generation = 60;
    mode = "local-apply";
    hosts.vm-single-host = {
      enabled = true;
      role = "single-site";
      hostname = "vm-single-host";
    };
    releases.example-release = {
      generation = 60;
      git_rev = "local-apply-without-optin";
      dolt_commit = "local-apply-without-optin";
      closures = { };
      dolt = {
        repository = "fishystuff/fishystuff";
        commit = "local-apply-without-optin";
        branch_context = "local-test";
        mode = "read_only";
      };
    };
    environments.local-test = {
      enabled = true;
      strategy = "single_active";
      host = "vm-single-host";
      active_release = "example-release";
      retained_releases = [ ];
      serve = false;
    };
  });
in
pkgs.testers.runNixOSTest {
  name = "fishystuff-gitops-local-apply-without-optin-refusal";

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
    machine.fail("timeout 120s env FISHYSTUFF_GITOPS_STATE_FILE=${desiredState} ${mgmtPackage}/bin/mgmt run --hostname vm-single-host --tmp-prefix --no-pgp --client-urls=http://127.0.0.1:2379 --server-urls=http://127.0.0.1:2380 --advertise-client-urls=http://127.0.0.1:2379 --advertise-server-urls=http://127.0.0.1:2380 --converged-timeout=-1 lang ${gitopsSrc}/main.mcl >/tmp/fishystuff-gitops-local-apply-without-optin-refusal.log 2>&1")
    machine.succeed("grep -F 'local-apply requires FISHYSTUFF_GITOPS_ENABLE_LOCAL_APPLY=1' /tmp/fishystuff-gitops-local-apply-without-optin-refusal.log")

    machine.succeed("test ! -e /var/lib/fishystuff/gitops")
    machine.succeed("test ! -e /var/lib/fishystuff/gitops-test")
    machine.succeed("test ! -e /run/fishystuff/gitops-test")
    machine.succeed("test ! -e /srv/fishystuff")
    machine.succeed("test ! -e /var/lib/fishystuff/mgmt")
    machine.fail("systemctl is-active fishystuff-api.service")
    machine.fail("systemctl is-active fishystuff-dolt.service")
    machine.fail("systemctl is-active fishystuff-edge.service")
  '';
}
