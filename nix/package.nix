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

  src = lib.cleanSourceWith {
    src = ../.;
    filter =
      path: type:
      let
        root = toString ../.;
        relative = lib.removePrefix "${root}/" (toString path);
      in
      !(lib.hasPrefix ".git" relative)
      && !(lib.hasPrefix "target" relative)
      && relative != "result";
  };

  commonArgs = {
    inherit pname version src;
    strictDeps = true;
    nativeBuildInputs = [ lld ];
  };

  serverCargoArtifacts = craneLib.buildDepsOnly commonArgs;

  server = craneLib.buildPackage (
    commonArgs
    // {
      cargoArtifacts = serverCargoArtifacts;
      doCheck = false;
    }
  );

  webArgs = commonArgs // {
    pname = "${pname}-web";
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
