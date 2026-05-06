{
  pkgs,
  mgmtPackage,
  gitopsSrc,
}:
let
  mkArtifact =
    name:
    pkgs.runCommand "fishystuff-gitops-unused-release-noop-${name}" { } ''
      mkdir -p "$out"
      printf '%s\n' '${name}' > "$out/${name}.txt"
    '';
  apiArtifact = mkArtifact "api";
  siteArtifact = mkArtifact "site";
  cdnRuntimeArtifact = mkArtifact "cdn-runtime";
  doltServiceArtifact = mkArtifact "dolt-service";
  bogusStorePath = name: "/nix/store/aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa-unused-release-${name}";
  artifact =
    releaseId: name: storePath:
    {
      enabled = true;
      store_path = "${storePath}";
      gcroot_path = "/nix/var/nix/gcroots/fishystuff/gitops-test/${releaseId}/${name}";
    };
  release =
    {
      generation,
      releaseId,
      gitRev,
      doltCommit,
      api,
      site,
      cdn,
      doltService,
    }:
    {
      inherit generation;
      git_rev = gitRev;
      dolt_commit = doltCommit;
      closures = {
        api = artifact releaseId "api" api;
        site = artifact releaseId "site" site;
        cdn_runtime = artifact releaseId "cdn-runtime" cdn;
        dolt_service = artifact releaseId "dolt-service" doltService;
      };
      dolt = {
        repository = "fishystuff/fishystuff";
        commit = doltCommit;
        branch_context = "local-test";
        mode = "read_only";
      };
    };
  desiredState = pkgs.writeText "vm-unused-release-closure-noop.desired.json" (builtins.toJSON {
    cluster = "local-test";
    generation = 50;
    mode = "vm-test-closures";
    hosts.vm-single-host = {
      enabled = true;
      role = "single-site";
      hostname = "vm-single-host";
    };
    releases = {
      used-release = release {
        generation = 50;
        releaseId = "used-release";
        gitRev = "used-release";
        doltCommit = "used-release";
        api = apiArtifact;
        site = siteArtifact;
        cdn = cdnRuntimeArtifact;
        doltService = doltServiceArtifact;
      };
      unused-release = release {
        generation = 49;
        releaseId = "unused-release";
        gitRev = "unused-release";
        doltCommit = "unused-release";
        api = bogusStorePath "api";
        site = bogusStorePath "site";
        cdn = bogusStorePath "cdn-runtime";
        doltService = bogusStorePath "dolt-service";
      };
    };
    environments.local-test = {
      enabled = true;
      strategy = "single_active";
      host = "vm-single-host";
      active_release = "used-release";
      retained_releases = [ ];
      serve = false;
    };
  });
in
pkgs.testers.runNixOSTest {
  name = "fishystuff-gitops-unused-release-closure-noop";

  nodes.machine =
    { ... }:
    {
      system.stateVersion = "25.11";
      networking.hostName = "vm-single-host";
      virtualisation.memorySize = 12288;
      virtualisation.additionalPaths = [
        apiArtifact
        siteArtifact
        cdnRuntimeArtifact
        doltServiceArtifact
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
    machine.succeed("jq -e '.mode == \"vm-test-closures\" and .environments.\"local-test\".active_release == \"used-release\" and (.releases.\"unused-release\" != null)' ${desiredState}")
    machine.succeed("env FISHYSTUFF_GITOPS_STATE_FILE=${desiredState} ${mgmtPackage}/bin/mgmt run --hostname vm-single-host --tmp-prefix --no-pgp --client-urls=http://127.0.0.1:2379 --server-urls=http://127.0.0.1:2380 --advertise-client-urls=http://127.0.0.1:2379 --advertise-server-urls=http://127.0.0.1:2380 --converged-timeout=-1 lang ${gitopsSrc}/main.mcl >/tmp/fishystuff-gitops-unused-release-closure-noop.log 2>&1 & echo $! >/tmp/fishystuff-gitops-unused-release-closure-noop.pid")

    roots = {
      "used-release/api": "${apiArtifact}",
      "used-release/site": "${siteArtifact}",
      "used-release/cdn-runtime": "${cdnRuntimeArtifact}",
      "used-release/dolt-service": "${doltServiceArtifact}",
    }

    for name, target in roots.items():
      root = f"/nix/var/nix/gcroots/fishystuff/gitops-test/{name}"
      machine.succeed(f"bash -c 'deadline=$((SECONDS + 300)); until test -L {root}; do if ! kill -0 $(cat /tmp/fishystuff-gitops-unused-release-closure-noop.pid); then cat /tmp/fishystuff-gitops-unused-release-closure-noop.log; exit 1; fi; if [ \"$SECONDS\" -ge \"$deadline\" ]; then cat /tmp/fishystuff-gitops-unused-release-closure-noop.log; exit 1; fi; sleep 1; done'")
      machine.succeed(f"test \"$(readlink {root})\" = \"{target}\"")
      machine.succeed(f"nix-store --gc --print-roots | grep -F {root}")
      machine.succeed(f"nix-store --verify-path {target}")

    status = "/var/lib/fishystuff/gitops-test/status/local-test.json"
    instance = "/var/lib/fishystuff/gitops-test/instances/local-test-used-release.json"

    machine.wait_for_file(status)
    machine.wait_for_file(instance)
    machine.succeed(f"jq -e '.desired_generation == 50 and .release_id == \"used-release\" and .environment == \"local-test\" and .admission_state == \"passed_fixture\" and .served == false' {status}")
    machine.succeed(f"jq -e '.release_id == \"used-release\" and .api_bundle == \"${apiArtifact}\" and .site_content == \"${siteArtifact}\" and .cdn_runtime_content == \"${cdnRuntimeArtifact}\" and .dolt_service_bundle == \"${doltServiceArtifact}\"' {instance}")

    machine.succeed("test ! -e /nix/var/nix/gcroots/fishystuff/gitops-test/unused-release")
    machine.succeed("test ! -e /var/lib/fishystuff/gitops-test/instances/local-test-unused-release.json")
    machine.succeed("test ! -e /var/lib/fishystuff/gitops-test/active/local-test.json")
    machine.succeed("kill $(cat /tmp/fishystuff-gitops-unused-release-closure-noop.pid) || true")

    machine.fail("systemctl is-active fishystuff-api.service")
    machine.fail("systemctl is-active fishystuff-dolt.service")
    machine.fail("systemctl is-active fishystuff-edge.service")
    machine.succeed("test ! -e /srv/fishystuff")
    machine.succeed("test ! -e /var/lib/fishystuff/mgmt")
    machine.succeed("! find /var/lib/fishystuff/gitops-test /run/fishystuff/gitops-test -type f -print0 | xargs -0 grep -E 'beta\\.fishystuff\\.fish|production|cloudflare|hcloud|ssh '")
  '';
}
