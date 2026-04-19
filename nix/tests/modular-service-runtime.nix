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
          printf '%s\n' "$@" > /tmp/fishystuff-dolt-args
          trap 'exit 0' TERM INT
          while true; do
            sleep 3600
          done
        '';
      };
    in
    {
      system.stateVersion = "25.11";

      users.groups.fishystuff-api = { };
      users.users.fishystuff-api = {
        isSystemUser = true;
        group = "fishystuff-api";
      };

      users.groups.fishystuff-dolt = { };
      users.users.fishystuff-dolt = {
        isSystemUser = true;
        group = "fishystuff-dolt";
      };

      systemd.tmpfiles.rules = [
        "d /var/lib/fishystuff/dolt 0750 fishystuff-dolt fishystuff-dolt -"
        "d /var/lib/fishystuff/dolt/.doltcfg 0750 fishystuff-dolt fishystuff-dolt -"
      ];

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
    machine.succeed("systemctl show fishystuff-dolt.service -p ExecStart --value | grep -- '--config'")
    machine.succeed("systemctl show fishystuff-dolt.service -p ExecStart --value | grep -- 'sql-server'")
    machine.succeed("systemctl show fishystuff-api.service -p EnvironmentFiles --value | grep '/run/fishystuff/api/env'")
    machine.succeed("systemctl show fishystuff-dolt.service -p EnvironmentFiles --value | grep '/run/fishystuff/dolt/env'")
    machine.succeed("systemctl show fishystuff-dolt.service -p Environment --value | grep 'HOME=/var/lib/fishystuff/dolt'")
  '';
}
