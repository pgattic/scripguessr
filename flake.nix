{
  description = "ScripGuessr, a Dioxus scripture location guessing game";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-parts.url = "github:hercules-ci/flake-parts";
  };

  outputs =
    inputs@{ flake-parts, ... }:
    flake-parts.lib.mkFlake { inherit inputs; } {
      flake.nixosModules.default = import ./nix/module.nix;

      systems = [
        "x86_64-linux"
        "aarch64-linux"
        "x86_64-darwin"
        "aarch64-darwin"
      ];

      perSystem =
        { config, pkgs, ... }:
        {
          devShells.default = pkgs.mkShell {
            packages = with pkgs; [
              cargo
              clippy
              dioxus-cli
              lld
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
              nixfmt flake.nix
              cargo fmt --all
            '';
          };

          packages.default = pkgs.callPackage ./nix/package.nix { };

          packages.dev = pkgs.writeShellApplication {
            name = "scripguessr-dev";
            runtimeInputs = with pkgs; [
              cargo
              dioxus-cli
              lld
              rustc
              wasm-bindgen-cli
            ];
            text = ''
              exec dx serve --platform web "$@"
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
