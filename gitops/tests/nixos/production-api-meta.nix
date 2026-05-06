{
  pkgs,
  mgmtPackage,
  fishystuffServerPackage,
  fishystuffDeployPackage,
  gitopsSrc,
  desiredState,
  apiArtifact,
  siteArtifact,
  cdnRuntimeArtifact,
  cdnRuntimeCurrentArtifact,
  doltServiceArtifact,
  previousApiArtifact,
  previousSiteArtifact,
  previousCdnRuntimeArtifact,
  previousCdnRuntimeCurrentArtifact,
  previousDoltServiceArtifact,
}:
let
  apiBaseConfig = pkgs.callPackage ../../../nix/packages/api-service-base-config.nix { };
  waitForPinnedDoltCache = pkgs.writeShellScript "wait-for-gitops-production-pinned-dolt-cache" ''
    set -eu

    i=0
    while [ "$i" -lt 240 ]; do
      if [ -s /run/fishystuff/gitops/dolt/production-production-api-meta-release.json ] \
        && [ -d /var/lib/fishystuff/gitops/dolt-cache/fishystuff/.dolt ]; then
        exit 0
      fi
      i=$((i + 1))
      sleep 1
    done

    exit 1
  '';
in
pkgs.testers.runNixOSTest {
  name = "fishystuff-gitops-production-api-meta";

  nodes.machine =
    { ... }:
    {
      system.stateVersion = "25.11";
      networking.hostName = "production-single-host";
      virtualisation.memorySize = 12288;
      virtualisation.additionalPaths = [
        desiredState
        apiArtifact
        siteArtifact
        cdnRuntimeArtifact
        cdnRuntimeCurrentArtifact
        doltServiceArtifact
        previousApiArtifact
        previousSiteArtifact
        previousCdnRuntimeArtifact
        previousCdnRuntimeCurrentArtifact
        previousDoltServiceArtifact
      ];
      environment.systemPackages = [
        fishystuffDeployPackage
        mgmtPackage
        pkgs.curl
        pkgs.dolt
        pkgs.jq
      ];
      systemd.services.fishystuff-gitops-production-dolt-sql-fixture = {
        serviceConfig = {
          ExecStartPre = "${waitForPinnedDoltCache}";
          ExecStart = "${pkgs.dolt}/bin/dolt sql-server --host 127.0.0.1 --port 18093 --data-dir /var/lib/fishystuff/gitops/dolt-cache/fishystuff --loglevel warning";
          Restart = "always";
          RestartSec = "1s";
        };
      };
      systemd.services.fishystuff-gitops-candidate-api-production = {
        after = [ "fishystuff-gitops-production-dolt-sql-fixture.service" ];
        requires = [ "fishystuff-gitops-production-dolt-sql-fixture.service" ];
        serviceConfig = {
          Environment = [
            "FISHYSTUFF_DATABASE_URL=mysql://root@127.0.0.1:18093/fishystuff"
            "FISHYSTUFF_CORS_ALLOWED_ORIGINS=https://fishystuff.fish"
            "FISHYSTUFF_PUBLIC_SITE_BASE_URL=https://fishystuff.fish"
            "FISHYSTUFF_PUBLIC_CDN_BASE_URL=https://cdn.fishystuff.fish"
            "FISHYSTUFF_RUNTIME_CDN_BASE_URL=https://cdn.fishystuff.fish"
            "FISHYSTUFF_OTEL_ENABLED=0"
          ];
          EnvironmentFile = "/var/lib/fishystuff/gitops/api/production.env";
          ExecStart = "${fishystuffServerPackage}/bin/fishystuff_server --config ${apiBaseConfig} --bind 127.0.0.1:18092 --request-timeout-secs 5";
          Restart = "always";
          RestartSec = "1s";
        };
      };
    };

  testScript = ''
    import textwrap

    start_all()

    work = "/tmp/fishystuff-gitops-production-api-meta"
    desired = f"{work}/desired.json"
    mgmt_log = "/tmp/fishystuff-gitops-production-api-meta.log"
    mgmt_pid = "/tmp/fishystuff-gitops-production-api-meta.pid"
    api_config = "/var/lib/fishystuff/gitops/api/production.json"
    api_env = "/var/lib/fishystuff/gitops/api/production.env"
    status = "/var/lib/fishystuff/gitops/status/production.json"
    active = "/var/lib/fishystuff/gitops/active/production.json"
    route = "/run/fishystuff/gitops/routes/production.json"
    admission = "/run/fishystuff/gitops/admission/production.json"
    rollback = "/var/lib/fishystuff/gitops/rollback/production.json"
    rollback_set = "/var/lib/fishystuff/gitops/rollback-set/production.json"
    previous_rollback_member = "/var/lib/fishystuff/gitops/rollback-set/production/previous-production-release.json"
    cache = "/var/lib/fishystuff/gitops/dolt-cache/fishystuff"
    remote = f"{work}/remote"
    release_id = "production-api-meta-release"
    previous_release_id = "previous-production-release"
    release_ref = "fishystuff/gitops/production-api-meta-release"
    previous_release_ref = "fishystuff/gitops/previous-production-release"
    dolt_status = f"/run/fishystuff/gitops/dolt/production-{release_id}.json"
    previous_dolt_status = f"/run/fishystuff/gitops/dolt/production-{previous_release_id}.json"
    request = f"/var/lib/fishystuff/gitops/admission/requests/production-{release_id}-http-api-meta.json"
    instance = f"/var/lib/fishystuff/gitops/instances/production-{release_id}.json"
    candidate_service = "fishystuff-gitops-candidate-api-production"
    dolt_fixture_service = "fishystuff-gitops-production-dolt-sql-fixture"

    def dump_gitops_debug():
      _, output = machine.execute(f"echo '--- desired ---'; cat {desired} 2>/dev/null || true; echo '--- mgmt log head ---'; head -120 {mgmt_log} 2>/dev/null || true; echo '--- mgmt log tail ---'; tail -240 {mgmt_log} 2>/dev/null || true; echo '--- dolt status ---'; cat {dolt_status} 2>/dev/null || true; echo '--- previous dolt status ---'; cat {previous_dolt_status} 2>/dev/null || true; echo '--- api config ---'; cat {api_config} 2>/dev/null || true; echo '--- api env ---'; cat {api_env} 2>/dev/null || true; echo '--- status ---'; cat {status} 2>/dev/null || true; echo '--- active ---'; cat {active} 2>/dev/null || true; echo '--- admission ---'; cat {admission} 2>/dev/null || true; echo '--- rollback ---'; cat {rollback} 2>/dev/null || true; echo '--- rollback set ---'; cat {rollback_set} 2>/dev/null || true; echo '--- candidate api status ---'; systemctl status {candidate_service} --no-pager 2>&1 || true; echo '--- candidate api journal ---'; journalctl -u {candidate_service} --no-pager -n 200 2>&1 || true; echo '--- dolt fixture journal ---'; journalctl -u {dolt_fixture_service} --no-pager -n 160 2>&1 || true; echo '--- probe curl ---'; curl -v http://127.0.0.1:18092/api/v1/meta 2>&1 || true; echo '--- gitops state tree ---'; find /var/lib/fishystuff/gitops /run/fishystuff/gitops -maxdepth 6 -ls 2>/dev/null || true; echo '--- mgmt process ---'; ps -ef | grep '[m]gmt' || true")
      print(output)

    def wait_for_gitops_file(path):
      try:
        machine.wait_for_file(path, timeout=240)
      except Exception:
        dump_gitops_debug()
        raise

    def wait_for_gitops_command(command, timeout=120):
      try:
        machine.wait_until_succeeds(command, timeout=timeout)
      except Exception:
        dump_gitops_debug()
        raise

    machine.succeed("test -x ${mgmtPackage}/bin/mgmt")
    machine.succeed("test -x ${fishystuffDeployPackage}/bin/fishystuff_deploy")
    machine.succeed("test -x ${fishystuffServerPackage}/bin/fishystuff_server")
    machine.succeed("test -x /run/current-system/sw/bin/fishystuff_deploy")
    machine.succeed("test -x /run/current-system/sw/bin/dolt")
    machine.succeed("systemctl cat fishystuff-gitops-candidate-api-production | grep -Fx 'EnvironmentFile=/var/lib/fishystuff/gitops/api/production.env'")
    machine.succeed("systemctl show fishystuff-gitops-candidate-api-production -p Environment --value | grep -F 'FISHYSTUFF_DATABASE_URL=mysql://root@127.0.0.1:18093/fishystuff'")
    machine.fail("systemctl is-active fishystuff-gitops-candidate-api-production")
    machine.fail("systemctl is-active fishystuff-gitops-production-dolt-sql-fixture")
    machine.succeed("jq -e '.cluster == \"production\" and .mode == \"local-apply\" and .generation == 4 and .environments.production.serve == true and .environments.production.host == \"production-single-host\" and .environments.production.retained_releases == [\"previous-production-release\"] and .environments.production.api_upstream == \"http://127.0.0.1:18092\" and .environments.production.api_service == \"fishystuff-gitops-candidate-api-production\" and .environments.production.admission_probe.kind == \"api_meta\" and .releases[.environments.production.active_release].dolt.materialization == \"fetch_pin\"' ${desiredState}")

    machine.succeed(f"mkdir -p {work}/source {remote} {work}/home")
    machine.succeed(f"env HOME={work}/home dolt config --global --add versioncheck.disabled true || true")
    machine.succeed(f"env HOME={work}/home dolt config --global --add metrics.disabled true || true")
    machine.succeed(textwrap.dedent(f"""
      set -euo pipefail
      export HOME={work}/home
      cd {work}/source
      dolt init --name "FishyStuff GitOps Production VM" --email gitops-production-vm@example.invalid
      dolt sql <<'SQL'
      CREATE TABLE patches (
        patch_id varchar(255) NOT NULL,
        start_ts_utc bigint NOT NULL,
        patch_name varchar(255),
        PRIMARY KEY (patch_id)
      );
      CREATE TABLE map_versions (
        map_version_id varchar(255) NOT NULL,
        name varchar(255),
        is_default bigint,
        PRIMARY KEY (map_version_id)
      );
      CREATE TABLE languagedata (
        lang varchar(32) NOT NULL,
        id bigint NOT NULL,
        format varchar(32) NOT NULL,
        category varchar(255) NOT NULL,
        text varchar(255),
        PRIMARY KEY (lang, id, format, category)
      );
      INSERT INTO patches (patch_id, start_ts_utc, patch_name)
        VALUES ('gitops-production-previous-patch', 1600000000, 'GitOps Production Previous Patch');
      INSERT INTO map_versions (map_version_id, name, is_default)
        VALUES ('gitops-production-previous-map', 'GitOps Production Previous Map', 1);
      INSERT INTO languagedata (lang, id, format, category, text)
        VALUES ('en', 1, 'A', 'gitops', 'GitOps production previous language row');
      SQL
      dolt add .
      dolt commit -m previous-production-api-meta-fixture
      dolt remote add origin file://{remote}
      dolt push origin main
      dolt sql -r csv -q "select dolt_hashof('main') as hash" | tail -n 1 > {work}/commit-previous
      dolt sql <<'SQL'
      DELETE FROM patches;
      DELETE FROM map_versions;
      DELETE FROM languagedata;
      INSERT INTO patches (patch_id, start_ts_utc, patch_name)
        VALUES ('gitops-production-api-meta-patch', 1700000000, 'GitOps Production API Meta Patch');
      INSERT INTO map_versions (map_version_id, name, is_default)
        VALUES ('gitops-production-map', 'GitOps Production Map', 1);
      INSERT INTO languagedata (lang, id, format, category, text)
        VALUES ('en', 1, 'A', 'gitops', 'GitOps production fixture language row');
      SQL
      dolt add .
      dolt commit -m active-production-api-meta-fixture
      dolt push origin main
      dolt sql -r csv -q "select dolt_hashof('main') as hash" | tail -n 1 > {work}/commit-active
    """))
    machine.succeed(textwrap.dedent(f"""
      set -euo pipefail
      active_commit="$(cat {work}/commit-active)"
      previous_commit="$(cat {work}/commit-previous)"
      cat > {desired} <<EOF
      {{
        "cluster": "production",
        "generation": 4,
        "mode": "local-apply",
        "hosts": {{
          "production-single-host": {{
            "enabled": true,
            "role": "single-site",
            "hostname": "production-single-host"
          }}
        }},
        "releases": {{
          "{release_id}": {{
            "generation": 4,
            "git_rev": "production-local-apply-api-meta",
            "dolt_commit": "$active_commit",
            "closures": {{
              "api": {{"enabled": true, "store_path": "${apiArtifact}", "gcroot_path": "/var/lib/fishystuff/gitops/gcroots/{release_id}/api"}},
              "site": {{"enabled": true, "store_path": "${siteArtifact}", "gcroot_path": "/var/lib/fishystuff/gitops/gcroots/{release_id}/site"}},
              "cdn_runtime": {{"enabled": true, "store_path": "${cdnRuntimeArtifact}", "gcroot_path": "/var/lib/fishystuff/gitops/gcroots/{release_id}/cdn-runtime"}},
              "dolt_service": {{"enabled": true, "store_path": "${doltServiceArtifact}", "gcroot_path": "/var/lib/fishystuff/gitops/gcroots/{release_id}/dolt-service"}}
            }},
            "dolt": {{
              "repository": "fishystuff/fishystuff",
              "commit": "$active_commit",
              "branch_context": "main",
              "mode": "read_only",
              "materialization": "fetch_pin",
              "remote_url": "file://{remote}",
              "cache_dir": "{cache}",
              "release_ref": "{release_ref}"
            }}
          }},
          "{previous_release_id}": {{
            "generation": 3,
            "git_rev": "previous-production-local-apply-api-meta",
            "dolt_commit": "$previous_commit",
            "closures": {{
              "api": {{"enabled": true, "store_path": "${previousApiArtifact}", "gcroot_path": "/var/lib/fishystuff/gitops/gcroots/{previous_release_id}/api"}},
              "site": {{"enabled": true, "store_path": "${previousSiteArtifact}", "gcroot_path": "/var/lib/fishystuff/gitops/gcroots/{previous_release_id}/site"}},
              "cdn_runtime": {{"enabled": true, "store_path": "${previousCdnRuntimeArtifact}", "gcroot_path": "/var/lib/fishystuff/gitops/gcroots/{previous_release_id}/cdn-runtime"}},
              "dolt_service": {{"enabled": true, "store_path": "${previousDoltServiceArtifact}", "gcroot_path": "/var/lib/fishystuff/gitops/gcroots/{previous_release_id}/dolt-service"}}
            }},
            "dolt": {{
              "repository": "fishystuff/fishystuff",
              "commit": "$previous_commit",
              "branch_context": "main",
              "mode": "read_only",
              "materialization": "fetch_pin",
              "remote_url": "file://{remote}",
              "cache_dir": "{cache}",
              "release_ref": "{previous_release_ref}"
            }}
          }}
        }},
        "environments": {{
          "production": {{
            "enabled": true,
            "strategy": "single_active",
            "host": "production-single-host",
            "active_release": "{release_id}",
            "retained_releases": ["{previous_release_id}"],
            "api_upstream": "http://127.0.0.1:18092",
            "api_service": "fishystuff-gitops-candidate-api-production",
            "admission_probe": {{
              "kind": "api_meta",
              "probe_name": "api-meta",
              "url": "http://127.0.0.1:18092/api/v1/meta",
              "expected_status": 200,
              "timeout_ms": 2000
            }},
            "serve": true
          }}
        }}
      }}
      EOF
    """))

    active_commit = machine.succeed(f"cat {work}/commit-active").strip()
    previous_commit = machine.succeed(f"cat {work}/commit-previous").strip()
    expected_release_identity = f"release={release_id};generation=4;git_rev=production-local-apply-api-meta;dolt_commit={active_commit};dolt_repository=fishystuff/fishystuff;dolt_branch_context=main;dolt_mode=read_only;api=${apiArtifact};site=${siteArtifact};cdn_runtime=${cdnRuntimeArtifact};dolt_service=${doltServiceArtifact}"

    machine.succeed(f"jq -e '.cluster == \"production\" and .mode == \"local-apply\" and .generation == 4 and .environments.production.serve == true and .environments.production.active_release == \"{release_id}\" and .environments.production.retained_releases == [\"{previous_release_id}\"] and .releases.\"{release_id}\".dolt.materialization == \"fetch_pin\" and .releases.\"{previous_release_id}\".dolt.materialization == \"fetch_pin\"' {desired}")
    machine.succeed(f"env FISHYSTUFF_GITOPS_ENABLE_LOCAL_APPLY=1 FISHYSTUFF_GITOPS_STATE_FILE={desired} ${mgmtPackage}/bin/mgmt run --hostname production-single-host --tmp-prefix --no-pgp --client-urls=http://127.0.0.1:2379 --server-urls=http://127.0.0.1:2380 --advertise-client-urls=http://127.0.0.1:2379 --advertise-server-urls=http://127.0.0.1:2380 --converged-timeout=-1 lang ${gitopsSrc}/main.mcl >{mgmt_log} 2>&1 & echo $! >{mgmt_pid}")

    wait_for_gitops_file(dolt_status)
    wait_for_gitops_file(previous_dolt_status)
    wait_for_gitops_file(api_config)
    wait_for_gitops_file(api_env)
    wait_for_gitops_command("systemctl is-active fishystuff-gitops-production-dolt-sql-fixture", timeout=180)
    wait_for_gitops_command("systemctl is-active fishystuff-gitops-candidate-api-production", timeout=180)
    wait_for_gitops_command(f"curl -fsS http://127.0.0.1:18092/api/v1/meta | jq -e '.release_id == \"{release_id}\" and .release_identity == \"{expected_release_identity}\" and .dolt_commit == \"{active_commit}\" and .default_patch.patch_id == \"gitops-production-api-meta-patch\" and .map_versions[0].map_version_id == \"gitops-production-map\" and (.data_languages | index(\"en\"))'", timeout=180)
    wait_for_gitops_file(request)
    wait_for_gitops_file(admission)
    wait_for_gitops_file(status)
    wait_for_gitops_file(active)
    wait_for_gitops_file(route)
    wait_for_gitops_file(instance)
    wait_for_gitops_file(rollback)
    wait_for_gitops_file(rollback_set)
    wait_for_gitops_file(previous_rollback_member)

    machine.succeed(f"jq -e --arg commit \"{active_commit}\" '.state == \"pinned\" and .requested_commit == $commit and .verified_commit == $commit and .release_id == \"{release_id}\" and .cache_dir == \"{cache}\" and .release_ref == \"{release_ref}\"' {dolt_status}")
    machine.succeed(f"jq -e --arg commit \"{previous_commit}\" '.state == \"pinned\" and .requested_commit == $commit and .verified_commit == $commit and .release_id == \"{previous_release_id}\" and .cache_dir == \"{cache}\" and .release_ref == \"{previous_release_ref}\"' {previous_dolt_status}")
    machine.succeed(f"cd {cache} && test \"$(dolt sql -r csv -q \"select dolt_hashof('{release_ref}') as hash\" | tail -n 1)\" = \"{active_commit}\"")
    machine.succeed(f"cd {cache} && test \"$(dolt sql -r csv -q \"select dolt_hashof('{previous_release_ref}') as hash\" | tail -n 1)\" = \"{previous_commit}\"")
    machine.succeed(f"jq -e --arg commit \"{active_commit}\" '.desired_generation == 4 and .environment == \"production\" and .host == \"production-single-host\" and .release_id == \"{release_id}\" and .release_identity == \"{expected_release_identity}\" and .api_bundle == \"${apiArtifact}\" and .api_upstream == \"http://127.0.0.1:18092\" and .service_name == \"fishystuff-gitops-candidate-api-production\" and .dolt_commit == $commit and .dolt_release_ref == \"{release_ref}\" and .state == \"candidate_api_config\"' {api_config}")
    machine.succeed(f"grep -Fx \"FISHYSTUFF_RELEASE_ID='{release_id}'\" {api_env}")
    machine.succeed(f"grep -Fx \"FISHYSTUFF_RELEASE_IDENTITY='{expected_release_identity}'\" {api_env}")
    machine.succeed(f"grep -Fx \"FISHYSTUFF_DOLT_COMMIT='{active_commit}'\" {api_env}")
    machine.succeed(f"grep -Fx \"FISHYSTUFF_DEFAULT_DOLT_REF='{release_ref}'\" {api_env}")
    machine.succeed(f"grep -Fx \"FISHYSTUFF_DEPLOYMENT_ENVIRONMENT='production'\" {api_env}")
    machine.succeed(f"jq -e --arg commit \"{active_commit}\" '.environment == \"production\" and .host == \"production-single-host\" and .release_id == \"{release_id}\" and .probe_name == \"api-meta\" and .url == \"http://127.0.0.1:18092/api/v1/meta\" and .expected_status == 200 and .timeout_ms == 2000 and .expected_scalars.\"/release_id\" == \"{release_id}\" and .expected_scalars.\"/release_identity\" == \"{expected_release_identity}\" and .expected_scalars.\"/dolt_commit\" == $commit' {request}")
    machine.succeed(f"jq -e --arg commit \"{active_commit}\" '.environment == \"production\" and .host == \"production-single-host\" and .release_id == \"{release_id}\" and .release_identity == \"{expected_release_identity}\" and .probe_name == \"api-meta\" and .url == \"http://127.0.0.1:18092/api/v1/meta\" and .observed_status == 200 and .scalars.\"/release_id\" == \"{release_id}\" and .scalars.\"/release_identity\" == \"{expected_release_identity}\" and .scalars.\"/dolt_commit\" == $commit and .admission_state == \"passed_fixture\" and .probe == \"http-json-scalars\"' {admission}")
    machine.succeed(f"jq -e --arg commit \"{active_commit}\" '.desired_generation == 4 and .release_id == \"{release_id}\" and .release_identity == \"{expected_release_identity}\" and .environment == \"production\" and .host == \"production-single-host\" and .phase == \"served\" and .admission_state == \"passed_fixture\" and .dolt_commit == $commit and .dolt_materialization == \"fetch_pin\" and .dolt_cache_dir == \"{cache}\" and .dolt_release_ref == \"{release_ref}\" and .served == true and .retained_release_ids == [\"previous-production-release\"] and .retained_dolt_status_paths == [\"{previous_dolt_status}\"] and .rollback_available == true and .rollback_primary_release_id == \"previous-production-release\" and .rollback_retained_count == 1' {status}")
    machine.succeed(f"jq -e '.desired_generation == 4 and .environment == \"production\" and .host == \"production-single-host\" and .release_id == \"{release_id}\" and .release_identity == \"{expected_release_identity}\" and .api_upstream == \"http://127.0.0.1:18092\" and .site_link == \"/var/lib/fishystuff/gitops/served/production/site\" and .cdn_link == \"/var/lib/fishystuff/gitops/served/production/cdn\" and .retained_release_ids == [\"previous-production-release\"] and .retained_dolt_status_paths == [\"{previous_dolt_status}\"] and .admission_state == \"passed_fixture\" and .served == true and .route_state == \"selected_local_symlinks\"' {active}")
    machine.succeed(f"jq -e '.desired_generation == 4 and .environment == \"production\" and .host == \"production-single-host\" and .release_id == \"{release_id}\" and .release_identity == \"{expected_release_identity}\" and .api_upstream == \"http://127.0.0.1:18092\" and .active_path == \"/var/lib/fishystuff/gitops/active/production.json\" and .site_root == \"/var/lib/fishystuff/gitops/served/production/site\" and .cdn_root == \"/var/lib/fishystuff/gitops/served/production/cdn\" and .served == true and .state == \"selected_local_route\"' {route}")
    machine.succeed(f"jq -e --arg commit \"{active_commit}\" '.desired_generation == 4 and .release_id == \"{release_id}\" and .release_identity == \"{expected_release_identity}\" and .api_upstream == \"http://127.0.0.1:18092\" and .serve_requested == true and .dolt_branch_context == \"main\" and .dolt_commit == $commit and .dolt_materialization == \"fetch_pin\" and .dolt_cache_dir == \"{cache}\" and .dolt_release_ref == \"{release_ref}\"' {instance}")
    machine.succeed(f"jq -e --arg commit \"{previous_commit}\" '.desired_generation == 4 and .current_release_id == \"{release_id}\" and .rollback_release_id == \"previous-production-release\" and .rollback_api_bundle == \"${previousApiArtifact}\" and .rollback_dolt_service_bundle == \"${previousDoltServiceArtifact}\" and .rollback_site_content == \"${previousSiteArtifact}\" and .rollback_cdn_runtime_content == \"${previousCdnRuntimeArtifact}\" and .rollback_dolt_commit == $commit and .rollback_dolt_materialization == \"fetch_pin\" and .rollback_dolt_cache_dir == \"{cache}\" and .rollback_dolt_release_ref == \"{previous_release_ref}\" and .rollback_available == true' {rollback}")
    machine.succeed(f"jq -e '.desired_generation == 4 and .current_release_id == \"{release_id}\" and .retained_release_count == 1 and .retained_release_ids == [\"previous-production-release\"] and .retained_release_document_paths == [\"{previous_rollback_member}\"] and .rollback_set_available == true' {rollback_set}")
    machine.succeed(f"jq -e --arg commit \"{previous_commit}\" '.desired_generation == 4 and .current_release_id == \"{release_id}\" and .release_id == \"{previous_release_id}\" and .dolt_commit == $commit and .dolt_materialization == \"fetch_pin\" and .dolt_cache_dir == \"{cache}\" and .dolt_release_ref == \"{previous_release_ref}\" and .dolt_status_path == \"{previous_dolt_status}\" and .rollback_member_state == \"retained_hot_release\"' {previous_rollback_member}")

    machine.succeed("test \"$(readlink /var/lib/fishystuff/gitops/served/production/site)\" = \"${siteArtifact}\"")
    machine.succeed("test \"$(readlink /var/lib/fishystuff/gitops/served/production/cdn)\" = \"${cdnRuntimeArtifact}\"")
    machine.succeed("jq -e '.schema_version == 1 and .current_root == \"${cdnRuntimeCurrentArtifact}\" and .retained_root_count == 1' ${cdnRuntimeArtifact}/cdn-serving-manifest.json")
    machine.succeed("jq -e '.current_root == \"${previousCdnRuntimeCurrentArtifact}\" and .retained_root_count == 0' ${previousCdnRuntimeArtifact}/cdn-serving-manifest.json")
    machine.succeed(f"${fishystuffDeployPackage}/bin/fishystuff_deploy gitops check-served --status {status} --active {active} --rollback-set {rollback_set} --rollback {rollback} --environment production --host production-single-host --release-id {release_id}")
    machine.succeed(f"${fishystuffDeployPackage}/bin/fishystuff_deploy gitops summary-served --status {status} --active {active} --rollback-set {rollback_set} --rollback {rollback} --environment production --host production-single-host --release-id {release_id} | grep -Fx 'served_release: {release_id}'")
    machine.succeed(f"${fishystuffDeployPackage}/bin/fishystuff_deploy gitops summary-served --status {status} --active {active} --rollback-set {rollback_set} --rollback {rollback} --environment production --host production-single-host --release-id {release_id} | grep -Fx 'retained_rollback_releases: previous-production-release'")

    machine.succeed(f"kill $(cat {mgmt_pid}) || true")

    machine.fail("systemctl is-active fishystuff-api.service")
    machine.fail("systemctl is-active fishystuff-dolt.service")
    machine.fail("systemctl is-active fishystuff-edge.service")
    machine.succeed("test ! -e /srv/fishystuff")
    machine.succeed("test ! -e /var/lib/fishystuff/gitops-test")
    machine.succeed("test ! -e /run/fishystuff/gitops-test")
    machine.succeed("test ! -e /tmp/fishystuff-gitops-test")
    machine.succeed("test ! -e /var/lib/fishystuff/mgmt")
    machine.succeed("! find /var/lib/fishystuff/gitops /run/fishystuff/gitops -type f -print0 | xargs -0 grep -E 'beta\\.fishystuff\\.fish|cloudflare|hcloud|ssh '")
  '';
}
