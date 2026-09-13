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

  src = lib.cleanSource ../.;

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
    runHook postBuild
  '';

  installPhase = ''
    runHook preInstall
    mkdir -p "$out"
    cp -R target/dx/scripguessr/release/web/public/. "$out/"
    runHook postInstall
  '';

  meta = {
    description = "Static web build of ScripGuessr";
    platforms = lib.platforms.all;
  };
}
