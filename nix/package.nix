{
  pkgs,
  beam,
  src,
}:
let
  deps = beam.fetchMixDeps {
    pname = "scripguessr-deps";
    version = "0.1.0";
    inherit src;
    mixEnv = "test";
    hash = "sha256-b4b8fS6FoyZctnVG4kzHfWAPRt+KBLaamsh5k2/nk6s=";
  };
  lexbor = pkgs.fetchFromGitHub {
    owner = "lexbor";
    repo = "lexbor";
    rev = "244b84956a6dc7eec293781d051354f351274c46";
    hash = "sha256-Oup/lGU8a9Dqfho4Llg39t9Y9n4xfUmGk0772OkpnLQ=";
  };
  common = {
    pname = "scripguessr";
    version = "0.1.0";
    inherit src;
    mixFodDeps = deps;
    nativeBuildInputs = [ pkgs.esbuild ];
    compileFlags = [ "--warnings-as-errors" ];
    postBuild = "mix assets.deploy";
    meta.mainProgram = "scripguessr";
  };
  release = beam.mixRelease common;
  tests = beam.mixRelease (
    common
    // {
      pname = "scripguessr-checks";
      mixEnv = "test";
      nativeBuildInputs = [
        pkgs.esbuild
        pkgs.postgresql
        pkgs.cmake
        pkgs.pkg-config
      ];
      env.ELIXIR_MAKE_FORCE_BUILD = "true";
      dontUseCmakeConfigure = true;
      preConfigure = ''
        mkdir -p "$MIX_DEPS_PATH/lazy_html/_build/c/third_party/lexbor"
        cp -R ${lexbor} "$MIX_DEPS_PATH/lazy_html/_build/c/third_party/lexbor/244b84956a6dc7eec293781d051354f351274c46"
        chmod -R u+w "$MIX_DEPS_PATH/lazy_html/_build"
      '';
      postBuild = ''
        mix format --check-formatted
        mix assets.build
        export PGHOST="$TMPDIR/pgsocket"
        export PGPORT=55437
        export PGUSER=postgres
        mkdir -p "$PGHOST"
        initdb -D "$TMPDIR/pgdata" --username="$PGUSER" --auth=trust --no-locale --encoding=UTF8 >/dev/null
        pg_ctl -D "$TMPDIR/pgdata" -o "-k $PGHOST -p $PGPORT -c listen_addresses=" -w start
        trap 'pg_ctl -D "$TMPDIR/pgdata" -m immediate -w stop' EXIT
        mix ecto.create
        mix ecto.migrate
        mix test
      '';
      installPhase = ''
        mkdir -p "$out"
        touch "$out/passed"
      '';
      dontFixup = true;
    }
  );
in
{
  inherit release tests deps;
}
