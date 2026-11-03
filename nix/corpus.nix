{ pkgs }:
let
  release = "data-2026-10-29-154147";
  archive = pkgs.fetchurl {
    url = "https://github.com/mikaelweiss/standard-works-corpus/releases/download/${release}/scripture-db.zip";
    hash = "sha256-so5DAm9EvtY5ULnwO8x/KUxrN3PCNWCXRrLDLLKWf2E=";
  };
in
pkgs.runCommand "standard-works-corpus-${release}" { nativeBuildInputs = [ pkgs.unzip ]; } ''
  mkdir -p "$out"
  unzip -q ${archive} scripture.db LICENSE-DATA.txt CREDITS.md -d "$out"
  printf '%s\n' '${release}' > "$out/release.txt"
''
