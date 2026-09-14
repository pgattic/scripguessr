{
  lib,
  stdenv,
  craneLib,
  binaryen,
  dioxus-cli,
  lld,
  wasm-bindgen-cli,
}:

let
  pname = "scripguessr";
  version = "0.1.0";

  sourceWith =
    {
      files,
      dirs,
    }:
    lib.cleanSourceWith {
      src = ../.;
      filter =
        path: type:
        let
          root = toString ../.;
          relative = lib.removePrefix "${root}/" (toString path);
        in
        (type == "directory" && builtins.elem relative dirs) || builtins.elem relative files;
    };

  cargoFiles = [
    "Cargo.lock"
    "Cargo.toml"
  ];

  serverSrc = sourceWith {
    dirs = [
      ""
      "assets"
      "assets/data"
      "src"
    ];
    files = cargoFiles ++ [
      "assets/data/book-of-mormon-flat.json"
      "assets/data/doctrine-and-covenants-flat.json"
      "assets/data/new-testament-flat.json"
      "assets/data/old-testament-flat.json"
      "assets/data/pearl-of-great-price-flat.json"
      "src/api.rs"
      "src/main.rs"
      "src/scoring.rs"
      "src/scriptures.rs"
      "src/server.rs"
    ];
  };

  webSrc = sourceWith {
    dirs = [
      ""
      "assets"
      "src"
    ];
    files = cargoFiles ++ [
      "Dioxus.toml"
      "assets/main.css"
      "src/api.rs"
      "src/components.rs"
      "src/game.rs"
      "src/loader.rs"
      "src/main.rs"
      "src/scoring.rs"
      "src/scriptures.rs"
      "src/stats.rs"
    ];
  };

  commonArgs = {
    inherit pname version;
    strictDeps = true;
    nativeBuildInputs = [ lld ];
  };

  serverArgs = commonArgs // {
    src = serverSrc;
  };

  serverCargoArtifacts = craneLib.buildDepsOnly serverArgs;

  server = craneLib.buildPackage (
    serverArgs
    // {
      cargoArtifacts = serverCargoArtifacts;
      doCheck = false;
    }
  );

  webArgs = commonArgs // {
    pname = "${pname}-web";
    src = webSrc;
    nativeBuildInputs = [
      binaryen
      dioxus-cli
      lld
      wasm-bindgen-cli
    ];
    CARGO_BUILD_TARGET = "wasm32-unknown-unknown";
    CARGO_BUILD_RUSTFLAGS = ''--cfg getrandom_backend="wasm_js"'';
    cargoExtraArgs = "--features web";
  };

  webCargoArtifacts = craneLib.buildDepsOnly webArgs;

  web = craneLib.mkCargoDerivation (
    webArgs
    // {
      cargoArtifacts = webCargoArtifacts;
      buildPhaseCargoCommand = "dx build --release --platform web --locked";
      checkPhaseCargoCommand = "";

      installPhaseCommand = ''
        mkdir -p "$out/share/scripguessr/public"
        cp -R target/dx/scripguessr/release/web/public/. "$out/share/scripguessr/public/"
      '';
    }
  );
in
stdenv.mkDerivation {
  inherit pname version;

  dontUnpack = true;

  installPhase = ''
    runHook preInstall
    mkdir -p "$out/bin" "$out/share/scripguessr/public"
    cp ${server}/bin/scripguessr "$out/bin/"
    cp -R ${web}/share/scripguessr/public/. "$out/share/scripguessr/public/"
    runHook postInstall
  '';

  meta = {
    description = "ScripGuessr web server";
    mainProgram = "scripguessr";
    platforms = lib.platforms.all;
  };
}
