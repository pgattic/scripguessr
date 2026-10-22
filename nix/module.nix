{ self }:
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
      default = self.packages.${pkgs.stdenv.hostPlatform.system}.default;
      description = "ScripGuessr Phoenix release.";
    };
    hostName = lib.mkOption {
      type = lib.types.str;
      description = "Public HTTPS hostname.";
    };
    port = lib.mkOption {
      type = lib.types.port;
      default = 4000;
    };
    environmentFile = lib.mkOption {
      type = lib.types.str;
      description = "Runtime file containing SECRET_KEY_BASE; keep it outside the Nix store.";
    };
    configureNginx = lib.mkOption {
      type = lib.types.bool;
      default = true;
    };
  };
  config = lib.mkIf cfg.enable {
    users.users.scripguessr = {
      isSystemUser = true;
      group = "scripguessr";
    };
    users.groups.scripguessr = { };
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
    systemd.services.scripguessr = {
      description = "ScripGuessr Phoenix quiz";
      wantedBy = [ "multi-user.target" ];
      after = [
        "network.target"
        "postgresql.service"
        "postgresql-setup.service"
      ];
      requires = [
        "postgresql.service"
        "postgresql-setup.service"
      ];
      environment = {
        PHX_SERVER = "true";
        PHX_HOST = cfg.hostName;
        PORT = toString cfg.port;
        PGHOST = "/run/postgresql";
        PGPORT = toString config.services.postgresql.settings.port;
        PGUSER = "scripguessr";
        PGDATABASE = "scripguessr";
        RELEASE_DISTRIBUTION = "none";
        RELEASE_COOKIE = "unused-with-distribution-disabled";
        RELEASE_TMP = "/run/scripguessr";
      };
      serviceConfig = {
        User = "scripguessr";
        Group = "scripguessr";
        EnvironmentFile = cfg.environmentFile;
        ExecStartPre = "${cfg.package}/bin/scripguessr eval Scripguessr.Release.migrate";
        ExecStart = "${cfg.package}/bin/scripguessr start";
        Restart = "on-failure";
        RestartSec = "5s";
        RuntimeDirectory = "scripguessr";
        StateDirectory = "scripguessr";
        WorkingDirectory = "/var/lib/scripguessr";
        UMask = "0077";
        NoNewPrivileges = true;
        PrivateTmp = true;
        ProtectSystem = "strict";
        ProtectHome = true;
      };
    };
    services.nginx = lib.mkIf cfg.configureNginx {
      enable = true;
      recommendedProxySettings = true;
      virtualHosts.${cfg.hostName} = {
        enableACME = true;
        forceSSL = true;
        locations."/" = {
          proxyPass = "http://127.0.0.1:${toString cfg.port}";
          proxyWebsockets = true;
        };
      };
    };
  };
}
