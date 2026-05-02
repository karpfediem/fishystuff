{
  pkgs,
  mgmtPackage,
  gitopsSrc,
}:
let
  siteArtifact = pkgs.runCommand "fishystuff-gitops-raw-cdn-refusal-site" { } ''
    mkdir -p "$out"
    printf 'raw cdn refusal site\n' > "$out/index.html"
  '';
  rawCdnRuntime = pkgs.runCommand "fishystuff-gitops-raw-cdn-runtime" { } ''
    mkdir -p "$out/map"
    printf '{"module":"fishystuff_ui_bevy.raw.js","wasm":"fishystuff_ui_bevy_bg.raw.wasm"}\n' > "$out/map/runtime-manifest.json"
    printf 'raw module\n' > "$out/map/fishystuff_ui_bevy.raw.js"
    printf 'raw wasm\n' > "$out/map/fishystuff_ui_bevy_bg.raw.wasm"
  '';
  previousCdnRoot = pkgs.runCommand "fishystuff-gitops-raw-cdn-refusal-previous" { } ''
    mkdir -p "$out/map"
    printf '{"module":"fishystuff_ui_bevy.previous.js","wasm":"fishystuff_ui_bevy_bg.previous.wasm"}\n' > "$out/map/runtime-manifest.json"
    printf 'previous module\n' > "$out/map/fishystuff_ui_bevy.previous.js"
    printf 'previous wasm\n' > "$out/map/fishystuff_ui_bevy_bg.previous.wasm"
  '';
  desiredState = pkgs.writeText "vm-raw-cdn-serve-refusal.desired.json" (builtins.toJSON {
    cluster = "local-test";
    generation = 9;
    mode = "vm-test";
    hosts.vm-single-host = {
      enabled = true;
      role = "single-site";
      hostname = "vm-single-host";
    };
    releases.raw-cdn-release = {
      generation = 9;
      git_rev = "raw-cdn-refusal";
      dolt_commit = "raw-cdn-refusal";
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
          store_path = "${rawCdnRuntime}";
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
        commit = "raw-cdn-refusal";
        branch_context = "local-test";
        mode = "read_only";
      };
    };
    releases.previous-release = {
      generation = 8;
      git_rev = "previous-raw-cdn-refusal";
      dolt_commit = "previous-raw-cdn-refusal";
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
          store_path = "${previousCdnRoot}";
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
        commit = "previous-raw-cdn-refusal";
        branch_context = "local-test";
        mode = "read_only";
      };
    };
    environments.local-test = {
      enabled = true;
      strategy = "single_active";
      host = "vm-single-host";
      active_release = "raw-cdn-release";
      retained_releases = [ "previous-release" ];
      serve = true;
    };
  });
in
pkgs.testers.runNixOSTest {
  name = "fishystuff-gitops-raw-cdn-serve-refusal";

  nodes.machine =
    { ... }:
    {
      system.stateVersion = "25.11";
      networking.hostName = "vm-single-host";
      virtualisation.additionalPaths = [
        siteArtifact
        rawCdnRuntime
        previousCdnRoot
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
    machine.fail("timeout 15s env FISHYSTUFF_GITOPS_STATE_FILE=${desiredState} ${mgmtPackage}/bin/mgmt run --hostname vm-single-host --tmp-prefix --no-pgp --client-urls=http://127.0.0.1:2379 --server-urls=http://127.0.0.1:2380 --advertise-client-urls=http://127.0.0.1:2379 --advertise-server-urls=http://127.0.0.1:2380 --converged-timeout=-1 lang ${gitopsSrc}/main.mcl >/tmp/fishystuff-gitops-raw-cdn-refusal.log 2>&1")
    machine.succeed("test ! -e /var/lib/fishystuff/gitops-test/active/local-test.json")
    machine.succeed("test ! -e /srv/fishystuff")
    machine.succeed("test ! -e /var/lib/fishystuff/mgmt")
  '';
}
