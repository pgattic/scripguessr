{ lib }:

let
  sourceWith =
    {
      files,
      dirs ? [
        ""
        "assets"
        "assets/data"
        "src"
      ],
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

  scriptureData = [
    "assets/data/book-of-mormon-flat.json"
    "assets/data/doctrine-and-covenants-flat.json"
    "assets/data/new-testament-flat.json"
    "assets/data/old-testament-flat.json"
    "assets/data/pearl-of-great-price-flat.json"
  ];

  sharedRust = [
    "src/api.rs"
    "src/main.rs"
    "src/scoring.rs"
    "src/scriptures.rs"
    "src/study_sets.rs"
  ];

  generatedStudySets = "assets/data/preach-my-gospel-study-sets.json";

  serverFiles =
    cargoFiles
    ++ scriptureData
    ++ sharedRust
    ++ [
      generatedStudySets
      "src/server.rs"
    ];

  webFiles =
    cargoFiles
    ++ sharedRust
    ++ [
      "Dioxus.toml"
      "assets/main.css"
      generatedStudySets
      "src/components.rs"
      "src/game.rs"
      "src/loader.rs"
      "src/stats.rs"
    ];
in
{
  server = sourceWith { files = serverFiles; };
  web = sourceWith { files = webFiles; };
  check = sourceWith { files = lib.unique (serverFiles ++ webFiles); };
  pmg = sourceWith {
    dirs = [
      ""
      "assets"
      "assets/data"
      "scripts"
    ];
    files = scriptureData ++ [
      generatedStudySets
      "preach_my_gospel_2_0.pdf"
      "scripts/extract_pmg_study_sets.py"
    ];
  };
}
