{
  lib,
  stdenv,
  rustPlatform,
  binaryen,
  cargo,
  dioxus-cli,
  lld,
  rustc,
  wasm-bindgen-cli,
}:

stdenv.mkDerivation {
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

  cargoDeps = rustPlatform.importCargoLock {
    lockFile = ../Cargo.lock;
  };

  nativeBuildInputs = [
    rustPlatform.cargoSetupHook
    binaryen
    cargo
    dioxus-cli
    lld
    rustc
    wasm-bindgen-cli
  ];

  dontConfigure = true;

  buildPhase = ''
    runHook preBuild
    export HOME="$TMPDIR"
    export CARGO_NET_OFFLINE=true
    dx build --release --platform web --locked
    cargo build --release --locked
    runHook postBuild
  '';

  installPhase = ''
    runHook preInstall
    mkdir -p "$out/bin" "$out/share/scripguessr/public"
    cp target/release/scripguessr "$out/bin/"
    cp -R target/dx/scripguessr/release/web/public/. "$out/share/scripguessr/public/"
    runHook postInstall
  '';

  meta = {
    description = "ScripGuessr web server";
    mainProgram = "scripguessr";
    platforms = lib.platforms.all;
  };
}
