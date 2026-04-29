{ pkgs, serviceModules }:
pkgs.testers.runNixOSTest {
  name = "fishystuff-modular-service-runtime";

  nodes.machine =
    { pkgs, ... }:
    let
      fakeApiPackage = pkgs.writeShellApplication {
        name = "fishystuff_server";
        text = ''
          printf '%s\n' "$@" > /tmp/fishystuff-api-args
          trap 'exit 0' TERM INT
          while true; do
            sleep 3600
          done
        '';
      };

      fakeDoltPackage = pkgs.writeShellApplication {
        name = "dolt";
        text = ''
          set -euo pipefail

          subcommand=""
          for arg in "$@"; do
            case "$arg" in
              clone|config|fetch|reset|sql|sql-server)
                subcommand="$arg"
                break
                ;;
            esac
          done

          case "$subcommand" in
            clone)
              printf '%s\n' "$@" > /var/lib/fishystuff/dolt/fishystuff-dolt-clone-args
              target=""
              for arg in "$@"; do
                target="$arg"
              done
              mkdir -p "$target/.dolt"
              ;;
            config)
              printf '%s\n' "$@" >> /var/lib/fishystuff/dolt/fishystuff-dolt-config-args
              if [ "''${3:-}" = "--get" ]; then
                exit 1
              fi
              ;;
            fetch)
              printf '%s\n' "$@" > /var/lib/fishystuff/dolt/fishystuff-dolt-fetch-args
              ;;
            reset)
              printf '%s\n' "$@" > /var/lib/fishystuff/dolt/fishystuff-dolt-reset-args
              ;;
            sql)
              printf '%s\n' "$@" >> /var/lib/fishystuff/dolt/fishystuff-dolt-sql-args
              cat >> /var/lib/fishystuff/dolt/fishystuff-dolt-sql-stdin
              case "$*" in
                *"SELECT @@global.read_only"*)
                  printf '| 0\n'
                  ;;
              esac
              ;;
            sql-server)
              printf '%s\n' "$@" > /var/lib/fishystuff/dolt/fishystuff-dolt-args
              pwd > /var/lib/fishystuff/dolt/fishystuff-dolt-pwd
              printf '%s\n' "HOME=$HOME" > /var/lib/fishystuff/dolt/fishystuff-dolt-env
              trap 'exit 0' TERM INT
              while true; do
                sleep 3600
              done
              ;;
            *)
              printf 'unexpected fake dolt invocation: %s\n' "$*" >&2
              exit 1
              ;;
          esac
        '';
      };
    in
    {
      system.stateVersion = "25.11";

      system.services.fishystuff-api = {
        imports = [ serviceModules.api ];
        fishystuff.api.package = fakeApiPackage;
      };

      system.services.fishystuff-dolt = {
        imports = [ serviceModules.dolt ];
        fishystuff.dolt.package = fakeDoltPackage;
        fishystuff.dolt.runtimeEnvFile = "/run/fishystuff/api/env";
      };
    };

  testScript = ''
    start_all()

    machine.wait_for_unit("fishystuff-api.service")
    machine.wait_for_unit("fishystuff-dolt.service")

    machine.succeed("systemctl is-active fishystuff-api.service")
    machine.succeed("systemctl is-active fishystuff-dolt.service")
    machine.succeed("test -f /etc/system-services/fishystuff-api/config.toml")
    machine.succeed("test -f /etc/system-services/fishystuff-dolt/sql-server.yaml")
    machine.succeed("systemctl show fishystuff-api.service -p ExecStart --value | grep -- '--config'")
    machine.succeed("systemctl show fishystuff-api.service -p DynamicUser --value | grep '^yes$'")
    machine.succeed("systemctl show fishystuff-dolt.service -p ExecStart --value | grep -- 'fishystuff-dolt-start'")
    machine.succeed("systemctl show fishystuff-dolt.service -p ExecReload --value | grep -- 'fishystuff-dolt-refresh'")
    machine.succeed("systemctl show fishystuff-dolt.service -p DynamicUser --value | grep '^yes$'")
    machine.succeed("systemctl show fishystuff-api.service -p EnvironmentFiles --value | grep '/run/fishystuff/api/env'")
    machine.succeed("systemctl show fishystuff-dolt.service -p EnvironmentFiles --value | grep '/run/fishystuff/api/env'")
    machine.succeed("systemctl cat fishystuff-dolt.service | grep '^StateDirectory=fishystuff/dolt$'")
    machine.succeed("systemctl show fishystuff-dolt.service -p Environment --value | grep 'HOME=/var/lib/fishystuff/dolt/home'")
    machine.succeed("test -d /var/lib/fishystuff/dolt/fishystuff/.dolt")
    machine.succeed("grep '^/var/lib/fishystuff/dolt/fishystuff$' /var/lib/fishystuff/dolt/fishystuff-dolt-pwd")
    machine.succeed("grep '^HOME=/var/lib/fishystuff/dolt/home$' /var/lib/fishystuff/dolt/fishystuff-dolt-env")
    machine.succeed("systemctl reload fishystuff-dolt.service")
    machine.succeed("grep 'SET GLOBAL read_only = 0' /var/lib/fishystuff/dolt/fishystuff-dolt-sql-args")
    machine.succeed("grep \"CALL DOLT_FETCH('origin')\" /var/lib/fishystuff/dolt/fishystuff-dolt-sql-args")
    machine.succeed("grep \"CALL DOLT_RESET('--hard', 'origin/beta')\" /var/lib/fishystuff/dolt/fishystuff-dolt-sql-args")
    machine.succeed("grep 'SET GLOBAL read_only = 1' /var/lib/fishystuff/dolt/fishystuff-dolt-sql-args")
    machine.succeed("mkdir -p /run/fishystuff/api /var/lib/fishystuff/dolt-snapshot/.dolt/noms")
    machine.succeed("printf 'manifest\\n' > /var/lib/fishystuff/dolt-snapshot/.dolt/noms/manifest")
    machine.succeed("chmod -R a-w /var/lib/fishystuff/dolt-snapshot/.dolt")
    machine.succeed("printf 'FISHYSTUFF_DOLT_REPO_SNAPSHOT=/var/lib/fishystuff/dolt-snapshot\\n' > /run/fishystuff/api/env")
    machine.succeed("systemctl restart fishystuff-dolt.service")
    machine.wait_for_unit("fishystuff-dolt.service")
    machine.succeed("pid=$(systemctl show fishystuff-dolt.service -p MainPID --value); tr '\\0' '\\n' < /proc/$pid/environ | grep '^FISHYSTUFF_DOLT_REPO_SNAPSHOT=/var/lib/fishystuff/dolt-snapshot$'")
    machine.wait_until_succeeds("test -d /var/lib/fishystuff/dolt/fishystuff/.dolt/noms")
    machine.wait_until_succeeds("grep '^/var/lib/fishystuff/dolt-snapshot$' /var/lib/fishystuff/dolt/fishystuff/.fishystuff-dolt-snapshot-source")
    machine.succeed("stat -c '%A' /var/lib/fishystuff/dolt/fishystuff/.dolt | grep '^drwx'")
    machine.succeed("stat -c '%A' /var/lib/fishystuff/dolt/fishystuff/.dolt/noms | grep '^drwx'")
    machine.succeed("test \"$(stat -c '%a' /var/lib/fishystuff/dolt/fishystuff/.dolt/noms/manifest)\" = 600")
    machine.succeed("test \"$(stat -c '%u' /var/lib/fishystuff/dolt/fishystuff/.dolt/noms/manifest)\" != 0")
    machine.succeed("grep '^/var/lib/fishystuff/dolt/fishystuff$' /var/lib/fishystuff/dolt/fishystuff-dolt-pwd")
    machine.succeed("rm -f /var/lib/fishystuff/dolt/fishystuff-dolt-sql-args")
    machine.succeed("systemctl reload fishystuff-dolt.service")
    machine.succeed("test ! -e /var/lib/fishystuff/dolt/fishystuff-dolt-sql-args")
    machine.succeed("systemctl stop fishystuff-dolt.service")
    machine.succeed("mkdir -p /var/lib/fishystuff/dolt/.dolt")
    machine.succeed("start_cmd=$(systemctl cat fishystuff-dolt.service | sed -n 's/^ExecStart=//p' | tr -d '\"'); test -n \"$start_cmd\"; set +e; $start_cmd 2>/tmp/fishystuff-dolt-start.err; status=$?; set -e; test \"$status\" = 64")
    machine.succeed("grep 'refusing to start: found top-level /var/lib/fishystuff/dolt/.dolt' /tmp/fishystuff-dolt-start.err")
  '';
}
