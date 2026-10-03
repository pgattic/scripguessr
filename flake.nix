{
  description = "ScripGuessr, a Dioxus scripture location guessing game";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-parts.url = "github:hercules-ci/flake-parts";
    crane.url = "github:ipetkov/crane";
  };

  outputs =
    inputs@{ flake-parts, ... }:
    flake-parts.lib.mkFlake { inherit inputs; } {
      flake.nixosModules.default =
        { pkgs, ... }:
        {
          imports = [
            (import ./nix/module.nix {
              defaultPackage = inputs.self.packages.${pkgs.stdenv.hostPlatform.system}.default;
            })
          ];
        };

      systems = [
        "x86_64-linux"
        "aarch64-linux"
        "aarch64-darwin"
      ];

      perSystem =
        { config, pkgs, ... }:
        let
          craneLib = inputs.crane.mkLib pkgs;
          sources = import ./nix/sources.nix { inherit (pkgs) lib; };
          checkArgs = {
            pname = "scripguessr-checks";
            version = "0.1.0";
            src = sources.check;
            strictDeps = true;
            nativeBuildInputs = [ pkgs.lld ];
          };
          checkCargoArtifacts = craneLib.buildDepsOnly checkArgs;
        in
        {
          devShells.default = pkgs.mkShell {
            packages = with pkgs; [
              cargo
              clippy
              dioxus-cli
              lld
              poppler-utils
              python3
              postgresql
              rustc
              rustfmt
              wasm-bindgen-cli
            ];
          };

          formatter = pkgs.writeShellApplication {
            name = "scripguessr-format";
            runtimeInputs = with pkgs; [
              cargo
              nixfmt
              rustfmt
            ];
            text = ''
              nixfmt flake.nix nix/*.nix
              cargo fmt --all
            '';
          };

          packages.default = pkgs.callPackage ./nix/package.nix { inherit craneLib; };
          packages.scripguessr = config.packages.default;

          checks = {
            rust-tests = craneLib.cargoTest (
              checkArgs
              // {
                cargoArtifacts = checkCargoArtifacts;
                cargoTestExtraArgs = "--all-targets";
              }
            );

            rust-clippy = craneLib.cargoClippy (
              checkArgs
              // {
                cargoArtifacts = checkCargoArtifacts;
                cargoClippyExtraArgs = "--all-targets -- --deny warnings";
              }
            );

            web = config.packages.default.passthru.web;

            pmg-data =
              pkgs.runCommand "scripguessr-pmg-data-check"
                {
                  nativeBuildInputs = [
                    pkgs.python3
                    pkgs.poppler-utils
                  ];
                  src = sources.pmg;
                }
                ''
                  cp -R "$src" source
                  chmod -R u+w source
                  python3 source/scripts/extract_pmg_study_sets.py \
                    source/preach_my_gospel_2_0.pdf \
                    source/assets/data/preach-my-gospel-study-sets.json \
                    --check
                  touch "$out"
                '';
          };

          packages.dev = pkgs.writeShellApplication {
            name = "scripguessr-dev";
            runtimeInputs = with pkgs; [
              gcc
              cargo
              curl
              dioxus-cli
              lld
              postgresql
              rustc
              wasm-bindgen-cli
            ];
            text = ''
              export SCRIPGUESSR_API_BASE="''${SCRIPGUESSR_API_BASE:-http://localhost:8099}"
              export PORT="''${SCRIPGUESSR_BACKEND_PORT:-8099}"
              export SCRIPGUESSR_STATIC_DIR="target/dx/scripguessr/debug/web/public"
              export SCRIPGUESSR_ALLOWED_ORIGIN="''${SCRIPGUESSR_ALLOWED_ORIGIN:-http://localhost:8080,http://127.0.0.1:8080}"
              export SCRIPGUESSR_SECURE_COOKIES=false

              pg_data="''${SCRIPGUESSR_DEV_PGDATA:-$PWD/target/dev-postgres-data}"
              pg_socket="$PWD/target/dev-postgres-socket"
              if [ ! -s "$pg_data/PG_VERSION" ]; then
                initdb --username="$USER" --auth=trust --no-locale "$pg_data"
              fi
              mkdir -p "$pg_socket"
              started_postgres=false
              if ! pg_ctl -D "$pg_data" status >/dev/null 2>&1; then
                pg_ctl -D "$pg_data" -o "-k $pg_socket -p 55432" -w start
                started_postgres=true
              fi
              createdb -h "$pg_socket" -p 55432 scripguessr 2>/dev/null || true
              export DATABASE_URL="postgresql://$USER@localhost/scripguessr?host=$pg_socket&port=55432"

              backend_pid=""
              if ! curl --fail --silent --max-time 2 "http://127.0.0.1:$PORT/readyz" >/dev/null; then
                cargo run &
                backend_pid="$!"
              fi
              cleanup() {
                if [ -n "$backend_pid" ]; then
                  kill "$backend_pid" 2>/dev/null || true
                  wait "$backend_pid" 2>/dev/null || true
                fi
                if [ "$started_postgres" = true ]; then
                  pg_ctl -D "$pg_data" -m fast -w stop 2>/dev/null || true
                fi
              }
              trap cleanup EXIT INT TERM

              dx serve --platform web "$@"
            '';
          };

          packages.serve = pkgs.writeShellApplication {
            name = "scripguessr-serve";
            runtimeInputs = [ pkgs.python3 ];
            text = ''
              cd target/dx/scripguessr/debug/web/public
              exec python3 -m http.server "''${1:-8080}" --bind 127.0.0.1
            '';
          };

          apps.dev = {
            type = "app";
            program = pkgs.lib.getExe config.packages.dev;
            meta.description = "Run the ScripGuessr Dioxus web dev server";
          };

          apps.serve = {
            type = "app";
            program = pkgs.lib.getExe config.packages.serve;
            meta.description = "Serve the last built ScripGuessr web bundle";
          };
        };
    };
}
