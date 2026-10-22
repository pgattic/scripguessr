{
  description = "ScripGuessr";

  inputs = {
    nixpkgs.url = "https://channels.nixos.org/nixos-unstable/nixexprs.tar.zst";
    flake-parts.url = "github:hercules-ci/flake-parts";
  };

  outputs =
    inputs@{ flake-parts, ... }:
    flake-parts.lib.mkFlake { inherit inputs; } {
      flake.nixosModules.default = import ./nix/module.nix { self = inputs.self; };
      systems = [
        "x86_64-linux"
        "aarch64-linux"
        "aarch64-darwin"
      ];
      perSystem =
        { pkgs, lib, ... }:
        let
          beam = pkgs.beamPackages.overrideScope (_: prev: { elixir = prev.elixir_1_19; });
          src = lib.cleanSourceWith {
            src = ./.;
            filter =
              path: type:
              let
                name = baseNameOf path;
              in
              !(builtins.elem name [
                ".git"
                ".nix"
                "deps"
                "_build"
                "result"
                "result-checks"
              ]);
          };
          packages = import ./nix/package.nix { inherit pkgs beam src; };
          tools = with pkgs; [
            beam.elixir
            beam.erlang
            beam.hex
            beam.rebar3
            postgresql
            esbuild
            nixfmt
            git
            curl
            gnumake
            gcc
            cmake
            pkg-config
          ];
          environment = ''
            export MIX_HOME="$PWD/.nix/mix"
            export HEX_HOME="$PWD/.nix/hex"
            export MIX_ARCHIVES="$PWD/.nix/archives"
            export ERL_LIBS="${beam.hex}/lib/erlang/lib''${ERL_LIBS:+:$ERL_LIBS}"
            export MIX_REBAR3="${beam.rebar3}/bin/rebar3"
            export HEX_CACERTS_PATH="${pkgs.cacert}/etc/ssl/certs/ca-bundle.crt"
          '';
          command = name: text: {
            type = "app";
            meta.description = name;
            program = lib.getExe (
              pkgs.writeShellApplication {
                inherit name;
                runtimeInputs = tools;
                text = environment + text;
              }
            );
          };
        in
        {
          packages.default = packages.release;
          packages.deps = packages.deps;
          checks.quiz = packages.tests;
          checks.release = packages.release;
          checks.nix-format = pkgs.runCommand "nix-format" { nativeBuildInputs = [ pkgs.nixfmt ]; } ''
            cd ${src}
            nixfmt --check flake.nix nix/*.nix
            touch "$out"
          '';
          formatter = pkgs.writeShellApplication {
            name = "scripguessr-format";
            runtimeInputs = tools;
            text = environment + ''
              nixfmt flake.nix nix/*.nix
              mix format
            '';
          };
          apps.dev = command "scripguessr-dev" ''
            # shellcheck disable=SC1091
            source ${./scripts/database.sh}
            mix deps.get
            mix ecto.create
            mix ecto.migrate
            mix assets.build
            mix phx.server
          '';
          apps.test = command "scripguessr-test" ''
            # shellcheck disable=SC1091
            source ${./scripts/database.sh}
            export MIX_ENV=test
            mix deps.get
            mix ecto.create
            mix ecto.migrate
            mix test "$@"
          '';
          apps.smoke = command "scripguessr-smoke" ''
            mix deps.get
            mix run --no-start scripts/smoke.exs
          '';
          devShells.default = pkgs.mkShell {
            packages = tools;
            shellHook = environment;
          };
        };
    };
}
