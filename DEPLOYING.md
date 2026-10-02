# Deploying ScripGuessr on NixOS

ScripGuessr ships a NixOS module as `nixosModules.default`. The module builds the
Dioxus web app and backend server, provisions a local PostgreSQL database, runs
the app on localhost, and leaves your reverse proxy configuration in your host
config.

## Example

```nix
{
  inputs.scripguessr.url = "github:your-user/scripguessr";

  outputs =
    { nixpkgs, scripguessr, ... }:
    {
      nixosConfigurations.your-server = nixpkgs.lib.nixosSystem {
        system = "x86_64-linux";
        modules = [
          scripguessr.nixosModules.default
          {
            services.scripguessr = {
              enable = true;
              port = 8087;
            };

            services.nginx = {
              enable = true;
              virtualHosts."scripguessr.example.com" = {
                enableACME = true;
                forceSSL = true;
                locations."/".proxyPass = "http://127.0.0.1:8087";
              };
            };
          }
        ];
      };
    };
}
```

If another reverse proxy already owns TLS, point it at
`http://127.0.0.1:8087`.

## Health Checks

The server exposes liveness and database-readiness endpoints:

```text
GET /healthz
GET /readyz
```

Use `/healthz` for process liveness and `/readyz` when the check should also
verify PostgreSQL connectivity.
