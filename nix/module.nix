{
  config,
  lib,
  pkgs,
  ...
}:

let
  cfg = config.services.scripguessr;
  bindAddress = "127.0.0.1";
in
{
  options.services.scripguessr = {
    enable = lib.mkEnableOption "ScripGuessr";

    package = lib.mkOption {
      type = lib.types.package;
      default = pkgs.callPackage ./package.nix { };
      defaultText = lib.literalExpression "pkgs.callPackage ./package.nix { }";
      description = "ScripGuessr static site package to serve.";
    };

    port = lib.mkOption {
      type = lib.types.port;
      default = 8087;
      description = "Port for the local ScripGuessr service to bind.";
    };
  };

  config = lib.mkIf cfg.enable {
    systemd.services.scripguessr = {
      description = "ScripGuessr static web service";
      after = [ "network.target" ];
      wantedBy = [ "multi-user.target" ];
      serviceConfig = {
        ExecStart = "${lib.getExe pkgs.static-web-server} --host ${bindAddress} --port ${toString cfg.port} --root ${cfg.package}";
        DynamicUser = true;
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
