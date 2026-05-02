{
  pkgs,
  mgmtPackage,
  gitopsSrc,
}:
let
  siteArtifact = pkgs.runCommand "fishystuff-gitops-missing-cdn-serving-entry-site" { } ''
    mkdir -p "$out"
    printf 'missing cdn serving manifest entry refusal site\n' > "$out/index.html"
  '';
  brokenCdnServingRoot = pkgs.runCommand "fishystuff-gitops-missing-cdn-serving-entry-root" { } ''
    mkdir -p "$out/map"
    printf '{"module":"fishystuff_ui_bevy.present.js","wasm":"fishystuff_ui_bevy_bg.present.wasm"}\n' > "$out/map/runtime-manifest.json"
    printf 'present module\n' > "$out/map/fishystuff_ui_bevy.present.js"
    printf 'present wasm\n' > "$out/map/fishystuff_ui_bevy_bg.present.wasm"
    printf '{"schema_version":1,"current_root":"%s","retained_root_count":0,"assets":[{"path":"/map/runtime-manifest.json","source":"current"},{"path":"/map/fishystuff_ui_bevy.present.js","source":"current"}]}\n' "$out" > "$out/cdn-serving-manifest.json"
  '';
  desiredState = pkgs.writeText "vm-missing-cdn-serving-manifest-entry-refusal.desired.json" (builtins.toJSON {
    cluster = "local-test";
    generation = 11;
    mode = "vm-test";
    hosts.vm-single-host = {
      enabled = true;
      role = "single-site";
      hostname = "vm-single-host";
    };
    releases.missing-cdn-serving-manifest-entry-release = {
      generation = 11;
      git_rev = "missing-cdn-serving-manifest-entry-refusal";
      dolt_commit = "missing-cdn-serving-manifest-entry-refusal";
      closures = {
        api = {
          enabled = false;
          store_path = "";
          gcroot_path = "";
        };
        site = {
          enabled = false;
          store_path = "${siteArtifact}";
          gcroot_path = "";
        };
        cdn_runtime = {
          enabled = false;
          store_path = "${brokenCdnServingRoot}";
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
        commit = "missing-cdn-serving-manifest-entry-refusal";
        branch_context = "local-test";
        mode = "read_only";
      };
    };
    releases.previous-release = {
      generation = 10;
      git_rev = "previous-missing-cdn-serving-manifest-entry-refusal";
      dolt_commit = "previous-missing-cdn-serving-manifest-entry-refusal";
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
        commit = "previous-missing-cdn-serving-manifest-entry-refusal";
        branch_context = "local-test";
        mode = "read_only";
      };
    };
    environments.local-test = {
      enabled = true;
      strategy = "single_active";
      host = "vm-single-host";
      active_release = "missing-cdn-serving-manifest-entry-release";
      retained_releases = [ "previous-release" ];
      serve = true;
    };
  });
in
pkgs.testers.runNixOSTest {
  name = "fishystuff-gitops-missing-cdn-serving-manifest-entry-refusal";

  nodes.machine =
    { ... }:
    {
      system.stateVersion = "25.11";
      networking.hostName = "vm-single-host";
      virtualisation.additionalPaths = [
        siteArtifact
        brokenCdnServingRoot
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
    machine.succeed("jq -e '.mode == \"vm-test\" and .environments.\"local-test\".serve == true' ${desiredState}")
    machine.succeed("test -f ${brokenCdnServingRoot}/cdn-serving-manifest.json")
    machine.succeed("test -f ${brokenCdnServingRoot}/map/runtime-manifest.json")
    machine.succeed("test -f ${brokenCdnServingRoot}/map/fishystuff_ui_bevy.present.js")
    machine.succeed("test -f ${brokenCdnServingRoot}/map/fishystuff_ui_bevy_bg.present.wasm")
    machine.succeed("! grep -F fishystuff_ui_bevy_bg.present.wasm ${brokenCdnServingRoot}/cdn-serving-manifest.json")
    machine.fail("timeout 15s env FISHYSTUFF_GITOPS_STATE_FILE=${desiredState} ${mgmtPackage}/bin/mgmt run --hostname vm-single-host --tmp-prefix --no-pgp --client-urls=http://127.0.0.1:2379 --server-urls=http://127.0.0.1:2380 --advertise-client-urls=http://127.0.0.1:2379 --advertise-server-urls=http://127.0.0.1:2380 --converged-timeout=-1 lang ${gitopsSrc}/main.mcl >/tmp/fishystuff-gitops-missing-cdn-serving-manifest-entry-refusal.log 2>&1")
    machine.succeed("test ! -e /var/lib/fishystuff/gitops-test/active/local-test.json")
    machine.succeed("test ! -e /srv/fishystuff")
    machine.succeed("test ! -e /var/lib/fishystuff/mgmt")
  '';
}
