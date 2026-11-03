# NixOS deployment

The flake exports `nixosModules.default` and a production Phoenix release as
`packages.<system>.default`. The release contains Erlang and compiled assets;
production does not need Mix, a compiler, or Node. Nix fetches a pinned corpus
release archive during the build and bundles its read-only SQLite database,
license, and credits. Games and the Atlas need no runtime corpus network access.

## Host configuration

Add this repository as a flake input using its actual GitHub owner, then import
the module into your server's NixOS configuration:

```nix
{
  inputs.scripguessr.url = "github:YOUR-OWNER/scripguessr";

  outputs = { nixpkgs, scripguessr, ... }: {
    nixosConfigurations.your-server = nixpkgs.lib.nixosSystem {
      system = "x86_64-linux"; # Or aarch64-linux, matching the host.
      modules = [
        scripguessr.nixosModules.default
        {
          services.scripguessr = {
            enable = true;
            hostName = "game.example.com";
            environmentFile = "/var/lib/secrets/scripguessr.env";
          };
          security.acme = {
            acceptTerms = true;
            defaults.email = "you@example.com";
          };
          networking.firewall.allowedTCPPorts = [ 80 443 ];
        }
      ];
    };
  };
}
```

Point the hostname at the server before requesting its certificate. The module
provisions PostgreSQL and its database/user, applies migrations before starting
the app, and configures nginx with HTTPS and WebSocket forwarding. The app
listens on localhost port 4000 by default; `services.scripguessr.port` overrides it.
The module uses the host's PostgreSQL package and cluster, including when the
host already runs PostgreSQL. Follow the host's normal database version and
backup policy.

If you already manage the proxy, set `configureNginx = false`. Forward HTTPS
traffic to the configured localhost port, preserve the public Host header, set
`X-Forwarded-Proto: https`, overwrite `X-Real-IP` with the client's actual IP, and
enable WebSocket upgrades for `/live/websocket`. The application trusts that IP
header only from a loopback proxy, for authentication request limiting.

## Runtime secret

On the target host, create a root-readable environment file at the configured
path containing:

```text
SECRET_KEY_BASE=<a random secret generated with mix phx.gen.secret>
```

Generate the value with `nix develop -c mix phx.gen.secret` after fetching the Mix
dependencies, and place it in the host's secret file. Keep the directory and
file restricted to root (for example, modes 0700 and 0600); systemd reads the file
before dropping privileges to the service user. Do not put the secret in Nix
expressions, command-line arguments, Git, or the Nix store. Preserve it across
upgrades so existing browser identities and account sessions remain valid.

Accounts use usernames, passwords, and saved recovery codes. No mail server,
email provider, or additional runtime credentials are needed.

The module supplies `PHX_HOST`, `PHX_SERVER`, `PORT`, and PostgreSQL connection
settings. The default corpus database is bundled under the release's `priv/corpus`.
An optional `CORPUS_PATH` can point to a separately managed SQLite file, accompanied
by a sibling `release.txt`; ensure the service can read both files. A missing
database or release identifier prevents startup. Production uses a local
PostgreSQL Unix socket with peer authentication.

## Build and activate

Run `nix flake check` and `nix build` in the app repository first. Update the app
input in your host flake, review the change, and activate it with the host's
normal `nixos-rebuild switch` workflow. This repository does not assume an SSH
host or modify any machine configuration automatically.

The systemd service runs `Scripguessr.Release.migrate` before starting Phoenix.
Migration failure prevents startup. Application restarts retain game snapshots
and accepted guesses in PostgreSQL. Prepared games can finish without a
working corpus database.

To update scripture data, change the release tag and archive hash in
`nix/corpus.nix`, run the checks, and deploy the resulting package. Restarting the
service opens the new bundled database and clears runtime caches. Previous game
snapshots and saved verses keep their original text and release. NixOS rollback
restores the previous bundled corpus along with the application.

## Health and operations

- `GET /healthz`: process liveness, HTTP 200.
- `GET /readyz`: PostgreSQL connectivity, HTTP 200 or 503. Corpus availability is
  deliberately not part of readiness because saved games work without it.
- `journalctl -u scripguessr`: application and migration logs.
- `systemctl restart scripguessr`: restart using the current release.

From the host, probe the local endpoint with the public forwarding header:

```sh
curl --fail -H 'X-Forwarded-Proto: https' http://127.0.0.1:4000/readyz
```

Back up the `scripguessr` PostgreSQL database and the runtime secret before
upgrades. NixOS rollback restores the application package, not database changes;
review migrations for compatibility before rolling back. No automatic retention
or game deletion job is included in this milestone.

After initial deployment, run a real five-round game through the HTTPS hostname,
refresh mid-round, and restart the service to confirm resumption. This live-host
verification is separate from the local and sandboxed checks.
Also register an account, save its recovery code, attach an anonymous game, sign
out, and sign in from a second browser to verify saved history. Check that recovery
with the saved code replaces the password and issues a new code.
