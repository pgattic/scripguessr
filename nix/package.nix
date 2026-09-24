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

  sources = import ./sources.nix { inherit lib; };

  commonArgs = {
    inherit pname version;
    strictDeps = true;
    nativeBuildInputs = [ lld ];
  };

  serverArgs = commonArgs // {
    src = sources.server;
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
    src = sources.web;
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
  passthru = { inherit server web; };

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
