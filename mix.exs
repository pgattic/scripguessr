defmodule Scripguessr.MixProject do
  use Mix.Project

  def project do
    [
      app: :scripguessr,
      version: "0.1.0",
      elixir: "~> 1.19",
      elixirc_paths: elixirc_paths(Mix.env()),
      start_permanent: Mix.env() == :prod,
      compilers: [:phoenix_live_view] ++ Mix.compilers(),
      listeners: [Phoenix.CodeReloader],
      deps: deps(),
      aliases: [
        "assets.build": [
          "cmd esbuild assets/js/app.js --bundle --minify --target=es2022 --outfile=priv/static/assets/app.js"
        ],
        "assets.deploy": ["assets.build", "phx.digest"]
      ]
    ]
  end

  def application,
    do: [mod: {Scripguessr.Application, []}, extra_applications: [:logger, :runtime_tools]]

  defp elixirc_paths(:test), do: ["lib", "test/support"]
  defp elixirc_paths(_), do: ["lib"]

  defp deps do
    [
      {:phoenix, "~> 1.8.0"},
      {:phoenix_live_view, "~> 1.2.0"},
      {:phoenix_html, "~> 4.1"},
      {:phoenix_ecto, "~> 4.6"},
      {:ecto_sql, "~> 3.13"},
      {:postgrex, "~> 0.21"},
      {:bandit, "~> 1.5"},
      {:jason, "~> 1.4"},
      {:eqrcode, "~> 0.2.1"},
      {:req, "~> 0.7"},
      {:pbkdf2_elixir, "~> 2.3"},
      {:lazy_html, ">= 0.1.0", only: :test}
    ]
  end
end
