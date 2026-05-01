{
  pkgs,
  mgmtPackage,
  gitopsSrc,
}:
let
  mkArtifact =
    name:
    pkgs.runCommand "fishystuff-gitops-${name}-artifact" { } ''
      mkdir -p "$out"
      printf '%s\n' '${name}' > "$out/${name}.txt"
    '';
  apiArtifact = mkArtifact "api";
  siteArtifact = mkArtifact "site";
  cdnRuntimeArtifact = mkArtifact "cdn-runtime";
  doltServiceArtifact = mkArtifact "dolt-service";
  desiredState = pkgs.writeText "vm-closure-roots.example.desired.json" (builtins.toJSON {
    cluster = "local-test";
    generation = 2;
    mode = "vm-test-closures";
    hosts.vm-single-host = {
      enabled = true;
      role = "single-site";
      hostname = "vm-single-host";
    };
    releases.example-release = {
      generation = 2;
      git_rev = "closure-test";
      dolt_commit = "closure-test";
      closures = {
        api = {
          enabled = true;
          store_path = "${apiArtifact}";
          gcroot_path = "/var/lib/fishystuff/gitops-test/gcroots/example-release/api";
        };
        site = {
          enabled = true;
          store_path = "${siteArtifact}";
          gcroot_path = "/var/lib/fishystuff/gitops-test/gcroots/example-release/site";
        };
        cdn_runtime = {
          enabled = true;
          store_path = "${cdnRuntimeArtifact}";
          gcroot_path = "/var/lib/fishystuff/gitops-test/gcroots/example-release/cdn-runtime";
        };
        dolt_service = {
          enabled = true;
          store_path = "${doltServiceArtifact}";
          gcroot_path = "/var/lib/fishystuff/gitops-test/gcroots/example-release/dolt-service";
        };
      };
      dolt = {
        repository = "fishystuff/fishystuff";
        commit = "closure-test";
        branch_context = "local-test";
        mode = "read_only";
      };
    };
    environments.local-test = {
      enabled = true;
      strategy = "single_active";
      host = "vm-single-host";
      active_release = "example-release";
      serve = false;
    };
  });
in
pkgs.testers.runNixOSTest {
  name = "fishystuff-gitops-closure-roots";

  nodes.machine =
    { ... }:
    {
      system.stateVersion = "25.11";
      networking.hostName = "vm-single-host";
      virtualisation.additionalPaths = [
        apiArtifact
        siteArtifact
        cdnRuntimeArtifact
        doltServiceArtifact
      ];
      environment.systemPackages = [
        mgmtPackage
        pkgs.jq
      ];
    };

  testScript = ''
    start_all()

    machine.succeed("test -x ${mgmtPackage}/bin/mgmt")
    machine.succeed("nix-store --query --hash ${apiArtifact}")
    machine.succeed("nix-store --query --hash ${siteArtifact}")
    machine.succeed("nix-store --query --hash ${cdnRuntimeArtifact}")
    machine.succeed("nix-store --query --hash ${doltServiceArtifact}")
    machine.succeed("env FISHYSTUFF_GITOPS_STATE_FILE=${desiredState} ${mgmtPackage}/bin/mgmt run --hostname vm-single-host --tmp-prefix --no-pgp --client-urls=http://127.0.0.1:2379 --server-urls=http://127.0.0.1:2380 --advertise-client-urls=http://127.0.0.1:2379 --advertise-server-urls=http://127.0.0.1:2380 --converged-timeout=-1 lang ${gitopsSrc}/main.mcl >/tmp/fishystuff-gitops-mgmt.log 2>&1 & echo $! >/tmp/fishystuff-gitops-mgmt.pid")

    roots = {
      "api": "${apiArtifact}",
      "site": "${siteArtifact}",
      "cdn-runtime": "${cdnRuntimeArtifact}",
      "dolt-service": "${doltServiceArtifact}",
    }

    for name, target in roots.items():
      root = f"/var/lib/fishystuff/gitops-test/gcroots/example-release/{name}"
      machine.succeed(f"bash -c 'deadline=$((SECONDS + 120)); until test -L {root}; do if ! kill -0 $(cat /tmp/fishystuff-gitops-mgmt.pid); then cat /tmp/fishystuff-gitops-mgmt.log; exit 1; fi; if [ \"$SECONDS\" -ge \"$deadline\" ]; then cat /tmp/fishystuff-gitops-mgmt.log; exit 1; fi; sleep 1; done'")
      machine.succeed(f"test \"$(readlink {root})\" = \"{target}\"")
      machine.succeed(f"nix-store --verify-path {target}")

    status = "/var/lib/fishystuff/gitops-test/status/local-test.json"
    instance = "/var/lib/fishystuff/gitops-test/instances/local-test-example-release.json"

    machine.wait_for_file(status)
    machine.wait_for_file(instance)
    machine.succeed(f"jq -e '.desired_generation == 2 and .release_id == \"example-release\" and .environment == \"local-test\" and .admission_state == \"passed_fixture\" and .served == false' {status}")
    machine.succeed(f"jq -e '.api_bundle == \"${apiArtifact}\" and .site_content == \"${siteArtifact}\" and .cdn_runtime_content == \"${cdnRuntimeArtifact}\" and .dolt_service_bundle == \"${doltServiceArtifact}\"' {instance}")
    machine.succeed("test ! -e /var/lib/fishystuff/gitops-test/active/local-test.json")
    machine.succeed("kill $(cat /tmp/fishystuff-gitops-mgmt.pid)")

    machine.fail("systemctl is-active fishystuff-api.service")
    machine.fail("systemctl is-active fishystuff-dolt.service")
    machine.fail("systemctl is-active fishystuff-edge.service")
    machine.succeed("test ! -e /srv/fishystuff")
    machine.succeed("test ! -e /var/lib/fishystuff/mgmt")
    machine.succeed("! find /var/lib/fishystuff/gitops-test /run/fishystuff/gitops-test -type f -print0 | xargs -0 grep -E 'beta\\.fishystuff\\.fish|production|cloudflare|hcloud|ssh '")
  '';
}
