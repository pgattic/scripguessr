[
  import_deps: [:ecto, :ecto_sql, :phoenix, :phoenix_live_view],
  plugins: [Phoenix.LiveView.HTMLFormatter],
  inputs: [
    "mix.exs",
    ".formatter.exs",
    "{config,lib,test,scripts}/**/*.{ex,exs,heex}",
    "priv/repo/**/*.exs"
  ]
]
