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
              dioxus-cli
              lld
              rustc
              wasm-bindgen-cli
            ];
            text = ''
              export SCRIPGUESSR_API_BASE="''${SCRIPGUESSR_API_BASE:-http://127.0.0.1:8099}"
              export PORT="''${SCRIPGUESSR_BACKEND_PORT:-8099}"
              export SCRIPGUESSR_STATIC_DIR="target/dx/scripguessr/debug/web/public"

              cargo run &
              backend_pid="$!"
              cleanup() {
                kill "$backend_pid" 2>/dev/null || true
                wait "$backend_pid" 2>/dev/null || true
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
