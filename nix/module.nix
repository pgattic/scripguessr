{
  defaultPackage ? null,
}:

{
  config,
  lib,
  pkgs,
  ...
}:

let
  cfg = config.services.scripguessr;
in
{
  options.services.scripguessr = {
    enable = lib.mkEnableOption "ScripGuessr";

    package = lib.mkOption {
      type = lib.types.package;
      default =
        if defaultPackage != null then
          defaultPackage
        else
          throw "services.scripguessr.package must be set when importing nix/module.nix directly";
      defaultText = lib.literalExpression "self.packages.\${pkgs.system}.default";
      description = "ScripGuessr package to run.";
    };

    port = lib.mkOption {
      type = lib.types.port;
      default = 8087;
      description = "Port for the local ScripGuessr service to bind.";
    };

    gameTtlSeconds = lib.mkOption {
      type = lib.types.ints.positive;
      default = 21600;
      description = "Seconds to keep inactive games before pruning.";
    };
  };

  config = lib.mkIf cfg.enable {
    services.postgresql = {
      enable = true;
      ensureDatabases = [ "scripguessr" ];
      ensureUsers = [
        {
          name = "scripguessr";
          ensureDBOwnership = true;
        }
      ];
    };

    users.users.scripguessr = {
      isSystemUser = true;
      group = "scripguessr";
    };
    users.groups.scripguessr = { };

    systemd.services.scripguessr = {
      description = "ScripGuessr web service";
      after = [
        "network.target"
        "postgresql.service"
        "postgresql-setup.service"
      ];
      requires = [
        "postgresql.service"
        "postgresql-setup.service"
      ];
      wantedBy = [ "multi-user.target" ];
      environment = {
        PORT = toString cfg.port;
        SCRIPGUESSR_GAME_TTL_SECONDS = toString cfg.gameTtlSeconds;
        SCRIPGUESSR_STATIC_DIR = "${cfg.package}/share/scripguessr/public";
        DATABASE_URL = "postgresql://scripguessr@localhost/scripguessr?host=/run/postgresql";
      };
      serviceConfig = {
        ExecStart = lib.getExe cfg.package;
        User = "scripguessr";
        Group = "scripguessr";
        Restart = "on-failure";
        RestartSec = "5s";
        NoNewPrivileges = true;
        PrivateTmp = true;
        ProtectHome = true;
        ProtectSystem = "strict";
      };
    };
  };
}
