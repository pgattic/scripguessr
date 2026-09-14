# Deploying ScripGuessr on NixOS

ScripGuessr ships a NixOS module as `nixosModules.default`. The module builds the
Dioxus web app and backend server, runs the app on localhost, and leaves your
reverse proxy configuration in your host config.

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

The server exposes a lightweight health endpoint:

```text
GET /healthz
```

It returns `200 OK` with `ok` in the response body. Point uptime checks or reverse
proxy health checks at `http://127.0.0.1:8087/healthz`.
