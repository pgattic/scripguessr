import Config

config :scripguessr, :match_clock, false
config :scripguessr, corpus_client: Scripguessr.Corpus.Fake

config :scripguessr, Scripguessr.Repo,
  database: "scripguessr_test",
  username: System.get_env("PGUSER", System.get_env("USER", "postgres")),
  socket_dir: System.get_env("PGHOST", Path.expand(".nix/postgres/socket")),
  port: String.to_integer(System.get_env("PGPORT", "55437")),
  pool: Ecto.Adapters.SQL.Sandbox,
  pool_size: 10

config :scripguessr, ScripguessrWeb.Endpoint,
  http: [ip: {127, 0, 0, 1}, port: 4002],
  secret_key_base: String.duplicate("test-only-", 8),
  server: false

config :logger, level: :warning
config :phoenix, :plug_init_mode, :runtime
