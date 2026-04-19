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

          case "''${1:-}" in
            clone)
              printf '%s\n' "$@" > /tmp/fishystuff-dolt-clone-args
              target=""
              for arg in "$@"; do
                target="$arg"
              done
              mkdir -p "$target/.dolt"
              ;;
            config)
              printf '%s\n' "$@" >> /tmp/fishystuff-dolt-config-args
              if [ "''${3:-}" = "--get" ]; then
                exit 1
              fi
              ;;
            sql-server)
              printf '%s\n' "$@" > /tmp/fishystuff-dolt-args
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
    machine.succeed("systemctl show fishystuff-dolt.service -p DynamicUser --value | grep '^yes$'")
    machine.succeed("systemctl show fishystuff-api.service -p EnvironmentFiles --value | grep '/run/fishystuff/api/env'")
    machine.succeed("systemctl cat fishystuff-dolt.service | grep '^StateDirectory=fishystuff/dolt$'")
    machine.succeed("systemctl show fishystuff-dolt.service -p Environment --value | grep 'HOME=/var/lib/fishystuff/dolt'")
    machine.succeed("test -d /var/lib/fishystuff/dolt/fishystuff/.dolt")
  '';
}
