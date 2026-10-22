import Config

config :scripguessr, Scripguessr.Repo,
  database: "scripguessr_dev",
  username: System.get_env("PGUSER", System.get_env("USER", "postgres")),
  socket_dir: System.get_env("PGHOST", Path.expand(".nix/postgres/socket")),
  port: String.to_integer(System.get_env("PGPORT", "55437")),
  show_sensitive_data_on_connection_error: true

config :scripguessr, ScripguessrWeb.Endpoint,
  http: [ip: {127, 0, 0, 1}, port: String.to_integer(System.get_env("PORT", "4000"))],
  secret_key_base: String.duplicate("development-only-", 5),
  check_origin: false,
  code_reloader: true,
  debug_errors: true

config :phoenix, :stacktrace_depth, 20
