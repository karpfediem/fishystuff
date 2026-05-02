{
  pkgs,
  mgmtPackage,
  gitopsSrc,
}:
let
  emptyArtifact = {
    enabled = false;
    store_path = "";
    gcroot_path = "";
  };

  release = {
    generation,
    gitRev,
    doltCommit,
  }: {
    inherit generation;
    git_rev = gitRev;
    dolt_commit = doltCommit;
    closures = {
      api = emptyArtifact;
      site = emptyArtifact;
      cdn_runtime = emptyArtifact;
      dolt_service = emptyArtifact;
    };
    dolt = {
      repository = "fishystuff/fishystuff";
      commit = doltCommit;
      branch_context = "preview";
      mode = "read_only";
    };
  };

  desiredState = pkgs.writeText "vm-multi-environment-candidates.desired.json" (builtins.toJSON {
    cluster = "preview-local";
    generation = 6;
    mode = "vm-test";
    hosts = {
      local-preview-host = {
        enabled = true;
        role = "single-site";
        hostname = "vm-single-host";
      };
    };
    releases = {
      preview-a-release = release {
        generation = 11;
        gitRev = "preview-a";
        doltCommit = "preview-a";
      };
      preview-b-release = release {
        generation = 12;
        gitRev = "preview-b";
        doltCommit = "preview-b";
      };
    };
    environments = {
      preview-branch-a = {
        enabled = true;
        strategy = "single_active";
        host = "local-preview-host";
        active_release = "preview-a-release";
        retained_releases = [ ];
        serve = false;
      };
      preview-branch-b = {
        enabled = true;
        strategy = "single_active";
        host = "local-preview-host";
        active_release = "preview-b-release";
        retained_releases = [ ];
        serve = false;
      };
    };
  });
in
pkgs.testers.runNixOSTest {
  name = "fishystuff-gitops-multi-environment-candidates";

  nodes.machine =
    { ... }:
    {
      system.stateVersion = "25.11";
      networking.hostName = "vm-single-host";
      virtualisation.memorySize = 2048;
      environment.systemPackages = [
        mgmtPackage
        pkgs.jq
      ];
    };

  testScript = ''
    start_all()

    machine.succeed("test -x ${mgmtPackage}/bin/mgmt")
    machine.succeed("jq -e '.mode == \"vm-test\" and (.environments | keys | length) == 2' ${desiredState}")
    machine.succeed("env FISHYSTUFF_GITOPS_STATE_FILE=${desiredState} ${mgmtPackage}/bin/mgmt run --hostname vm-single-host --tmp-prefix --no-pgp --client-urls=http://127.0.0.1:2379 --server-urls=http://127.0.0.1:2380 --advertise-client-urls=http://127.0.0.1:2379 --advertise-server-urls=http://127.0.0.1:2380 --converged-timeout=-1 lang ${gitopsSrc}/main.mcl >/tmp/fishystuff-gitops-multi-environment.log 2>&1 & echo $! >/tmp/fishystuff-gitops-multi-environment.pid")

    for env, release, generation, git_rev in [
        ("preview-branch-a", "preview-a-release", 11, "preview-a"),
        ("preview-branch-b", "preview-b-release", 12, "preview-b"),
    ]:
        identity = f"release={release};generation={generation};git_rev={git_rev};dolt_commit={git_rev};dolt_repository=fishystuff/fishystuff;dolt_branch_context=preview;dolt_mode=read_only;api=;site=;cdn_runtime=;dolt_service="
        status = f"/var/lib/fishystuff/gitops-test/status/{env}.json"
        instance = f"/var/lib/fishystuff/gitops-test/instances/{env}-{release}.json"
        admission = f"/run/fishystuff/gitops-test/admission/{env}.json"
        marker = f"/run/fishystuff/gitops-test/candidates/{env}-{release}.ready"
        active = f"/var/lib/fishystuff/gitops-test/active/{env}.json"
        route = f"/run/fishystuff/gitops-test/routes/{env}.json"

        machine.wait_for_file(status)
        machine.wait_for_file(instance)
        machine.wait_for_file(admission)
        machine.wait_for_file(marker)
        machine.succeed(f"jq -e '.desired_generation == 6 and .release_id == \"{release}\" and .release_identity == \"{identity}\" and .environment == \"{env}\" and .host == \"vm-single-host\" and .phase == \"candidate\" and .admission_state == \"passed_fixture\" and .served == false and .retained_release_ids == []' {status}")
        machine.succeed(f"jq -e '.cluster == \"preview-local\" and .desired_generation == 6 and .instance_name == \"{env}-{release}\" and .release_identity == \"{identity}\" and .environment == \"{env}\" and .host == \"vm-single-host\" and .serve_requested == false' {instance}")
        machine.succeed(f"jq -e '.environment == \"{env}\" and .release_id == \"{release}\" and .release_identity == \"{identity}\" and .admission_state == \"passed_fixture\" and .probe == \"local-fixture\"' {admission}")
        machine.succeed(f"test ! -e {active}")
        machine.succeed(f"test ! -e {route}")

    machine.succeed("test ! -e /var/lib/fishystuff/gitops-test/served")
    machine.succeed("kill $(cat /tmp/fishystuff-gitops-multi-environment.pid) || true")

    machine.fail("systemctl is-active fishystuff-api.service")
    machine.fail("systemctl is-active fishystuff-dolt.service")
    machine.fail("systemctl is-active fishystuff-edge.service")
    machine.succeed("test ! -e /srv/fishystuff")
    machine.succeed("test ! -e /var/lib/fishystuff/mgmt")
    machine.succeed("! find /var/lib/fishystuff/gitops-test /run/fishystuff/gitops-test -type f -print0 | xargs -0 grep -E 'beta\\.fishystuff\\.fish|production|cloudflare|hcloud|ssh '")
  '';
}
