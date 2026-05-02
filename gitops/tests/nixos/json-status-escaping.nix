{
  pkgs,
  mgmtPackage,
  gitopsSrc,
}:
let
  specialGitRev = "git-with-\"quote\\slash";
  specialDoltCommit = "commit-with-\"quote\\slash";
  releaseId = "escaped-json-release";
  expectedReleaseIdentity =
    "release=${releaseId};generation=12;git_rev=${specialGitRev};dolt_commit=${specialDoltCommit};dolt_repository=fishystuff/fishystuff;dolt_branch_context=local-test;dolt_mode=read_only;api=;site=;cdn_runtime=;dolt_service=";
  expectedReleaseIdentityFile = pkgs.writeText "fishystuff-gitops-escaped-release-identity" expectedReleaseIdentity;
  expectedDoltCommitFile = pkgs.writeText "fishystuff-gitops-escaped-dolt-commit" specialDoltCommit;
  desiredState = pkgs.writeText "vm-json-status-escaping.desired.json" (builtins.toJSON {
    cluster = "local-test";
    generation = 12;
    mode = "vm-test";
    hosts.vm-single-host = {
      enabled = true;
      role = "single-site";
      hostname = "vm-single-host";
    };
    releases.${releaseId} = {
      generation = 12;
      git_rev = specialGitRev;
      dolt_commit = specialDoltCommit;
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
        commit = specialDoltCommit;
        branch_context = "local-test";
        mode = "read_only";
      };
    };
    environments.local-test = {
      enabled = true;
      strategy = "single_active";
      host = "vm-single-host";
      active_release = releaseId;
      retained_releases = [ ];
      serve = false;
    };
  });
in
pkgs.testers.runNixOSTest {
  name = "fishystuff-gitops-json-status-escaping";

  nodes.machine =
    { ... }:
    {
      system.stateVersion = "25.11";
      networking.hostName = "vm-single-host";
      virtualisation.additionalPaths = [
        desiredState
        expectedDoltCommitFile
        expectedReleaseIdentityFile
      ];
      environment.systemPackages = [
        mgmtPackage
        pkgs.jq
      ];
    };

  testScript = ''
    start_all()

    machine.succeed("test -x ${mgmtPackage}/bin/mgmt")
    machine.succeed("env FISHYSTUFF_GITOPS_STATE_FILE=${desiredState} ${mgmtPackage}/bin/mgmt run --hostname vm-single-host --tmp-prefix --no-pgp --client-urls=http://127.0.0.1:2379 --server-urls=http://127.0.0.1:2380 --advertise-client-urls=http://127.0.0.1:2379 --advertise-server-urls=http://127.0.0.1:2380 --converged-timeout=-1 lang ${gitopsSrc}/main.mcl >/tmp/fishystuff-gitops-json-status-escaping.log 2>&1 & echo $! >/tmp/fishystuff-gitops-json-status-escaping.pid")

    status = "/var/lib/fishystuff/gitops-test/status/local-test.json"
    instance = "/var/lib/fishystuff/gitops-test/instances/local-test-escaped-json-release.json"
    admission = "/run/fishystuff/gitops-test/admission/local-test.json"

    machine.wait_for_file(status)
    machine.wait_for_file(instance)
    machine.wait_for_file(admission)

    machine.succeed(f"expected=$(cat ${expectedReleaseIdentityFile}); jq -e --arg expected \"$expected\" '.desired_generation == 12 and .release_id == \"escaped-json-release\" and .release_identity == $expected and .phase == \"candidate\" and .served == false' {status}")
    machine.succeed(f"expected=$(cat ${expectedReleaseIdentityFile}); commit=$(cat ${expectedDoltCommitFile}); jq -e --arg expected \"$expected\" --arg commit \"$commit\" '.release_id == \"escaped-json-release\" and .release_identity == $expected and .dolt_commit == $commit and .serve_requested == false' {instance}")
    machine.succeed(f"expected=$(cat ${expectedReleaseIdentityFile}); jq -e --arg expected \"$expected\" '.release_id == \"escaped-json-release\" and .release_identity == $expected and .admission_state == \"passed_fixture\" and .probe == \"local-fixture\"' {admission}")
    machine.succeed("test ! -e /var/lib/fishystuff/gitops-test/active/local-test.json")
    machine.succeed("kill $(cat /tmp/fishystuff-gitops-json-status-escaping.pid) || true")
  '';
}
